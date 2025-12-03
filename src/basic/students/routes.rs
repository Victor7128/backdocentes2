use crate::basic::students::models::*;
use crate::AppState;
use actix_web::{delete, get, post, put, web, HttpResponse, Responder};
use actix_multipart::Multipart;
use futures_util::StreamExt;
use sqlx::Row;

#[post("/sections/{sec_id}/students")]
pub async fn create_student(
    path: web::Path<i32>,
    data: web::Data<AppState>,
    body: web::Json<NewName>,
) -> impl Responder {
    let sec_id = path.into_inner();
    let rec = sqlx::query_as::<_, Student>(
        "INSERT INTO students (section_id, full_name) VALUES ($1,$2) RETURNING id, section_id, full_name, user_id, dni"
    )
    .bind(sec_id)
    .bind(&body.full_name)
    .fetch_one(&data.pool)
    .await
    .unwrap();
    HttpResponse::Ok().json(rec)
}

#[get("/sections/{sec_id}/students")]
pub async fn list_students(path: web::Path<i32>, data: web::Data<AppState>) -> impl Responder {
    let sec_id = path.into_inner();
    let rows = sqlx::query_as::<_, Student>(
        "SELECT id, section_id, full_name, user_id, dni FROM students WHERE section_id=$1 ORDER BY full_name",
    )
    .bind(sec_id)
    .fetch_all(&data.pool)
    .await
    .unwrap();
    HttpResponse::Ok().json(rows)
}

#[put("/students/{id}")]
pub async fn update_student(
    path: web::Path<i32>,
    data: web::Data<AppState>,
    body: web::Json<NewName>,
) -> impl Responder {
    let id = path.into_inner();
    let rec = sqlx::query_as::<_, Student>(
        "UPDATE students SET full_name=$1 WHERE id=$2 RETURNING id, section_id, full_name, user_id, dni",
    )
    .bind(&body.full_name)
    .bind(id)
    .fetch_one(&data.pool)
    .await
    .unwrap();
    HttpResponse::Ok().json(rec)
}

#[delete("/students/{id}")]
pub async fn delete_student(path: web::Path<i32>, data: web::Data<AppState>) -> impl Responder {
    let id = path.into_inner();
    sqlx::query("DELETE FROM students WHERE id=$1")
        .bind(id)
        .execute(&data.pool)
        .await
        .unwrap();
    HttpResponse::NoContent().finish()
}

#[post("/sections/{sec_id}/students/import")]
pub async fn import_students_json(
    path: web::Path<i32>,
    data: web::Data<AppState>, // ← Cambia esto
    body: web::Json<BatchStudentsIn>,
) -> impl Responder {
    let sec_id = path.into_inner();
    let mut successes = Vec::new();
    let mut errors = Vec::new();

    for s in &body.students {
        let name = s.full_name.trim();
        if name.is_empty() {
            errors.push(format!("Nombre vacío"));
            continue;
        }
        let res = sqlx::query_as::<_, Student>(
            "INSERT INTO students (section_id, full_name) VALUES ($1,$2) RETURNING id, section_id, full_name, user_id, dni",
        )
        .bind(sec_id)
        .bind(name)
        .fetch_one(&data.pool)  // ← Cambia esto a data.pool
        .await;

        match res {
            Ok(student) => successes.push(student.full_name),
            Err(e) => errors.push(format!("{}: {}", name, e)),
        }
    }

    HttpResponse::Ok().json(serde_json::json!({
        "imported": successes.len(),
        "successes": successes,
        "errors": errors,
    }))
}

