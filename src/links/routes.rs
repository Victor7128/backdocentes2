use crate::links::models::*;
use crate::AppState;
use actix_web::{get, post, web, HttpResponse, Responder};
use sqlx::Row;
use tracing;

#[post("/admin/link-student")]
pub async fn link_student_to_user(
    data: web::Data<AppState>,
    body: web::Json<LinkStudentIn>,
) -> impl Responder {
    let res = sqlx::query("UPDATE students SET user_id = $1 WHERE id = $2 RETURNING id")
        .bind(body.user_id)
        .bind(body.student_id)
        .fetch_one(&data.pool)
        .await;

    match res {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "message": "Alumno vinculado exitosamente"
        })),
        Err(e) => {
            eprintln!("Error linking student: {:?}", e);
            HttpResponse::InternalServerError().body("Error al vincular")
        }
    }
}

#[get("/admin/unlinked-students")]
pub async fn list_unlinked_students(data: web::Data<AppState>) -> impl Responder {
    let rows = sqlx::query(
        r#"
        SELECT s.id, s.full_name, s.section_id, sec.letter, g.number AS grade_number, b.name AS bimester_name
        FROM students s
        JOIN sections sec ON sec.id = s.section_id
        JOIN grades g ON g.id = sec.grade_id
        JOIN bimesters b ON b.id = g.bimester_id
        WHERE s.user_id IS NULL
        ORDER BY b.year DESC, b.id DESC, g.number, sec.letter, s.full_name
        "#
    )
    .fetch_all(&data.pool)
    .await;

    match rows {
        Ok(data) => {
            let unlinked: Vec<UnlinkedStudent> = data
                .into_iter()
                .map(|row| UnlinkedStudent {
                    id: row.try_get("id").unwrap(),
                    full_name: row.try_get("full_name").unwrap(),
                    section_id: row.try_get("section_id").unwrap(),
                    section_letter: row.try_get("letter").unwrap(),
                    grade_number: row.try_get("grade_number").unwrap(),
                    bimester_name: row.try_get("bimester_name").unwrap(),
                })
                .collect();

            HttpResponse::Ok().json(unlinked)
        }
        Err(e) => {
            eprintln!("Error fetching unlinked students: {:?}", e);
            HttpResponse::InternalServerError().body("Error al obtener alumnos")
        }
    }
}

#[get("/admin/search-students")]
pub async fn search_students(
    query: web::Query<SearchStudentQuery>,
    data: web::Data<AppState>,
) -> impl Responder {
    let search_term = format!("%{}%", query.name.to_lowercase());

    let rows = sqlx::query(
        r#"
        SELECT 
            s.id,
            s.full_name,
            s.section_id,
            s.user_id,
            sec.letter,
            g.number AS grade_number,
            b.name AS bimester_name,
            b.year
        FROM students s
        JOIN sections sec ON sec.id = s.section_id
        JOIN grades g ON g.id = sec.grade_id
        JOIN bimesters b ON b.id = g.bimester_id
        WHERE LOWER(s.full_name) LIKE $1
        ORDER BY b.year DESC, b.id DESC, g.number, sec.letter, s.full_name
        LIMIT 50
        "#,
    )
    .bind(&search_term)
    .fetch_all(&data.pool)
    .await;

    match rows {
        Ok(data) => {
            let results: Vec<_> = data
                .into_iter()
                .map(|row| {
                    serde_json::json!({
                        "id": row.try_get::<i32, _>("id").unwrap(),
                        "full_name": row.try_get::<String, _>("full_name").unwrap(),
                        "section_id": row.try_get::<i32, _>("section_id").unwrap(),
                        "user_id": row.try_get::<Option<i32>, _>("user_id").unwrap(),
                        "section_letter": row.try_get::<String, _>("letter").unwrap(),
                        "grade_number": row.try_get::<i32, _>("grade_number").unwrap(),
                        "bimester_name": row.try_get::<String, _>("bimester_name").unwrap(),
                        "year": row.try_get::<i32, _>("year").unwrap(),
                    })
                })
                .collect();

            HttpResponse::Ok().json(results)
        }
        Err(e) => {
            eprintln!("Error searching students: {:?}", e);
            HttpResponse::InternalServerError().body("Error al buscar alumnos")
        }
    }
}

