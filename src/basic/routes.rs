use crate::basic::models::*;
use crate::AppState;
use actix_web::{delete, get, post, web, HttpResponse, Responder};
use serde_json::json;
use sqlx::Row;

#[post("/bimesters")]
pub async fn create_bimester(
    data: web::Data<AppState>,
    body: web::Json<serde_json::Value>,
) -> impl Responder {
    let name = body.get("name").and_then(|v| v.as_str()).unwrap_or("I");
    let rec = sqlx::query_as::<_, Bimester>(
        "INSERT INTO bimesters (name) VALUES ($1) RETURNING id, name",
    )
    .bind(name)
    .fetch_one(&data.pool)
    .await
    .unwrap();
    HttpResponse::Ok().json(rec)
}

#[get("/bimesters")]
pub async fn list_bimesters(data: web::Data<AppState>) -> impl Responder {
    let rows = sqlx::query_as::<_, Bimester>("SELECT id, name FROM bimesters ORDER BY id")
        .fetch_all(&data.pool)
        .await
        .unwrap();
    HttpResponse::Ok().json(rows)
}

#[get("/bimesters/full")]
pub async fn list_bimesters_full(data: web::Data<AppState>) -> impl Responder {
    // Obtener todos los bimestres
    let bimesters =
        match sqlx::query_as::<_, Bimester>("SELECT id, name FROM bimesters ORDER BY id")
            .fetch_all(&data.pool)
            .await
        {
            Ok(rows) => rows,
            Err(e) => {
                eprintln!("Error fetching bimesters: {:?}", e);
                return HttpResponse::InternalServerError().body("Error al obtener bimestres");
            }
        };

    let mut result = Vec::new();

    for bimester in bimesters {
        // Obtener grados del bimestre
        let grades = match sqlx::query_as::<_, Grade>(
            "SELECT id, bimester_id, number FROM grades WHERE bimester_id=$1 ORDER BY number",
        )
        .bind(bimester.id)
        .fetch_all(&data.pool)
        .await
        {
            Ok(rows) => rows,
            Err(e) => {
                eprintln!(
                    "Error fetching grades for bimester {}: {:?}",
                    bimester.id, e
                );
                continue;
            }
        };

        let mut grades_with_sections = Vec::new();

        for grade in grades {
            // Obtener secciones del grado
            let sections = match sqlx::query_as::<_, Section>(
                "SELECT id, grade_id, letter FROM sections WHERE grade_id=$1 ORDER BY letter",
            )
            .bind(grade.id)
            .fetch_all(&data.pool)
            .await
            {
                Ok(rows) => rows,
                Err(e) => {
                    eprintln!("Error fetching sections for grade {}: {:?}", grade.id, e);
                    vec![] // Continuar con secciones vacías si hay error
                }
            };

            grades_with_sections.push(json!({
                "id": grade.id,
                "bimester_id": grade.bimester_id,
                "number": grade.number,
                "sections": sections
            }));
        }

        result.push(json!({
            "id": bimester.id,
            "name": bimester.name,
            "grades": grades_with_sections
        }));
    }

    HttpResponse::Ok().json(result)
}

#[post("/bimesters/{b_id}/grades")]
pub async fn create_grade(
    path: web::Path<i32>,
    data: web::Data<AppState>,
    body: web::Json<serde_json::Value>,
) -> impl Responder {
    let b_id = path.into_inner();
    // Forzamos i32 desde el body
    let number = body.get("number").and_then(|v| v.as_i64()).unwrap_or(1) as i32;

    // Cambia i64 por i32 aquí:
    let existing = sqlx::query_scalar::<_, Option<i32>>(
        "SELECT 1 FROM grades WHERE bimester_id=$1 AND number=$2",
    )
    .bind(b_id)
    .bind(number)
    .fetch_optional(&data.pool)
    .await
    .unwrap();

    if existing.is_some() {
        return HttpResponse::BadRequest().body("Ese grado ya existe en este bimestre");
    }

    let rec = sqlx::query_as::<_, Grade>(
        "INSERT INTO grades (bimester_id, number) VALUES ($1,$2) RETURNING id, bimester_id, number",
    )
    .bind(b_id)
    .bind(number)
    .fetch_one(&data.pool)
    .await
    .unwrap();
    HttpResponse::Ok().json(rec)
}

#[delete("/grades/{g_id}")]
pub async fn delete_grade(path: web::Path<i32>, data: web::Data<AppState>) -> impl Responder {
    let g_id = path.into_inner();
    let result = sqlx::query("DELETE FROM grades WHERE id = $1")
        .bind(g_id)
        .execute(&data.pool)
        .await;

    match result {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => {
            eprintln!("Error deleting grade: {:?}", e);
            HttpResponse::InternalServerError().body("No se pudo eliminar el grado")
        }
    }
}

#[get("/bimesters/{b_id}/grades")]
pub async fn list_grades(path: web::Path<i32>, data: web::Data<AppState>) -> impl Responder {
    let b_id = path.into_inner();
    let rows = sqlx::query_as::<_, Grade>(
        "SELECT id, bimester_id, number FROM grades WHERE bimester_id=$1 ORDER BY number",
    )
    .bind(b_id)
    .fetch_all(&data.pool)
    .await
    .unwrap();
    HttpResponse::Ok().json(rows)
}

