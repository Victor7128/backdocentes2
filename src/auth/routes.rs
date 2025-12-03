use crate::auth::models::*;
use crate::AppState;
use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder};
use sqlx::PgPool;
use sqlx::Row;
use tracing;

#[post("/api/auth/register/alumno")]
pub async fn register_alumno(
    data: web::Data<AppState>,
    body: web::Json<RegisterAlumnoRequest>,
) -> impl Responder {
    if body.dni.len() != 8 || !body.dni.chars().all(|c| c.is_numeric()) {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "DNI inválido".to_string(),
            details: Some("El DNI debe tener 8 dígitos numéricos".to_string()),
        });
    }
    let exists = sqlx::query_scalar::<_, Option<i32>>(
        "SELECT 1 FROM users WHERE firebase_uid = $1 OR email = $2",
    )
    .bind(&body.firebase_uid)
    .bind(&body.email)
    .fetch_optional(&data.pool)
    .await;

    if let Ok(Some(_)) = exists {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "Usuario ya existe".to_string(),
            details: Some("El firebase_uid o email ya están registrados".to_string()),
        });
    }
    let dni_exists =
        sqlx::query_scalar::<_, Option<i32>>("SELECT 1 FROM student_profiles WHERE dni = $1")
            .bind(&body.dni)
            .fetch_optional(&data.pool)
            .await;

    if let Ok(Some(_)) = dni_exists {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "DNI ya registrado".to_string(),
            details: Some("Ya existe un estudiante con este DNI".to_string()),
        });
    }

    let mut tx = match data.pool.begin().await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("❌ Error iniciando transacción: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Error de base de datos".to_string(),
                details: Some(e.to_string()),
            });
        }
    };

    // Crear usuario
    let user = match sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (firebase_uid, email, role, status)
        VALUES ($1, $2, $3, $4)
        RETURNING id, firebase_uid, email, role, status, 
                  created_at, updated_at, last_login, 
                  profile_photo_url, phone
        "#,
    )
    .bind(&body.firebase_uid)
    .bind(&body.email)
    .bind(UserRole::Alumno)
    .bind(UserStatus::Active)
    .fetch_one(&mut *tx)
    .await
    {
        Ok(u) => {
            tracing::info!("✅ Usuario creado: id={}, email={}", u.id, u.email);
            u
        }
        Err(e) => {
            let _ = tx.rollback().await;
            tracing::error!("❌ Error creando usuario: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Error creando usuario".to_string(),
                details: Some(e.to_string()),
            });
        }
    };

    // Crear perfil de alumno (esto disparará el trigger)
    let student_profile = match sqlx::query_as::<_, StudentProfile>(
        r#"
        INSERT INTO student_profiles (user_id, dni, full_name, enrollment_date)
        VALUES ($1, $2, $3, CURRENT_DATE)
        RETURNING user_id, dni, full_name, date_of_birth, gender, 
                  address, enrollment_code, enrollment_date
        "#,
    )
    .bind(user.id)
    .bind(&body.dni)
    .bind(&body.full_name)
    .fetch_one(&mut *tx)
    .await
    {
        Ok(p) => {
            tracing::info!(
                "✅ Perfil de alumno creado: user_id={}, dni={}, nombre='{}'",
                p.user_id,
                p.dni,
                p.full_name
            );
            p
        }
        Err(e) => {
            let _ = tx.rollback().await;
            tracing::error!("❌ Error creando perfil de alumno: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Error creando perfil de alumno".to_string(),
                details: Some(e.to_string()),
            });
        }
    };

    // Commit de la transacción
    if let Err(e) = tx.commit().await {
        tracing::error!("❌ Error confirmando transacción: {:?}", e);
        return HttpResponse::InternalServerError().json(ErrorResponse {
            error: "Error confirmando registro".to_string(),
            details: Some(e.to_string()),
        });
    }

    tracing::info!("✅ Transacción confirmada exitosamente");

    // ============================================
    // 3. VERIFICAR VINCULACIÓN (POST-COMMIT)
    // ============================================

    // Esperar un momento para que el trigger se ejecute
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Verificar si el trigger vinculó automáticamente
    let linked_result = sqlx::query(
        r#"
        SELECT 
            s.id AS student_id,
            s.full_name AS student_name,
            spl.linked_by_method,
            spl.linked_at
        FROM students s
        LEFT JOIN student_profile_links spl ON spl.student_id = s.id AND spl.user_id = s.user_id
        WHERE s.user_id = $1
        LIMIT 1
        "#,
    )
    .bind(user.id)
    .fetch_optional(&data.pool)
    .await;

    let linking_info = match linked_result {
        Ok(Some(row)) => {
            let student_id: i32 = row.try_get("student_id").unwrap();
            let student_name: String = row.try_get("student_name").unwrap();
            let method: Option<String> = row.try_get("linked_by_method").ok();

            tracing::info!(
                "✅ Trigger vinculó automáticamente: user_id={}, student_id={}, nombre='{}', método={:?}",
                user.id, student_id, student_name, method
            );

            Some(LinkingInfo {
                student_id,
                student_name,
                linked_by: method.unwrap_or_else(|| "auto".to_string()),
                success: true,
            })
        }
        Ok(None) => {
            tracing::warn!(
                "⚠️ Trigger NO vinculó automáticamente, intentando vinculación manual..."
            );

            // Intentar vinculación manual como fallback
            match try_link_student_by_name(&data.pool, user.id, &body.full_name, &body.dni).await {
                Ok(Some(student_id)) => {
                    tracing::info!(
                        "✅ Vinculación manual exitosa: user_id={}, student_id={}",
                        user.id,
                        student_id
                    );

                    // Obtener nombre del estudiante vinculado
                    let student_name = sqlx::query_scalar::<_, String>(
                        "SELECT full_name FROM students WHERE id = $1",
                    )
                    .bind(student_id)
                    .fetch_one(&data.pool)
                    .await
                    .unwrap_or_else(|_| body.full_name.clone());

                    Some(LinkingInfo {
                        student_id,
                        student_name,
                        linked_by: "manual".to_string(),
                        success: true,
                    })
                }
                Ok(None) => {
                    tracing::warn!(
                        "⚠️ No se encontró estudiante para vincular: '{}'",
                        body.full_name
                    );
                    None
                }
                Err(e) => {
                    tracing::error!("❌ Error en vinculación manual: {:?}", e);
                    None
                }
            }
        }
        Err(e) => {
            tracing::error!("❌ Error verificando vinculación: {:?}", e);
            None
        }
    };

    // ============================================
    // 4. PREPARAR RESPUESTA
    // ============================================

    let mut profile_data = serde_json::json!({
        "dni": student_profile.dni,
        "full_name": student_profile.full_name,
        "enrollment_date": student_profile.enrollment_date,
    });

    let (message, is_linked) = if let Some(info) = linking_info {
        profile_data["linked_student_id"] = serde_json::json!(info.student_id);
        profile_data["linked_student_name"] = serde_json::json!(info.student_name);
        profile_data["linked_by_method"] = serde_json::json!(info.linked_by);
        profile_data["auto_linked"] =
            serde_json::json!(info.linked_by == "dni_auto" || info.linked_by == "full_name_auto");

        let method_desc = match info.linked_by.as_str() {
            "dni_auto" => "por DNI (automático)",
            "full_name_auto" => "por nombre (automático)",
            "full_name_manual" => "por nombre (manual)",
            _ => "exitosamente",
        };

        (
            format!("Alumno registrado y vinculado {}", method_desc),
            true,
        )
    } else {
        profile_data["auto_linked"] = serde_json::json!(false);
        profile_data["linking_note"] = serde_json::json!(
            "No se encontró un registro previo para vincular. El alumno podrá ser vinculado manualmente más tarde."
        );

        (
            "Alumno registrado exitosamente (sin vinculación automática)".to_string(),
            false,
        )
    };

    tracing::info!(
        "🎉 Registro completado: user_id={}, dni={}, vinculado={}",
        user.id,
        body.dni,
        is_linked
    );

    HttpResponse::Created().json(ApiResponse {
        success: true,
        message,
        data: Some(UserResponse {
            id: user.id,
            email: user.email.clone(),
            role: user.role.to_string(),
            status: user.status.to_string(),
            profile_data,
        }),
    })
}