#[get("/admin/homonyms")]
pub async fn detect_homonyms(data: web::Data<AppState>) -> impl Responder {
    let rows =
        sqlx::query("SELECT * FROM public.detect_student_homonyms() WHERE is_problematic = true")
            .fetch_all(&data.pool)
            .await;

    match rows {
        Ok(data) => {
            let results: Vec<_> = data
                .into_iter()
                .map(|row| {
                    serde_json::json!({
                        "full_name": row.try_get::<String, _>("full_name").unwrap(),
                        "count": row.try_get::<i64, _>("count").unwrap(),
                        "student_ids": row.try_get::<Vec<i32>, _>("student_ids").unwrap(),
                        "user_ids": row.try_get::<Vec<Option<i32>>, _>("user_ids").unwrap(),
                        "is_problematic": row.try_get::<bool, _>("is_problematic").unwrap(),
                    })
                })
                .collect();
            HttpResponse::Ok().json(results)
        }
        Err(e) => {
            eprintln!("Error detecting homonyms: {:?}", e);
            HttpResponse::InternalServerError().body("Error al detectar homónimos")
        }
    }
}

#[post("/admin/unlink-student")]
pub async fn unlink_student(
    data: web::Data<AppState>,
    body: web::Json<UnlinkStudentIn>,
) -> impl Responder {
    let result = sqlx::query_scalar::<_, serde_json::Value>("SELECT public.unlink_student($1)")
        .bind(body.student_id)
        .fetch_one(&data.pool)
        .await;

    match result {
        Ok(json_result) => HttpResponse::Ok().json(json_result),
        Err(e) => {
            eprintln!("Error unlinking student: {:?}", e);
            HttpResponse::InternalServerError().body("Error al desvincular")
        }
    }
}

#[post("/admin/link-student-by-dni")]
pub async fn link_student_by_dni(
    data: web::Data<AppState>,
    body: web::Json<LinkByDniIn>,
) -> impl Responder {
    if body.dni.len() != 8 || !body.dni.chars().all(|c| c.is_numeric()) {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "message": "DNI inválido. Debe tener 8 dígitos"
        }));
    }

    let result =
        sqlx::query_scalar::<_, serde_json::Value>("SELECT public.link_student_by_dni($1, $2)")
            .bind(body.student_id)
            .bind(&body.dni)
            .fetch_one(&data.pool)
            .await;

    match result {
        Ok(json_result) => HttpResponse::Ok().json(json_result),
        Err(e) => {
            eprintln!("Error linking by DNI: {:?}", e);
            HttpResponse::InternalServerError().body("Error al vincular por DNI")
        }
    }
}

#[get("/admin/linking-status")]
pub async fn get_linking_status(data: web::Data<AppState>) -> impl Responder {
    let rows = sqlx::query(
        "SELECT * FROM public.student_linking_status ORDER BY bimester_year DESC, grade_number, section_letter, student_name"
    )
    .fetch_all(&data.pool)
    .await;

    match rows {
        Ok(data) => {
            let results: Vec<_> = data.into_iter().map(|row| {
                serde_json::json!({
                    "student_id": row.try_get::<i32, _>("student_id").unwrap(),
                    "student_name": row.try_get::<String, _>("student_name").unwrap(),
                    "student_dni": row.try_get::<Option<String>, _>("student_dni").unwrap(),
                    "user_id": row.try_get::<Option<i32>, _>("user_id").unwrap(),
                    "section_letter": row.try_get::<String, _>("section_letter").unwrap(),
                    "grade_number": row.try_get::<i32, _>("grade_number").unwrap(),
                    "bimester_name": row.try_get::<String, _>("bimester_name").unwrap(),
                    "profile_dni": row.try_get::<Option<String>, _>("profile_dni").unwrap(),
                    "link_status": row.try_get::<String, _>("link_status").unwrap(),
                    "linked_by_method": row.try_get::<Option<String>, _>("linked_by_method").unwrap(),
                    "issue": row.try_get::<Option<String>, _>("issue").unwrap(),
                })
            }).collect();
            HttpResponse::Ok().json(results)
        }
        Err(e) => {
            eprintln!("Error fetching linking status: {:?}", e);
            HttpResponse::InternalServerError().body("Error al obtener estado")
        }
    }
}