#[post("/sections/{sec_id}/students/import_csv")]
pub async fn import_students_csv(
    path: web::Path<i32>,
    data: web::Data<AppState>,
    mut payload: Multipart,
) -> impl Responder {
    let sec_id = path.into_inner();
    let mut csv_data = Vec::new();

    // Leer el archivo del multipart
    while let Some(field_result) = payload.next().await {
        match field_result {
            Ok(mut field) => {
                while let Some(chunk_result) = field.next().await {
                    match chunk_result {
                        Ok(chunk) => csv_data.extend_from_slice(&chunk),
                        Err(e) => {
                            return HttpResponse::BadRequest().json(
                                serde_json::json!({"error": format!("Error leyendo chunk: {}", e)}),
                            );
                        }
                    }
                }
                break;
            }
            Err(e) => {
                return HttpResponse::BadRequest()
                    .json(serde_json::json!({"error": format!("Error leyendo campo: {}", e)}));
            }
        }
    }

    let content = String::from_utf8_lossy(&csv_data);
    let lines: Vec<&str> = content
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    if lines.len() < 2 {
        return HttpResponse::BadRequest().json(
            serde_json::json!({"error": "El archivo debe tener encabezado y al menos un alumno."}),
        );
    }

    // Verificar que el header sea válido
    let header = lines[0].to_lowercase();
    if !header.contains("full_name") && !header.contains("nombre") {
        return HttpResponse::BadRequest().json(
            serde_json::json!({"error": "El archivo debe tener una columna llamada 'full_name' o 'nombre'."}),
        );
    }

    let mut successes = Vec::new();
    let mut errors = Vec::new();

    // Procesar cada línea después del header
    for (i, line) in lines.iter().enumerate().skip(1) {
        let name = line.trim();

        if name.is_empty() {
            errors.push(format!("Fila {}: nombre vacío", i + 1));
            continue;
        }

        // Guardar exactamente como viene (sin invertir)
        let res = sqlx::query_as::<_, Student>(
            "INSERT INTO students (section_id, full_name) VALUES ($1,$2) RETURNING id, section_id, full_name, user_id, dni",
        )
        .bind(sec_id)
        .bind(name)
        .fetch_one(&data.pool)
        .await;

        match res {
            Ok(student) => successes.push(student.full_name),
            Err(e) => {
                let err_msg = if let sqlx::Error::Database(db_err) = &e {
                    if db_err.code().as_deref() == Some("23505") {
                        format!("{}: ya existe en la sección", name)
                    } else {
                        format!("{}: {}", name, db_err.message())
                    }
                } else {
                    format!("{}: {}", name, e)
                };
                errors.push(err_msg);
            }
        }
    }

    HttpResponse::Ok().json(serde_json::json!({
        "imported": successes.len(),
        "successes": successes,
        "errors": errors,
    }))
}

#[post("/sections/{sec_id}/students/import_txt")]
pub async fn import_students_txt(
    path: web::Path<i32>,
    data: web::Data<AppState>,
    mut payload: Multipart,
) -> impl Responder {
    let sec_id = path.into_inner();
    let mut txt_data = Vec::new();

    // Leer el archivo del multipart con manejo de errores
    while let Some(field_result) = payload.next().await {
        match field_result {
            Ok(mut field) => {
                while let Some(chunk_result) = field.next().await {
                    match chunk_result {
                        Ok(chunk) => txt_data.extend_from_slice(&chunk),
                        Err(e) => {
                            return HttpResponse::BadRequest().json(
                                serde_json::json!({"error": format!("Error leyendo chunk: {}", e)}),
                            );
                        }
                    }
                }
                break;
            }
            Err(e) => {
                return HttpResponse::BadRequest()
                    .json(serde_json::json!({"error": format!("Error leyendo campo: {}", e)}));
            }
        }
    }

    if txt_data.is_empty() {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"error": "No se recibió ningún archivo o está vacío"}));
    }

    let content = String::from_utf8_lossy(&txt_data);

    // Procesar líneas (sin invertir, mantener orden original)
    let lines: Vec<String> = content
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    if lines.is_empty() {
        return HttpResponse::BadRequest().json(
            serde_json::json!({"error": "El archivo está vacío o no contiene nombres válidos."}),
        );
    }

    let mut successes = Vec::new();
    let mut errors = Vec::new();

    for name in lines.iter() {
        let res = sqlx::query_as::<_, Student>(
            "INSERT INTO students (section_id, full_name) VALUES ($1,$2) RETURNING id, section_id, full_name, user_id, dni",
        )
        .bind(sec_id)
        .bind(name)
        .fetch_one(&data.pool)
        .await;

        match res {
            Ok(student) => successes.push(student.full_name),
            Err(e) => {
                let err_msg = if let sqlx::Error::Database(db_err) = &e {
                    if db_err.code().as_deref() == Some("23505") {
                        format!("{}: ya existe en la sección", name)
                    } else {
                        format!("{}: {}", name, db_err.message())
                    }
                } else {
                    format!("{}: {}", name, e)
                };
                errors.push(err_msg);
            }
        }
    }

    HttpResponse::Ok().json(serde_json::json!({
        "imported": successes.len(),
        "successes": successes,
        "errors": errors,
    }))
}