#[post("/api/auth/register/apoderado")]
pub async fn register_apoderado(
    data: web::Data<AppState>,
    body: web::Json<RegisterApoderadoRequest>,
) -> impl Responder {
    // Validar DNI
    if body.dni.len() != 8 || !body.dni.chars().all(|c| c.is_numeric()) {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "DNI inválido".to_string(),
            details: Some("El DNI debe tener 8 dígitos".to_string()),
        });
    }

    // Verificar si ya existe
    let exists = sqlx::query_scalar::<_, Option<i32>>(
        "SELECT 1 FROM users WHERE firebase_uid = $1 OR email = $2",
    )
    .bind(&body.firebase_uid)
    .bind(&body.email)
    .fetch_optional(&data.pool)
    .await;

    if let Ok(Some(_)) = exists {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "Usuario ya existe".to_string(),
            details: Some("El firebase_uid o email ya están registrados".to_string()),
        });
    }

    let mut tx = match data.pool.begin().await {
        Ok(t) => t,
        Err(e) => {
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Error de base de datos".to_string(),
                details: Some(e.to_string()),
            });
        }
    };

    // 1. Crear usuario
    let user = match sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (firebase_uid, email, role, status, phone)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, firebase_uid, email, role, status, 
                  created_at, updated_at, last_login, 
                  profile_photo_url, phone
        "#,
    )
    .bind(&body.firebase_uid)
    .bind(&body.email)
    .bind(UserRole::Apoderado)
    .bind(UserStatus::Active)
    .bind(Some(&body.phone))
    .fetch_one(&mut *tx)
    .await
    {
        Ok(u) => u,
        Err(e) => {
            let _ = tx.rollback().await;
            eprintln!("Error creando usuario: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Error creando usuario".to_string(),
                details: Some(e.to_string()),
            });
        }
    };

    // 2. Crear perfil de apoderado
    let guardian_profile = match sqlx::query_as::<_, GuardianProfile>(
        r#"
        INSERT INTO guardian_profiles 
        (user_id, full_name, dni, relationship_type, occupation, workplace, emergency_phone)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING user_id, full_name, dni, relationship_type, 
                  occupation, workplace, emergency_phone
        "#,
    )
    .bind(user.id)
    .bind(&body.full_name)
    .bind(Some(&body.dni))
    .bind(Some(&body.relationship_type))
    .bind(body.occupation.as_ref())
    .bind(body.workplace.as_ref())
    .bind(Some(&body.phone))
    .fetch_one(&mut *tx)
    .await
    {
        Ok(p) => p,
        Err(e) => {
            let _ = tx.rollback().await;
            eprintln!("Error creando perfil de apoderado: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Error creando perfil de apoderado".to_string(),
                details: Some(e.to_string()),
            });
        }
    };

    if let Err(e) = tx.commit().await {
        return HttpResponse::InternalServerError().json(ErrorResponse {
            error: "Error confirmando registro".to_string(),
            details: Some(e.to_string()),
        });
    }

    HttpResponse::Created().json(ApiResponse {
        success: true,
        message: "Apoderado registrado exitosamente".to_string(),
        data: Some(UserResponse {
            id: user.id,
            email: user.email.clone(),
            role: user.role.to_string(),
            status: user.status.to_string(),
            profile_data: serde_json::json!({
                "dni": guardian_profile.dni,
                "full_name": guardian_profile.full_name,
                "relationship_type": guardian_profile.relationship_type,
                "phone": guardian_profile.emergency_phone,
            }),
        }),
    })
}