#[post("/admin/backfill-dni")]
pub async fn backfill_dni(data: web::Data<AppState>) -> impl Responder {
    let result = sqlx::query_scalar::<_, serde_json::Value>("SELECT public.backfill_student_dni()")
        .fetch_one(&data.pool)
        .await;

    match result {
        Ok(json_result) => HttpResponse::Ok().json(json_result),
        Err(e) => {
            eprintln!("Error in backfill: {:?}", e);
            HttpResponse::InternalServerError().body("Error en backfill")
        }
    }
}

#[post("/api/validate-dni")]
pub async fn validate_dni(body: web::Json<ReniecRequest>) -> impl Responder {
    // Validar formato de DNI
    if body.dni.len() != 8 || !body.dni.chars().all(|c| c.is_numeric()) {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "message": "DNI inválido. Debe tener 8 dígitos numéricos"
        }));
    }

    // Configurar cliente HTTP
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error creando cliente HTTP: {:?}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "message": "Error interno del servidor"
            }));
        }
    };

    // Construir request a RENIEC
    let reniec_url = "https://apiperu.dev/api/dni";
    let token = "Bearer 9b7d94e281d215abcea99e362033353d6a9fd467f817ac6f92fae5edab452a45";

    let payload = serde_json::json!({
        "dni": body.dni
    });

    tracing::info!("🔍 Consultando RENIEC para DNI: {}", body.dni);

    // Realizar petición
    let response = client
        .post(reniec_url)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .header("Authorization", token)
        .json(&payload)
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();

            if status.is_success() {
                match resp.json::<ReniecResponse>().await {
                    Ok(reniec_data) => {
                        if reniec_data.success {
                            let nombre = reniec_data
                                .data
                                .as_ref()
                                .and_then(|d| d.nombre_completo.clone())
                                .unwrap_or_else(|| "Sin nombre".to_string());
                            tracing::info!("✅ DNI {} encontrado: {}", body.dni, nombre);
                            HttpResponse::Ok().json(reniec_data)
                        } else {
                            tracing::warn!("⚠️ DNI {} no encontrado en RENIEC", body.dni);
                            HttpResponse::NotFound().json(serde_json::json!({
                                "success": false,
                                "message": "DNI no encontrado en RENIEC"
                            }))
                        }
                    }
                    Err(e) => {
                        eprintln!("❌ Error parseando respuesta de RENIEC: {:?}", e);
                        HttpResponse::InternalServerError().json(serde_json::json!({
                            "success": false,
                            "message": "Error procesando respuesta de RENIEC"
                        }))
                    }
                }
            } else {
                eprintln!(
                    "❌ RENIEC respondió con status {}: {:?}",
                    status,
                    resp.text().await
                );
                HttpResponse::BadGateway().json(serde_json::json!({
                    "success": false,
                    "message": format!("Error en servicio RENIEC (status: {})", status)
                }))
            }
        }
        Err(e) => {
            eprintln!("❌ Error conectando con RENIEC: {:?}", e);
            HttpResponse::ServiceUnavailable().json(serde_json::json!({
                "success": false,
                "message": "No se pudo conectar con el servicio RENIEC"
            }))
        }
    }
}

#[post("/admin/guardian-relationships")]
pub async fn create_guardian_relationship(
    data: web::Data<AppState>,
    body: web::Json<CreateGuardianRelationshipIn>,
) -> impl Responder {
    let result = sqlx::query(
        r#"
        INSERT INTO guardian_student_relationships 
        (guardian_user_id, student_user_id, relationship_type, is_primary)
        VALUES ($1, $2, $3, $4)
        RETURNING id
        "#,
    )
    .bind(body.guardian_user_id)
    .bind(body.student_user_id)
    .bind(&body.relationship_type)
    .bind(body.is_primary.unwrap_or(false))
    .fetch_one(&data.pool)
    .await;

    match result {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "message": "Relación creada exitosamente"
        })),
        Err(e) => {
            eprintln!("Error creating relationship: {:?}", e);
            HttpResponse::InternalServerError().body("Error al crear relación")
        }
    }
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(link_student_to_user)
        .service(list_unlinked_students)
        .service(search_students)
        .service(detect_homonyms)
        .service(unlink_student)
        .service(link_student_by_dni)
        .service(get_linking_status)
        .service(backfill_dni)
        .service(validate_dni);
}