#[get("/students/{user_id}/profile")]
pub async fn get_student_profile(
    path: web::Path<i32>,
    data: web::Data<AppState>,
) -> impl Responder {
    let user_id = path.into_inner();

    // Obtener perfil
    let profile = sqlx::query(
        "SELECT user_id, dni, full_name, date_of_birth, gender, address, enrollment_code, enrollment_date 
         FROM student_profiles WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_optional(&data.pool)
    .await;

    let profile_data = match profile {
        Ok(Some(row)) => serde_json::json!({
            "user_id": row.try_get::<i32, _>("user_id").unwrap(),
            "dni": row.try_get::<String, _>("dni").unwrap(),
            "full_name": row.try_get::<String, _>("full_name").unwrap(),
            "date_of_birth": row.try_get::<Option<chrono::NaiveDate>, _>("date_of_birth").unwrap(),
            "gender": row.try_get::<Option<String>, _>("gender").unwrap(),
            "address": row.try_get::<Option<String>, _>("address").unwrap(),
            "enrollment_code": row.try_get::<Option<String>, _>("enrollment_code").unwrap(),
            "enrollment_date": row.try_get::<Option<chrono::NaiveDate>, _>("enrollment_date").unwrap(),
        }),
        Ok(None) => {
            return HttpResponse::NotFound().body("Perfil no encontrado");
        }
        Err(e) => {
            eprintln!("Error fetching profile: {:?}", e);
            return HttpResponse::InternalServerError().body("Error al obtener perfil");
        }
    };

    // Obtener secciones donde está matriculado
    let sections = sqlx::query(
        r#"
        SELECT s.id, sec.letter, g.number AS grade_number, b.name AS bimester_name, b.year
        FROM students s
        JOIN sections sec ON sec.id = s.section_id
        JOIN grades g ON g.id = sec.grade_id
        JOIN bimesters b ON b.id = g.bimester_id
        WHERE s.user_id = $1
        ORDER BY b.year DESC, b.id DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(&data.pool)
    .await;

    let sections_data = match sections {
        Ok(rows) => rows
            .into_iter()
            .map(|row| {
                serde_json::json!({
                    "id": row.try_get::<i32, _>("id").unwrap(),
                    "letter": row.try_get::<String, _>("letter").unwrap(),
                    "grade_number": row.try_get::<i32, _>("grade_number").unwrap(),
                    "bimester_name": row.try_get::<String, _>("bimester_name").unwrap(),
                    "year": row.try_get::<i32, _>("year").unwrap(),
                })
            })
            .collect::<Vec<_>>(),
        Err(e) => {
            eprintln!("Error fetching sections: {:?}", e);
            return HttpResponse::InternalServerError().body("Error al obtener secciones");
        }
    };

    HttpResponse::Ok().json(serde_json::json!({
        "profile": profile_data,
        "sections": sections_data
    }))
}

#[get("/students/{user_id}/enrollments")]
pub async fn get_student_enrollments(
    path: web::Path<i32>,
    data: web::Data<AppState>,
) -> impl Responder {
    let user_id = path.into_inner();

    let rows = sqlx::query(
        r#"
        SELECT 
            s.id AS student_id,
            s.full_name,
            sec.letter,
            g.number AS grade_number,
            b.name AS bimester_name,
            b.year
        FROM students s
        JOIN sections sec ON sec.id = s.section_id
        JOIN grades g ON g.id = sec.grade_id
        JOIN bimesters b ON b.id = g.bimester_id
        WHERE s.user_id = $1
        ORDER BY b.year DESC, b.id DESC, g.number, sec.letter
        "#,
    )
    .bind(user_id)
    .fetch_all(&data.pool)
    .await;

    match rows {
        Ok(data) => {
            let enrollments: Vec<LinkedStudent> = data
                .into_iter()
                .map(|row| LinkedStudent {
                    student_id: row.try_get("student_id").unwrap(),
                    full_name: row.try_get("full_name").unwrap(),
                    section_letter: row.try_get("letter").unwrap(),
                    grade_number: row.try_get("grade_number").unwrap(),
                    bimester_name: row.try_get("bimester_name").unwrap(),
                    year: row.try_get("year").unwrap(),
                })
                .collect();

            HttpResponse::Ok().json(enrollments)
        }
        Err(e) => {
            eprintln!("Error fetching enrollments: {:?}", e);
            HttpResponse::InternalServerError().body("Error al obtener matrículas")
        }
    }
}

#[get("/students/{user_id}/grades")]
pub async fn get_student_grades(path: web::Path<i32>, data: web::Data<AppState>) -> impl Responder {
    let user_id = path.into_inner();

    let rows = sqlx::query(
        r#"
        SELECT
            s.full_name,
            sec.letter AS section_letter,
            g.number AS grade_number,
            b.name AS bimester_name,
            sess.id AS session_id,
            sess.title AS session_title,
            comp.id AS competency_id,
            comp.name AS competency_name,
            abl.id AS ability_id,
            abl.name AS ability_name,
            crt.id AS criterion_id,
            crt.name AS criterion_name,
            ei.value::text AS value,
            ei.observation,
            ei.updated_at
        FROM evaluation_items ei
        JOIN students s ON s.id = ei.student_id
        JOIN sessions sess ON sess.id = ei.session_id
        JOIN sections sec ON sec.id = sess.section_id
        JOIN grades g ON g.id = sec.grade_id
        JOIN bimesters b ON b.id = g.bimester_id
        JOIN competencies comp ON comp.id = ei.competency_id
        JOIN abilities abl ON abl.id = ei.ability_id
        JOIN criteria crt ON crt.id = ei.criterion_id
        WHERE s.user_id = $1
        ORDER BY b.id, sess.number, comp.number, abl.number, crt.number
        "#,
    )
    .bind(user_id)
    .fetch_all(&data.pool)
    .await;

    if let Err(e) = rows {
        eprintln!("SQL ERROR: {:?}", e);
        return HttpResponse::InternalServerError().body("Error al obtener notas");
    }

    let rows = rows.unwrap();

    use std::collections::HashMap;

    // Agrupadores
    let mut sessions_map: HashMap<i32, StudentGradeSession> = HashMap::new();
    let mut competencies_map: HashMap<(i32, i32), StudentGradeCompetency> = HashMap::new();
    let mut abilities_map: HashMap<(i32, i32, i32), StudentGradeAbility> = HashMap::new();

    for row in rows {
        let session_id: i32 = row.try_get("session_id").unwrap();
        let competency_id: i32 = row.try_get("competency_id").unwrap();
        let ability_id: i32 = row.try_get("ability_id").unwrap();

        // Crear sesión si no existe
        sessions_map
            .entry(session_id)
            .or_insert_with(|| StudentGradeSession {
                bimester_name: row.try_get("bimester_name").unwrap(),
                grade_number: row.try_get("grade_number").unwrap(),
                section_letter: row.try_get("section_letter").unwrap(),
                session_title: row.try_get("session_title").unwrap(),
                competencies: vec![],
            });

        // Crear competencia si no existe
        competencies_map
            .entry((session_id, competency_id))
            .or_insert_with(|| StudentGradeCompetency {
                competency_name: row.try_get("competency_name").unwrap(),
                abilities: vec![],
            });

        // Crear habilidad si no existe
        abilities_map
            .entry((session_id, competency_id, ability_id))
            .or_insert_with(|| StudentGradeAbility {
                ability_name: row.try_get("ability_name").unwrap(),
                criteria: vec![],
            });

        // Agregar criterio
        let criterion = StudentGradeCriterion {
            criterion_name: row.try_get("criterion_name").unwrap(),
            value: row.try_get("value").unwrap(),
            observation: row.try_get("observation").ok(),
            updated_at: row.try_get("updated_at").unwrap(),
        };

        abilities_map
            .get_mut(&(session_id, competency_id, ability_id))
            .unwrap()
            .criteria
            .push(criterion);
    }

    // Enlazar abilities → competencies
    for ((session_id, competency_id), competency) in competencies_map {
        let mut merged_competency = competency;

        for ((s_id, c_id, _), ability) in &abilities_map {
            if *s_id == session_id && *c_id == competency_id {
                merged_competency.abilities.push(ability.clone());
            }
        }

        sessions_map
            .get_mut(&session_id)
            .unwrap()
            .competencies
            .push(merged_competency);
    }

    // Convertir a vector
    let mut sessions: Vec<StudentGradeSession> = sessions_map.into_iter().map(|(_, v)| v).collect();

    // Orden por título
    sessions.sort_by(|a, b| a.session_title.cmp(&b.session_title));

    HttpResponse::Ok().json(sessions)
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(create_student)
        .service(list_students)
        .service(update_student)
        .service(delete_student)
        .service(import_students_json)
        .service(import_students_csv)
        .service(import_students_txt)
        .service(get_student_grades)
        .service(get_student_profile)
        .service(get_student_enrollments);
}