#[post("/api/auth/register/docente")]
pub async fn register_docente(
    data: web::Data<AppState>,
    body: web::Json<RegisterDocenteRequest>,
) -> impl Responder {
    // Validar DNI
    if body.dni.len() != 8 || !body.dni.chars().all(|c| c.is_numeric()) {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "DNI inválido".to_string(),
            details: Some("El DNI debe tener 8 dígitos".to_string()),
        });
    }

    // Verificar si ya existe
    let exists = sqlx::query_scalar::<_, Option<i32>>(
        "SELECT 1 FROM users WHERE firebase_uid = $1 OR email = $2",
    )
    .bind(&body.firebase_uid)
    .bind(&body.email)
    .fetch_optional(&data.pool)
    .await;

    if let Ok(Some(_)) = exists {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "Usuario ya existe".to_string(),
            details: Some("El firebase_uid o email ya están registrados".to_string()),
        });
    }

    // Verificar área
    let area_id = if let Some(id) = body.area_id {
        println!("✅ Usando area_id proporcionado: {}", id);

        match sqlx::query_scalar::<_, i32>("SELECT id FROM areas WHERE id = $1")
            .bind(id)
            .fetch_optional(&data.pool)
            .await
        {
            Ok(Some(area_id)) => {
                println!("✅ Área verificada con ID: {}", area_id);
                area_id
            }
            Ok(None) => {
                println!("❌ Área no encontrada con ID: {}", id);
                return HttpResponse::BadRequest().json(ErrorResponse {
                    error: "Área no encontrada".to_string(),
                    details: Some(format!("No existe área con ID: {}", id)),
                });
            }
            Err(e) => {
                eprintln!("❌ Error verificando área por ID: {:?}", e);
                return HttpResponse::InternalServerError().json(ErrorResponse {
                    error: "Error verificando área".to_string(),
                    details: Some(e.to_string()),
                });
            }
        }
    } else {
        println!(
            "⚠️ No se proporcionó area_id, buscando por nombre: '{}'",
            body.area_name
        );

        match sqlx::query_scalar::<_, i32>(
            "SELECT id FROM areas WHERE LOWER(TRIM(nombre)) = LOWER(TRIM($1))",
        )
        .bind(&body.area_name)
        .fetch_optional(&data.pool)
        .await
        {
            Ok(Some(id)) => {
                println!(
                    "✅ Área encontrada por nombre: '{}' → ID: {}",
                    body.area_name, id
                );
                id
            }
            Ok(None) => {
                eprintln!("❌ Área no encontrada con nombre: '{}'", body.area_name);
                return HttpResponse::BadRequest().json(ErrorResponse {
                    error: "Área no encontrada".to_string(),
                    details: Some(format!("No existe el área: '{}'", body.area_name)),
                });
            }
            Err(e) => {
                eprintln!("❌ Error buscando área por nombre: {:?}", e);
                return HttpResponse::InternalServerError().json(ErrorResponse {
                    error: "Error buscando área".to_string(),
                    details: Some(e.to_string()),
                });
            }
        }
    };

    println!("✅ Área final seleccionada - ID: {}", area_id);

    let mut tx = match data.pool.begin().await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("❌ Error iniciando transacción: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Error de base de datos".to_string(),
                details: Some(e.to_string()),
            });
        }
    };

    // ✅ Insertar con ENUM directamente
    let user = match sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (firebase_uid, email, role, status)
        VALUES ($1, $2, $3, $4)
        RETURNING id, firebase_uid, email, role, status, 
                  created_at, updated_at, last_login, 
                  profile_photo_url, phone
        "#,
    )
    .bind(&body.firebase_uid)
    .bind(&body.email)
    .bind(UserRole::Docente) // ✅ Usar el ENUM
    .bind(UserStatus::Active) // ✅ Usar el ENUM
    .fetch_one(&mut *tx)
    .await
    {
        Ok(u) => {
            println!("✅ Usuario creado con ID: {} (email: {})", u.id, u.email);
            u
        }
        Err(e) => {
            let _ = tx.rollback().await;
            eprintln!("❌ Error creando usuario: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Error creando usuario".to_string(),
                details: Some(e.to_string()),
            });
        }
    };

    // 2. Crear perfil de docente
    let teacher_profile = match sqlx::query_as::<_, TeacherProfile>(
        r#"
        INSERT INTO teacher_profiles 
        (user_id, area_id, full_name, employee_code, specialization, hire_date)
        VALUES ($1, $2, $3, $4, $5, CURRENT_DATE)
        RETURNING user_id, area_id, full_name, specialization, hire_date, employee_code
        "#,
    )
    .bind(user.id)
    .bind(Some(area_id))
    .bind(&body.full_name)
    .bind(body.employee_code.as_ref())
    .bind(body.specialization.as_ref())
    .fetch_one(&mut *tx)
    .await
    {
        Ok(p) => {
            println!(
                "✅ Perfil de docente creado - user_id: {}, area_id: {}",
                p.user_id,
                p.area_id.unwrap_or(0)
            );
            p
        }
        Err(e) => {
            let _ = tx.rollback().await;
            eprintln!("❌ Error creando perfil de docente: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Error creando perfil de docente".to_string(),
                details: Some(e.to_string()),
            });
        }
    };

    if let Err(e) = tx.commit().await {
        eprintln!("❌ Error confirmando transacción: {:?}", e);
        return HttpResponse::InternalServerError().json(ErrorResponse {
            error: "Error confirmando registro".to_string(),
            details: Some(e.to_string()),
        });
    }

    println!(
        "🎉 Docente registrado exitosamente: {} (DNI: {}, Área ID: {})",
        body.email, body.dni, area_id
    );

    HttpResponse::Created().json(ApiResponse {
        success: true,
        message: "Docente registrado exitosamente".to_string(),
        data: Some(UserResponse {
            id: user.id,
            email: user.email.clone(),
            role: user.role.to_string(),
            status: user.status.to_string(),
            profile_data: serde_json::json!({
                "full_name": teacher_profile.full_name,
                "area_id": teacher_profile.area_id,
                "employee_code": teacher_profile.employee_code,
                "hire_date": teacher_profile.hire_date,
            }),
        }),
    })
}