#[get("/grades/{g_id}")]
pub async fn get_grade(path: web::Path<i32>, data: web::Data<AppState>) -> impl Responder {
    let g_id = path.into_inner();
    let grade =
        sqlx::query_as::<_, Grade>("SELECT id, bimester_id, number FROM grades WHERE id=$1")
            .bind(g_id)
            .fetch_optional(&data.pool)
            .await
            .unwrap();
    match grade {
        Some(g) => HttpResponse::Ok().json(g),
        None => HttpResponse::NotFound().body("Grade not found"),
    }
}

#[post("/grades/{g_id}/sections")]
pub async fn create_section(
    path: web::Path<i32>,
    data: web::Data<AppState>,
    body: web::Json<serde_json::Value>, // <-- Acepta body!
) -> impl Responder {
    let g_id = path.into_inner();

    // Si el body tiene "letter", úsalo, si no, calcula la siguiente libre.
    let letter = body
        .get("letter")
        .and_then(|v| v.as_str())
        .map(|v| v.to_string());

    let used_letters: Vec<(String,)> =
        sqlx::query_as::<_, (String,)>("SELECT letter FROM sections WHERE grade_id=$1")
            .bind(g_id)
            .fetch_all(&data.pool)
            .await
            .unwrap();

    let next_letter = match letter {
        Some(l) => {
            // Valida que no esté usada
            if used_letters.iter().any(|(used,)| used == &l) {
                return HttpResponse::BadRequest().body("Letra ya utilizada para este grado");
            }
            l
        }
        None => {
            // Genera la siguiente letra automáticamente
            let mut max_char = 'A';
            for (l,) in used_letters {
                if let Some(ch) = l.chars().next() {
                    if ch > max_char {
                        max_char = ch;
                    }
                }
            }
            (((max_char as u8) + 1u8) as char).to_string()
        }
    };

    let rec = sqlx::query_as::<_, Section>(
        "INSERT INTO sections (grade_id, letter) VALUES ($1,$2) RETURNING id, grade_id, letter",
    )
    .bind(g_id)
    .bind(&next_letter)
    .fetch_one(&data.pool)
    .await
    .unwrap();
    HttpResponse::Ok().json(rec)
}

#[get("/grades/{g_id}/sections")]
pub async fn list_sections(path: web::Path<i32>, data: web::Data<AppState>) -> impl Responder {
    let g_id = path.into_inner();
    let rows = sqlx::query_as::<_, Section>(
        "SELECT id, grade_id, letter FROM sections WHERE grade_id=$1 ORDER BY letter",
    )
    .bind(g_id)
    .fetch_all(&data.pool)
    .await
    .unwrap();
    HttpResponse::Ok().json(rows)
}

#[delete("/sections/{sec_id}")]
pub async fn delete_section(path: web::Path<i32>, data: web::Data<AppState>) -> impl Responder {
    let sec_id = path.into_inner();
    let result = sqlx::query("DELETE FROM sections WHERE id = $1")
        .bind(sec_id)
        .execute(&data.pool)
        .await;

    match result {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => {
            eprintln!("Error deleting section: {:?}", e);
            HttpResponse::InternalServerError().body("No se pudo eliminar la sección")
        }
    }
}

#[get("/sections/{sec_id}")]
pub async fn get_section(path: web::Path<i32>, data: web::Data<AppState>) -> impl Responder {
    let sec_id = path.into_inner();
    let section =
        sqlx::query_as::<_, Section>("SELECT id, grade_id, letter FROM sections WHERE id=$1")
            .bind(sec_id)
            .fetch_optional(&data.pool)
            .await
            .unwrap();
    match section {
        Some(s) => HttpResponse::Ok().json(s),
        None => HttpResponse::NotFound().body("Section not found"),
    }
}