#[get("/api/auth/me")]
pub async fn get_current_user(data: web::Data<AppState>, req: HttpRequest) -> impl Responder {
    // Obtener firebase_uid del header
    let firebase_uid = match req.headers().get("X-Firebase-UID") {
        Some(header) => match header.to_str() {
            Ok(uid) => uid.to_string(),
            Err(_) => {
                return HttpResponse::BadRequest().json(ErrorResponse {
                    error: "Header inválido".to_string(),
                    details: None,
                });
            }
        },
        None => {
            return HttpResponse::Unauthorized().json(ErrorResponse {
                error: "No autenticado".to_string(),
                details: Some("Falta header X-Firebase-UID".to_string()),
            });
        }
    };

    // Buscar usuario
    match sqlx::query_as::<_, User>(
        r#"
        SELECT id, firebase_uid, email, role, status, 
               created_at, updated_at, last_login, 
               profile_photo_url, phone
        FROM users 
        WHERE firebase_uid = $1
        "#,
    )
    .bind(&firebase_uid)
    .fetch_optional(&data.pool)
    .await
    {
        Ok(Some(user)) => {
            // Obtener perfil según rol (usando el ENUM)
            let profile_data = match user.role {
                UserRole::Alumno => {
                    sqlx::query_as::<_, StudentProfile>(
                        "SELECT user_id, dni, full_name, date_of_birth, gender, address, enrollment_code, enrollment_date FROM student_profiles WHERE user_id = $1"
                    )
                    .bind(user.id)
                    .fetch_optional(&data.pool)
                    .await
                    .ok()
                    .flatten()
                    .map(|p| serde_json::to_value(p).unwrap_or(serde_json::json!({})))
                    .unwrap_or(serde_json::json!({}))
                }
                UserRole::Apoderado => {
                    sqlx::query_as::<_, GuardianProfile>(
                        "SELECT user_id, full_name, dni, relationship_type, occupation, workplace, emergency_phone FROM guardian_profiles WHERE user_id = $1"
                    )
                    .bind(user.id)
                    .fetch_optional(&data.pool)
                    .await
                    .ok()
                    .flatten()
                    .map(|p| serde_json::to_value(p).unwrap_or(serde_json::json!({})))
                    .unwrap_or(serde_json::json!({}))
                }
                UserRole::Docente => {
                    sqlx::query_as::<_, TeacherProfile>(
                        "SELECT user_id, area_id, full_name, specialization, hire_date, employee_code FROM teacher_profiles WHERE user_id = $1"
                    )
                    .bind(user.id)
                    .fetch_optional(&data.pool)
                    .await
                    .ok()
                    .flatten()
                    .map(|p| serde_json::to_value(p).unwrap_or(serde_json::json!({})))
                    .unwrap_or(serde_json::json!({}))
                }
                UserRole::Admin => serde_json::json!({})
            };

            HttpResponse::Ok().json(ApiResponse {
                success: true,
                message: "Usuario encontrado".to_string(),
                data: Some(UserResponse {
                    id: user.id,
                    email: user.email.clone(),
                    role: user.role.to_string(),
                    status: user.status.to_string(),
                    profile_data,
                }),
            })
        }
        Ok(None) => HttpResponse::NotFound().json(ErrorResponse {
            error: "Usuario no encontrado".to_string(),
            details: None,
        }),
        Err(e) => {
            eprintln!("Error buscando usuario: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Error buscando usuario".to_string(),
                details: Some(e.to_string()),
            })
        }
    }
}