#[get("/sections/{section_id}/consolidado")]
pub async fn get_consolidado_section(
    path: web::Path<i32>,
    data: web::Data<AppState>,
) -> impl Responder {
    let section_id = path.into_inner();

    // Estudiantes
    let students = match sqlx::query(
        "SELECT id, full_name FROM students WHERE section_id = $1 ORDER BY full_name",
    )
    .bind(section_id)
    .fetch_all(&data.pool)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("Error fetching students: {:?}", e);
            return HttpResponse::InternalServerError().body("Error fetching students");
        }
    };

    // Sesiones
    let sessions = match sqlx::query(
        "SELECT id, title, number FROM sessions WHERE section_id = $1 ORDER BY number",
    )
    .bind(section_id)
    .fetch_all(&data.pool)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("Error fetching sessions: {:?}", e);
            return HttpResponse::InternalServerError().body("Error fetching sessions");
        }
    };

    // Competencias (agrega session_id)
    let competencies = match sqlx::query(
        "SELECT id, session_id, COALESCE(name, 'Competencia '||number::text) AS display_name
         FROM competencies
         WHERE session_id IN (SELECT id FROM sessions WHERE section_id = $1)
         ORDER BY session_id, number",
    )
    .bind(section_id)
    .fetch_all(&data.pool)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("Error fetching competencies: {:?}", e);
            return HttpResponse::InternalServerError().body("Error fetching competencies");
        }
    };

    // Habilidades (agrega competency_id)
    let abilities = match sqlx::query(
        "SELECT id, competency_id, COALESCE(name, 'Capacidad '||number::text) AS display_name
         FROM abilities
         WHERE competency_id IN (
             SELECT id FROM competencies WHERE session_id IN (
                 SELECT id FROM sessions WHERE section_id = $1
             )
         )
         ORDER BY competency_id, number",
    )
    .bind(section_id)
    .fetch_all(&data.pool)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("Error fetching abilities: {:?}", e);
            return HttpResponse::InternalServerError().body("Error fetching abilities");
        }
    };

    // Criterios (agrega ability_id)
    let criteria = match sqlx::query(
        "SELECT id, ability_id, COALESCE(name, 'C'||number::text) AS display_name
         FROM criteria
         WHERE ability_id IN (
            SELECT id FROM abilities WHERE competency_id IN (
                SELECT id FROM competencies WHERE session_id IN (
                    SELECT id FROM sessions WHERE section_id = $1
                )
            )
         )
         ORDER BY ability_id, number",
    )
    .bind(section_id)
    .fetch_all(&data.pool)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("Error fetching criteria: {:?}", e);
            return HttpResponse::InternalServerError().body("Error fetching criteria");
        }
    };

    // Valores de evaluación (con criterion_id)
    let values = match sqlx::query(
        "SELECT student_id, criterion_id, value::text AS value
         FROM evaluation_items
         WHERE session_id IN (SELECT id FROM sessions WHERE section_id = $1)",
    )
    .bind(section_id)
    .fetch_all(&data.pool)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("Error fetching values: {:?}", e);
            return HttpResponse::InternalServerError().body("Error fetching values");
        }
    };

    // Observaciones por habilidad y estudiante
    let observations = match sqlx::query(
        "SELECT student_id, ability_id, STRING_AGG(DISTINCT COALESCE(observation, '')::text, ' || ') AS observation
         FROM evaluation_items
         WHERE session_id IN (SELECT id FROM sessions WHERE section_id = $1)
           AND COALESCE(observation, '') <> ''
         GROUP BY student_id, ability_id"
    )
    .bind(section_id)
    .fetch_all(&data.pool)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("Error fetching observations: {:?}", e);
            return HttpResponse::InternalServerError().body("Error fetching observations");
        }
    };

    let resp = json!({
        "students": students.iter().map(|r| json!({
            "id": r.try_get::<i32, _>("id").unwrap(),
            "full_name": r.try_get::<String, _>("full_name").unwrap()
        })).collect::<Vec<_>>(),
        "sessions": sessions.iter().map(|r| json!({
            "id": r.try_get::<i32, _>("id").unwrap(),
            "title": r.try_get::<Option<String>, _>("title").unwrap(),
            "number": r.try_get::<i32, _>("number").unwrap()
        })).collect::<Vec<_>>(),
        "competencies": competencies.iter().map(|r| json!({
            "id": r.try_get::<i32, _>("id").unwrap(),
            "session_id": r.try_get::<i32, _>("session_id").unwrap(),
            "display_name": r.try_get::<String, _>("display_name").unwrap()
        })).collect::<Vec<_>>(),
        "abilities": abilities.iter().map(|r| json!({
            "id": r.try_get::<i32, _>("id").unwrap(),
            "competency_id": r.try_get::<i32, _>("competency_id").unwrap(),
            "display_name": r.try_get::<String, _>("display_name").unwrap()
        })).collect::<Vec<_>>(),
        "criteria": criteria.iter().map(|r| json!({
            "id": r.try_get::<i32, _>("id").unwrap(),
            "ability_id": r.try_get::<i32, _>("ability_id").unwrap(),
            "display_name": r.try_get::<String, _>("display_name").unwrap()
        })).collect::<Vec<_>>(),
        "values": values.iter().map(|r| json!({
            "student_id": r.try_get::<i32, _>("student_id").unwrap(),
            "criterion_id": r.try_get::<i32, _>("criterion_id").unwrap(),
            "value": r.try_get::<String, _>("value").unwrap()
        })).collect::<Vec<_>>(),
        "observations": observations.iter().map(|r| json!({
            "student_id": r.try_get::<i32, _>("student_id").unwrap(),
            "ability_id": r.try_get::<i32, _>("ability_id").unwrap(),
            "observation": r.try_get::<String, _>("observation").unwrap()
        })).collect::<Vec<_>>(),
    });

    HttpResponse::Ok().json(resp)
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(create_bimester)
        .service(list_bimesters)
        .service(create_grade)
        .service(list_grades)
        .service(create_section)
        .service(list_sections)
        .service(delete_grade)
        .service(get_grade)
        .service(delete_section)
        .service(get_section)
        .service(get_consolidado_section)
        .service(list_bimesters_full);
}