async fn try_link_student_by_name(
    pool: &PgPool,
    user_id: i32,
    full_name: &str,
    dni: &str,
) -> Result<Option<i32>, sqlx::Error> {
    tracing::info!("🔍 Buscando estudiante para vincular: '{}'", full_name);

    // ✅ AHORA usa la función normalize_name de SQL directamente
    let student = sqlx::query(
        r#"
        SELECT id, full_name, section_id
        FROM students
        WHERE user_id IS NULL
          AND public.normalize_name(full_name) = public.normalize_name($1)
        LIMIT 1
        "#,
    )
    .bind(full_name) // ✅ Ya no necesitas normalizar en Rust
    .fetch_optional(pool)
    .await?;

    if let Some(student) = student {
        let student_id: i32 = student.try_get("id")?;
        let student_full_name: String = student.try_get("full_name")?;

        tracing::info!(
            "✅ Encontrado estudiante ID {} con nombre: '{}'",
            student_id,
            student_full_name
        );

        // Actualizar student con user_id y dni
        sqlx::query(
            r#"
            UPDATE students
            SET user_id = $1, dni = $2
            WHERE id = $3
            "#,
        )
        .bind(user_id)
        .bind(dni)
        .bind(student_id)
        .execute(pool)
        .await?;

        // Registrar en auditoría
        sqlx::query(
            r#"
            INSERT INTO student_profile_links (student_id, user_id, linked_by_method)
            VALUES ($1, $2, 'full_name_manual')
            ON CONFLICT (student_id, user_id) DO UPDATE
            SET linked_by_method = 'full_name_manual',
                linked_at = NOW()
            "#,
        )
        .bind(student_id)
        .bind(user_id)
        .execute(pool)
        .await?;

        tracing::info!(
            "✅ Estudiante {} vinculado exitosamente: user_id={}, dni={}",
            student_full_name,
            user_id,
            dni
        );

        Ok(Some(student_id))
    } else {
        tracing::warn!(
            "⚠️ No se encontró estudiante sin vincular con nombre: '{}'",
            full_name
        );
        Ok(None)
    }
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(register_alumno)
        .service(register_apoderado)
        .service(register_docente)
        .service(get_current_user);
}
