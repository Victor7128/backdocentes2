use crate::models::*;
use actix_web::{delete, get, post, put, web, HttpRequest, HttpResponse, Responder};
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use sqlx::Row;
use tracing;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
}

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

//endpoints para la importación por lotes
use actix_multipart::Multipart;
use futures_util::StreamExt;

#[derive(Deserialize)]
pub struct BatchStudentsIn {
    pub students: Vec<NewName>,
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

//import students txt
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

#[derive(Deserialize)]
pub struct NewSessionIn {
    pub title: Option<String>,
    pub date: String,
}

#[post("/sections/{sec_id}/sessions")]
pub async fn create_session(
    path: web::Path<i32>,
    data: web::Data<AppState>,
    body: web::Json<NewSessionIn>,
) -> impl Responder {
    let sec_id = path.into_inner();

    // Obtener el siguiente número de sesión
    let next = match sqlx::query_scalar::<_, Option<i32>>(
        "SELECT MAX(number) FROM sessions WHERE section_id = $1",
    )
    .bind(sec_id)
    .fetch_one(&data.pool)
    .await
    {
        Ok(n) => n,
        Err(e) => {
            eprintln!("Error al obtener el número de sesión: {:?}", e);
            return HttpResponse::InternalServerError().body("Error en la base de datos");
        }
    };
    let number = next.unwrap_or(0) + 1;

    // Si el campo date está vacío, omite la columna para usar el DEFAULT de la base de datos
    let rec = if body.date.trim().is_empty() {
        match sqlx::query_as::<_, Session>(
            "INSERT INTO sessions (section_id, number, title)
             VALUES ($1, $2, $3)
             RETURNING id, section_id, number, title, date, created_at",
        )
        .bind(sec_id)
        .bind(number)
        .bind(body.title.clone())
        .fetch_one(&data.pool)
        .await
        {
            Ok(rec) => rec,
            Err(e) => {
                eprintln!("Error al crear sesión: {:?}", e);
                return HttpResponse::InternalServerError().body("Error en la base de datos");
            }
        }
    } else {
        let fecha = match body.date.parse::<chrono::NaiveDate>() {
            Ok(f) => f,
            Err(_) => {
                return HttpResponse::BadRequest().body("Fecha inválida. Usa formato YYYY-MM-DD.");
            }
        };
        match sqlx::query_as::<_, Session>(
            "INSERT INTO sessions (section_id, number, title, date)
             VALUES ($1, $2, $3, $4)
             RETURNING id, section_id, number, title, date, created_at",
        )
        .bind(sec_id)
        .bind(number)
        .bind(body.title.clone())
        .bind(fecha)
        .fetch_one(&data.pool)
        .await
        {
            Ok(rec) => rec,
            Err(e) => {
                eprintln!("Error al crear sesión: {:?}", e);
                return HttpResponse::InternalServerError().body("Error en la base de datos");
            }
        }
    };

    HttpResponse::Ok().json(rec)
}

// Listar sesiones de una sección
#[get("/sections/{sec_id}/sessions")]
pub async fn list_sessions(path: web::Path<i32>, data: web::Data<AppState>) -> impl Responder {
    let sec_id = path.into_inner();
    let rows = sqlx::query_as::<_, Session>(
        "SELECT id, section_id, number, title, date, created_at FROM sessions WHERE section_id=$1 ORDER BY number",
    )
    .bind(sec_id)
    .fetch_all(&data.pool)
    .await
    .unwrap();
    HttpResponse::Ok().json(rows)
}

// Obtener una sesión por id
#[get("/sessions/{session_id}")]
pub async fn get_session(path: web::Path<i32>, data: web::Data<AppState>) -> impl Responder {
    let id = path.into_inner();
    let result = sqlx::query_as::<_, Session>(
        "SELECT id, section_id, number, title, date, created_at FROM sessions WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&data.pool)
    .await;

    match result {
        Ok(Some(session)) => HttpResponse::Ok().json(session),
        Ok(None) => HttpResponse::NotFound().body("Sesión no encontrada"),
        Err(e) => {
            eprintln!("Error buscando sesión: {:?}", e);
            HttpResponse::InternalServerError().body("Error al buscar sesión")
        }
    }
}

#[derive(Deserialize)]
pub struct UpdateSessionIn {
    pub title: Option<String>,
    pub date: Option<String>, // "YYYY-MM-DD"
}

#[put("/sessions/{session_id}")]
pub async fn update_session(
    path: web::Path<i32>,
    data: web::Data<AppState>,
    body: web::Json<UpdateSessionIn>,
) -> impl Responder {
    let id = path.into_inner();
    let title = &body.title;

    // Parse date string (si existe) a Option<NaiveDate>
    let parsed_date = match &body.date {
        Some(date_str) if !date_str.trim().is_empty() => {
            match chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                Ok(d) => Some(d),
                Err(_) => {
                    return HttpResponse::BadRequest()
                        .body("Fecha inválida. Usa formato YYYY-MM-DD.");
                }
            }
        }
        _ => None,
    };

    let rec = sqlx::query_as::<_, Session>(
        "UPDATE sessions
         SET title = COALESCE($1, title),
             date = COALESCE($2, date)
         WHERE id = $3
         RETURNING id, section_id, number, title, date, created_at",
    )
    .bind(title)
    .bind(parsed_date) // Aquí ya es Option<NaiveDate>
    .bind(id)
    .fetch_one(&data.pool)
    .await;

    match rec {
        Ok(session) => HttpResponse::Ok().json(session),
        Err(e) => {
            eprintln!("Error actualizando sesión: {:?}", e);
            HttpResponse::InternalServerError().body("No se pudo actualizar la sesión")
        }
    }
}

#[delete("/sessions/{sess_id}")]
pub async fn delete_session(path: web::Path<i32>, data: web::Data<AppState>) -> impl Responder {
    let sess_id = path.into_inner();
    let result = sqlx::query("DELETE FROM sessions WHERE id = $1")
        .bind(sess_id)
        .execute(&data.pool)
        .await;

    match result {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => {
            eprintln!("Error deleting session: {:?}", e);
            HttpResponse::InternalServerError().body("No se pudo eliminar la sesión")
        }
    }
}

// --- PRODUCTOS (por sesión) ---
#[post("/sessions/{sess_id}/products")]
pub async fn create_product(
    path: web::Path<i32>,
    data: web::Data<AppState>,
    body: web::Json<serde_json::Value>,
) -> impl Responder {
    let sess_id = path.into_inner();
    let next = sqlx::query_scalar::<_, Option<i32>>(
        "SELECT MAX(number) FROM products WHERE session_id=$1",
    )
    .bind(sess_id)
    .fetch_one(&data.pool)
    .await
    .unwrap();
    let number = next.unwrap_or(0) + 1;
    let name = body.get("name").and_then(|v| v.as_str());
    let description = body.get("description").and_then(|v| v.as_str());
    let rec = sqlx::query_as::<_, Product>(
        "INSERT INTO products (session_id, number, name, description) VALUES ($1,$2,$3,$4)
         RETURNING id, session_id, number, name, description",
    )
    .bind(sess_id)
    .bind(number as i32)
    .bind(name)
    .bind(description)
    .fetch_one(&data.pool)
    .await
    .unwrap();
    HttpResponse::Ok().json(rec)
}

#[derive(Deserialize)]
pub struct UpdateProductIn {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[put("/products/{product_id}")]
pub async fn update_product(
    path: web::Path<i32>,
    data: web::Data<AppState>,
    body: web::Json<UpdateProductIn>,
) -> impl Responder {
    let id = path.into_inner();
    let name = &body.name;
    let desc = &body.description;
    let rec = sqlx::query_as::<_, Product>(
        "UPDATE products SET name=COALESCE($1,name), description=COALESCE($2,description) WHERE id=$3 RETURNING id, session_id, number, name, description"
    )
    .bind(name)
    .bind(desc)
    .bind(id)
    .fetch_one(&data.pool)
    .await
    .unwrap();
    HttpResponse::Ok().json(rec)
}

#[delete("/products/{product_id}")]
pub async fn delete_product(path: web::Path<i32>, data: web::Data<AppState>) -> impl Responder {
    let id = path.into_inner();
    sqlx::query("DELETE FROM products WHERE id=$1")
        .bind(id)
        .execute(&data.pool)
        .await
        .unwrap();
    HttpResponse::NoContent().finish()
}

#[get("/sessions/{sess_id}/products")]
pub async fn list_products(path: web::Path<i32>, data: web::Data<AppState>) -> impl Responder {
    let sess_id = path.into_inner();
    let rows = sqlx::query_as::<_, Product>(
        "SELECT id, session_id, number, name, description FROM products WHERE session_id=$1 ORDER BY number",
    )
    .bind(sess_id)
    .fetch_all(&data.pool)
    .await
    .unwrap();
    HttpResponse::Ok().json(rows)
}

#[derive(Debug, Deserialize)]
pub struct NewCompetencyIn {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[post("/sessions/{sess_id}/competencies")]
pub async fn create_competency(
    path: web::Path<i32>,
    data: web::Data<AppState>,
    body: web::Json<NewCompetencyIn>,
) -> impl Responder {
    let sess_id = path.into_inner();
    let pool = &data.pool;
    let next = sqlx::query_scalar::<_, Option<i32>>(
        "SELECT MAX(number) FROM competencies WHERE session_id=$1",
    )
    .bind(sess_id)
    .fetch_one(pool)
    .await;

    let number = match next {
        Ok(n) => n.unwrap_or(0) + 1,
        Err(e) => {
            return HttpResponse::InternalServerError().body(format!("DB error: {:?}", e));
        }
    };

    let res = sqlx::query_as::<_, Competency>(
        "INSERT INTO competencies (session_id, number, name, description) VALUES ($1,$2,$3,$4)
         RETURNING id, session_id, number, name, description",
    )
    .bind(sess_id)
    .bind(number as i32)
    .bind(&body.name)
    .bind(&body.description)
    .fetch_one(pool)
    .await;

    match res {
        Ok(rec) => HttpResponse::Ok().json(rec),
        Err(e) => HttpResponse::InternalServerError().body(format!("DB error: {:?}", e)),
    }
}

#[get("/sessions/{sess_id}/competencies")]
pub async fn list_competencies(path: web::Path<i32>, data: web::Data<AppState>) -> impl Responder {
    let sess_id = path.into_inner();
    let pool = &data.pool;

    let res = sqlx::query_as::<_, Competency>(
        "SELECT id, session_id, number, name, description FROM competencies WHERE session_id=$1 ORDER BY number"
    )
    .bind(sess_id)
    .fetch_all(pool)
    .await;

    match res {
        Ok(rows) => HttpResponse::Ok().json(rows),
        Err(e) => HttpResponse::InternalServerError().body(format!("DB error: {:?}", e)),
    }
}

#[derive(Deserialize)]
pub struct UpdateCompetencyIn {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[put("/competencies/{competency_id}")]
pub async fn update_competency(
    path: web::Path<i32>,
    data: web::Data<AppState>,
    body: web::Json<UpdateCompetencyIn>,
) -> impl Responder {
    let id = path.into_inner();
    let name = &body.name;
    let desc = &body.description;
    let rec = sqlx::query_as::<_, Competency>(
        "UPDATE competencies SET name=COALESCE($1,name), description=COALESCE($2,description) WHERE id=$3 RETURNING id, session_id, number, name, description"
    )
    .bind(name)
    .bind(desc)
    .bind(id)
    .fetch_one(&data.pool)
    .await
    .unwrap();
    HttpResponse::Ok().json(rec)
}

#[delete("/competencies/{competency_id}")]
pub async fn delete_competency(path: web::Path<i32>, data: web::Data<AppState>) -> impl Responder {
    let id = path.into_inner();
    sqlx::query("DELETE FROM competencies WHERE id=$1")
        .bind(id)
        .execute(&data.pool)
        .await
        .unwrap();
    HttpResponse::NoContent().finish()
}

#[post("/competencies/{comp_id}/abilities")]
pub async fn create_ability(
    path: web::Path<i32>,
    data: web::Data<AppState>,
    body: web::Json<serde_json::Value>,
) -> impl Responder {
    let comp_id = path.into_inner();
    let next = sqlx::query_scalar::<_, Option<i32>>(
        "SELECT MAX(number) FROM abilities WHERE competency_id=$1",
    )
    .bind(comp_id)
    .fetch_one(&data.pool)
    .await;

    let number = match next {
        Ok(n) => n.unwrap_or(0) + 1,
        Err(e) => {
            eprintln!("ERROR SQLX MAX(number): {:?}", e);
            return HttpResponse::InternalServerError().body(format!("DB error: {:?}", e));
        }
    };

    let name = body.get("name").and_then(|v| v.as_str());
    let description = body.get("description").and_then(|v| v.as_str());

    let res = sqlx::query_as::<_, Ability>(
        "INSERT INTO abilities (competency_id, number, name, description) VALUES ($1,$2,$3,$4)
         RETURNING id, competency_id, number, name, description",
    )
    .bind(comp_id)
    .bind(number as i32)
    .bind(name)
    .bind(description)
    .fetch_one(&data.pool)
    .await;

    match res {
        Ok(rec) => HttpResponse::Ok().json(rec),
        Err(e) => {
            eprintln!("ERROR SQLX INSERT: {:?}", e);
            HttpResponse::InternalServerError().body(format!("DB error: {:?}", e))
        }
    }
}

#[get("/competencies/{comp_id}/abilities")]
pub async fn list_abilities(path: web::Path<i32>, data: web::Data<AppState>) -> impl Responder {
    let comp_id = path.into_inner();
    let rows = sqlx::query_as::<_, Ability>(
        "SELECT id, competency_id, number, name, description FROM abilities WHERE competency_id=$1 ORDER BY number",
    )
    .bind(comp_id)
    .fetch_all(&data.pool)
    .await
    .unwrap();
    HttpResponse::Ok().json(rows)
}

#[derive(Deserialize)]
pub struct UpdateAbilityIn {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[put("/abilities/{ability_id}")]
pub async fn update_ability(
    path: web::Path<i32>,
    data: web::Data<AppState>,
    body: web::Json<UpdateAbilityIn>,
) -> impl Responder {
    let id = path.into_inner();
    let name = &body.name;
    let desc = &body.description;
    let rec = sqlx::query_as::<_, Ability>(
        "UPDATE abilities SET name=COALESCE($1,name), description=COALESCE($2,description) WHERE id=$3 RETURNING id, competency_id, number, name, description"
    )
    .bind(name)
    .bind(desc)
    .bind(id)
    .fetch_one(&data.pool)
    .await
    .unwrap();
    HttpResponse::Ok().json(rec)
}

#[delete("/abilities/{ability_id}")]
pub async fn delete_ability(path: web::Path<i32>, data: web::Data<AppState>) -> impl Responder {
    let id = path.into_inner();
    sqlx::query("DELETE FROM abilities WHERE id=$1")
        .bind(id)
        .execute(&data.pool)
        .await
        .unwrap();
    HttpResponse::NoContent().finish()
}

#[post("/abilities/{ability_id}/criteria")]
pub async fn create_criterion_new(
    path: web::Path<i32>,
    data: web::Data<AppState>,
    body: web::Json<serde_json::Value>,
) -> impl Responder {
    let ability_id = path.into_inner();
    let next = sqlx::query_scalar::<_, Option<i32>>(
        "SELECT MAX(number) FROM criteria WHERE ability_id=$1",
    )
    .bind(ability_id)
    .fetch_one(&data.pool)
    .await
    .unwrap();
    let number = next.unwrap_or(0) + 1;
    let name = body.get("name").and_then(|v| v.as_str());
    let description = body.get("description").and_then(|v| v.as_str());
    let rec = sqlx::query_as::<_, Criterion>(
        "INSERT INTO criteria (ability_id, number, name, description) VALUES ($1,$2,$3,$4)
         RETURNING id, ability_id, number, name, description",
    )
    .bind(ability_id)
    .bind(number as i32)
    .bind(name)
    .bind(description)
    .fetch_one(&data.pool)
    .await
    .unwrap();
    HttpResponse::Ok().json(rec)
}

#[get("/abilities/{ability_id}/criteria")]
pub async fn list_criteria_new(path: web::Path<i32>, data: web::Data<AppState>) -> impl Responder {
    let ability_id = path.into_inner();
    let rows = sqlx::query_as::<_, Criterion>(
        "SELECT id, ability_id, number, name, description FROM criteria WHERE ability_id=$1 ORDER BY number"
    )
    .bind(ability_id)
    .fetch_all(&data.pool)
    .await
    .unwrap();
    HttpResponse::Ok().json(rows)
}

#[derive(Deserialize)]
pub struct UpdateCriterionIn {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[put("/criteria/{criterion_id}")]
pub async fn update_criterion(
    path: web::Path<i32>,
    data: web::Data<AppState>,
    body: web::Json<UpdateCriterionIn>,
) -> impl Responder {
    let id = path.into_inner();
    let name = &body.name;
    let desc = &body.description;
    let rec = sqlx::query_as::<_, Criterion>(
        "UPDATE criteria SET name=COALESCE($1,name), description=COALESCE($2,description) WHERE id=$3 RETURNING id, ability_id, number, name, description"
    )
    .bind(name)
    .bind(desc)
    .bind(id)
    .fetch_one(&data.pool)
    .await
    .unwrap();
    HttpResponse::Ok().json(rec)
}

#[delete("/criteria/{criterion_id}")]
pub async fn delete_criterion(path: web::Path<i32>, data: web::Data<AppState>) -> impl Responder {
    let id = path.into_inner();
    sqlx::query("DELETE FROM criteria WHERE id=$1")
        .bind(id)
        .execute(&data.pool)
        .await
        .unwrap();
    HttpResponse::NoContent().finish()
}

#[put("/evaluation/value")]
pub async fn upsert_eval_new(
    data: web::Data<AppState>,
    body: web::Json<EvalValueIn>,
) -> impl Responder {
    // validar lock
    let lock_exists = sqlx::query_scalar::<_, Option<i64>>(
        "SELECT 1 FROM evaluation_locks WHERE session_id=$1 AND competency_id=$2",
    )
    .bind(body.session_id)
    .bind(body.competency_id)
    .fetch_optional(&data.pool)
    .await
    .unwrap();
    if lock_exists.is_some() {
        return HttpResponse::Forbidden().body("Locked");
    }
    let rec = sqlx::query(
        r#"INSERT INTO evaluation_items 
           (session_id, competency_id, ability_id, criterion_id, product_id, student_id, value, observation)
        VALUES ($1,$2,$3,$4,$5,$6,$7::eval_level,$8)
        ON CONFLICT (session_id, competency_id, ability_id, criterion_id, product_id, student_id)
        DO UPDATE SET value=EXCLUDED.value, observation=EXCLUDED.observation, updated_at=NOW()
        RETURNING id"#,
    )
    .bind(body.session_id)
    .bind(body.competency_id)
    .bind(body.ability_id)
    .bind(body.criterion_id)
    .bind(body.product_id)
    .bind(body.student_id)
    .bind(&body.value)
    .bind(&body.observation) // <--- OBSERVATION
    .fetch_one(&data.pool)
    .await
    .unwrap();
    HttpResponse::Ok().json(serde_json::json!({"id": rec.try_get::<i32, _>("id").unwrap() }))
}

#[get("/evaluation/item")]
pub async fn get_evaluation_item(
    query: web::Query<EvalValueIn>, // o un struct similar con las claves únicas
    data: web::Data<AppState>,
) -> impl Responder {
    let rec = sqlx::query_as::<_, EvaluationItem>(
        "SELECT id, session_id, competency_id, ability_id, criterion_id, product_id, student_id, value, updated_at, observation
         FROM evaluation_items
         WHERE session_id=$1 AND competency_id=$2 AND ability_id=$3 AND criterion_id=$4 AND product_id=$5 AND student_id=$6"
    )
    .bind(query.session_id)
    .bind(query.competency_id)
    .bind(query.ability_id)
    .bind(query.criterion_id)
    .bind(query.product_id)
    .bind(query.student_id)
    .fetch_optional(&data.pool)
    .await
    .unwrap();

    match rec {
        Some(item) => HttpResponse::Ok().json(item),
        None => {
            HttpResponse::NotFound().body("No existe evaluación para ese criterio y estudiante.")
        }
    }
}

#[delete("/evaluation/item")]
pub async fn delete_evaluation_item(
    query: web::Query<EvalValueIn>, // puedes usar también un struct solo con las claves necesarias
    data: web::Data<AppState>,
) -> impl Responder {
    let result = sqlx::query(
        "DELETE FROM evaluation_items
         WHERE session_id=$1 AND competency_id=$2 AND ability_id=$3 AND criterion_id=$4 AND product_id=$5 AND student_id=$6"
    )
    .bind(query.session_id)
    .bind(query.competency_id)
    .bind(query.ability_id)
    .bind(query.criterion_id)
    .bind(query.product_id)
    .bind(query.student_id)
    .execute(&data.pool)
    .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => HttpResponse::NoContent().finish(),
        Ok(_) => {
            HttpResponse::NotFound().body("No existe evaluación para ese criterio y estudiante.")
        }
        Err(e) => {
            eprintln!("Error al borrar evaluación: {:?}", e);
            HttpResponse::InternalServerError().body("Error en la base de datos")
        }
    }
}

#[get("/sessions/{sess_id}/products/{prod_id}/competencies/{comp_id}/matrix")]
pub async fn get_matrix_new(
    path: web::Path<(i32, i32, i32)>,
    data: web::Data<AppState>,
) -> impl Responder {
    let (sess_id, prod_id, comp_id) = path.into_inner();
    let locked = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM evaluation_locks WHERE session_id=$1 AND competency_id=$2)",
    )
    .bind(sess_id)
    .bind(comp_id)
    .fetch_one(&data.pool)
    .await
    .unwrap();

    let comp_row = sqlx::query(
        "SELECT id, number, COALESCE(name, 'Competencia '||number::text) AS display_name
         FROM competencies WHERE id=$1",
    )
    .bind(comp_id)
    .fetch_one(&data.pool)
    .await
    .unwrap();

    let abilities = sqlx::query(
        "SELECT id, number, COALESCE(name, 'Capacidad '||number::text) AS display_name
         FROM abilities WHERE competency_id=$1 ORDER BY number",
    )
    .bind(comp_id)
    .fetch_all(&data.pool)
    .await
    .unwrap();

    let criteria = sqlx::query(
        "SELECT id, ability_id, number, COALESCE(name, 'C'||number::text) AS display_name
         FROM criteria WHERE ability_id IN (
            SELECT id FROM abilities WHERE competency_id=$1
         ) ORDER BY ability_id, number",
    )
    .bind(comp_id)
    .fetch_all(&data.pool)
    .await
    .unwrap();

    let product_row = sqlx::query("SELECT id, name, description FROM products WHERE id=$1")
        .bind(prod_id)
        .fetch_one(&data.pool)
        .await
        .unwrap();

    let students = sqlx::query(
        "SELECT st.id, st.full_name
           FROM sessions s
           JOIN sections sec ON sec.id = s.section_id
           JOIN students st ON st.section_id = sec.id
          WHERE s.id=$1
          ORDER BY st.full_name",
    )
    .bind(sess_id)
    .fetch_all(&data.pool)
    .await
    .unwrap();

    let values = sqlx::query(
        "SELECT student_id, ability_id, criterion_id, value::text AS value, observation
        FROM evaluation_items
        WHERE session_id=$1 AND competency_id=$2 AND product_id=$3",
    )
    .bind(sess_id)
    .bind(comp_id)
    .bind(prod_id)
    .fetch_all(&data.pool)
    .await
    .unwrap();

    let resp = MatrixResponse {
        locked,
        competency: serde_json::json!({
            "id": comp_row.try_get::<i32, _>("id").unwrap(),
            "display_name": comp_row.try_get::<String, _>("display_name").unwrap()
        }),
        abilities: abilities
            .into_iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.try_get::<i32, _>("id").unwrap(),
                    "display_name": r.try_get::<String, _>("display_name").unwrap()
                })
            })
            .collect(),
        criteria: criteria
            .into_iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.try_get::<i32, _>("id").unwrap(),
                    "ability_id": r.try_get::<i32, _>("ability_id").unwrap(),
                    "display_name": r.try_get::<String, _>("display_name").unwrap()
                })
            })
            .collect(),
        products: vec![serde_json::json!({
            "id": product_row.try_get::<i32, _>("id").unwrap(),
            "name": product_row.try_get::<String, _>("name").unwrap(),
            "description": product_row.try_get::<Option<String>, _>("description").unwrap()
        })],
        students: students
            .into_iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.try_get::<i32, _>("id").unwrap(),
                    "full_name": r.try_get::<String, _>("full_name").unwrap()
                })
            })
            .collect(),
        values: values
            .into_iter()
            .map(|r| {
                serde_json::json!({
                    "student_id": r.try_get::<i32, _>("student_id").unwrap(),
                    "ability_id": r.try_get::<i32, _>("ability_id").unwrap(),
                    "criterion_id": r.try_get::<i32, _>("criterion_id").unwrap(),
                    "value": r.try_get::<String, _>("value").unwrap(),
                    "observation": r.try_get::<Option<String>, _>("observation").unwrap()
                })
            })
            .collect(),
    };

    HttpResponse::Ok().json(resp)
}

use serde::Serialize;

#[derive(Serialize)]
pub struct EvaluationContextResponse {
    pub locked: bool,
    pub competency: serde_json::Value,
    pub product: serde_json::Value,
    pub abilities: Vec<serde_json::Value>,
    pub criteria: Vec<serde_json::Value>,
    pub students: Vec<serde_json::Value>,
    pub values: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
pub struct EvaluationContextParams {
    pub session_id: i32,
    pub competency_id: i32,
    pub product_id: i32,
}

#[get("/evaluation/context")]
pub async fn evaluation_context(
    params: web::Query<EvaluationContextParams>,
    data: web::Data<AppState>,
) -> impl Responder {
    let session_id = params.session_id;
    let competency_id = params.competency_id;
    let product_id = params.product_id;

    // Ver si está bloqueada la competencia
    let locked = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM evaluation_locks WHERE session_id=$1 AND competency_id=$2)",
    )
    .bind(session_id)
    .bind(competency_id)
    .fetch_one(&data.pool)
    .await
    .unwrap_or(false);

    let competency = sqlx::query(
        "SELECT id, number, COALESCE(name, 'Competencia '||number::text) AS display_name
         FROM competencies WHERE id=$1",
    )
    .bind(competency_id)
    .fetch_one(&data.pool)
    .await
    .unwrap();

    let product = sqlx::query("SELECT id, name, description FROM products WHERE id=$1")
        .bind(product_id)
        .fetch_one(&data.pool)
        .await
        .unwrap();

    let abilities = match sqlx::query(
        "SELECT id, number, COALESCE(name, 'Capacidad '||number::text) AS display_name
        FROM abilities WHERE competency_id=$1 ORDER BY number",
    )
    .bind(competency_id)
    .fetch_all(&data.pool)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("Error fetching abilities: {:?}", e);
            return HttpResponse::InternalServerError().body("Error en la base de datos");
        }
    };

    let criteria = sqlx::query(
        "SELECT id, ability_id, number, COALESCE(name, 'C'||number::text) AS display_name
         FROM criteria WHERE ability_id IN (
            SELECT id FROM abilities WHERE competency_id=$1
         ) ORDER BY ability_id, number",
    )
    .bind(competency_id)
    .fetch_all(&data.pool)
    .await
    .unwrap();

    let students = sqlx::query(
        "SELECT st.id, st.full_name
           FROM sessions s
           JOIN sections sec ON sec.id = s.section_id
           JOIN students st ON st.section_id = sec.id
          WHERE s.id=$1
          ORDER BY st.full_name",
    )
    .bind(session_id)
    .fetch_all(&data.pool)
    .await
    .unwrap();

    let values = sqlx::query(
        "SELECT student_id, ability_id, criterion_id, value::text AS value, observation
         FROM evaluation_items
         WHERE session_id=$1 AND competency_id=$2 AND product_id=$3",
    )
    .bind(session_id)
    .bind(competency_id)
    .bind(product_id)
    .fetch_all(&data.pool)
    .await
    .unwrap();

    let resp = EvaluationContextResponse {
        locked,
        competency: serde_json::json!({
            "id": competency.try_get::<i32, _>("id").unwrap(),
            "display_name": competency.try_get::<String, _>("display_name").unwrap()
        }),
        product: serde_json::json!({
            "id": product.try_get::<i32, _>("id").unwrap(),
            "name": product.try_get::<String, _>("name").unwrap(),
            "description": product.try_get::<Option<String>, _>("description").unwrap()
        }),
        abilities: abilities
            .into_iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.try_get::<i32, _>("id").unwrap(),
                    "display_name": r.try_get::<String, _>("display_name").unwrap()
                })
            })
            .collect(),
        criteria: criteria
            .into_iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.try_get::<i32, _>("id").unwrap(),
                    "ability_id": r.try_get::<i32, _>("ability_id").unwrap(),
                    "display_name": r.try_get::<String, _>("display_name").unwrap()
                })
            })
            .collect(),
        students: students
            .into_iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.try_get::<i32, _>("id").unwrap(),
                    "full_name": r.try_get::<String, _>("full_name").unwrap()
                })
            })
            .collect(),
        values: values
            .into_iter()
            .map(|r| {
                serde_json::json!({
                    "student_id": r.try_get::<i32, _>("student_id").unwrap(),
                    "ability_id": r.try_get::<i32, _>("ability_id").unwrap(),
                    "criterion_id": r.try_get::<i32, _>("criterion_id").unwrap(),
                    "value": r.try_get::<String, _>("value").unwrap(),
                    "observation": r.try_get::<Option<String>, _>("observation").unwrap()
                })
            })
            .collect(),
    };

    HttpResponse::Ok().json(resp)
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

#[get("/abilities/{ability_id}")]
pub async fn get_ability(path: web::Path<i32>, data: web::Data<AppState>) -> impl Responder {
    let id = path.into_inner();
    let rec = sqlx::query_as::<_, Ability>(
        "SELECT id, competency_id, number, name, description FROM abilities WHERE id=$1",
    )
    .bind(id)
    .fetch_optional(&data.pool)
    .await
    .unwrap();

    match rec {
        Some(ability) => HttpResponse::Ok().json(ability),
        None => HttpResponse::NotFound().body("Ability not found"),
    }
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

// ============================================
// ENDPOINTS DE AUTENTICACIÓN
// ============================================

use crate::models::{
    ApiResponse, ErrorResponse, GuardianProfile, RegisterAlumnoRequest, RegisterApoderadoRequest,
    RegisterDocenteRequest, StudentProfile, TeacherProfile, User, UserResponse,
};

/// POST /api/auth/register/alumno
#[post("/api/auth/register/alumno")]
pub async fn register_alumno(
    data: web::Data<AppState>,
    body: web::Json<RegisterAlumnoRequest>,
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
            eprintln!("Error iniciando transacción: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Error de base de datos".to_string(),
                details: Some(e.to_string()),
            });
        }
    };

    // 1. Crear usuario con ENUM
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

    // 2. Crear perfil de alumno
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
        Ok(p) => p,
        Err(e) => {
            let _ = tx.rollback().await;
            eprintln!("Error creando perfil de alumno: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Error creando perfil de alumno".to_string(),
                details: Some(e.to_string()),
            });
        }
    };

    // ✅ 3. NUEVO: Intentar vincular automáticamente por nombre
    let linked_student_id =
        match try_link_student_by_name(&data.pool, user.id, &body.full_name, &body.dni).await {
            Ok(Some(student_id)) => {
                tracing::info!(
                    "✅ Vinculación automática exitosa: user_id={}, student_id={}",
                    user.id,
                    student_id
                );
                Some(student_id)
            }
            Ok(None) => {
                tracing::warn!(
                    "⚠️ No se pudo vincular automáticamente al alumno: {}",
                    body.full_name
                );
                None
            }
            Err(e) => {
                tracing::error!("❌ Error en vinculación automática: {:?}", e);
                // No hacer rollback, solo loguear el error
                None
            }
        };

    if let Err(e) = tx.commit().await {
        eprintln!("Error confirmando transacción: {:?}", e);
        return HttpResponse::InternalServerError().json(ErrorResponse {
            error: "Error confirmando registro".to_string(),
            details: Some(e.to_string()),
        });
    }

    // Preparar respuesta con información de vinculación
    let mut profile_data = serde_json::json!({
        "dni": student_profile.dni,
        "full_name": student_profile.full_name,
        "enrollment_date": student_profile.enrollment_date,
    });

    if let Some(student_id) = linked_student_id {
        profile_data["linked_student_id"] = serde_json::json!(student_id);
        profile_data["auto_linked"] = serde_json::json!(true);
    } else {
        profile_data["auto_linked"] = serde_json::json!(false);
    }

    HttpResponse::Created().json(ApiResponse {
        success: true,
        message: if linked_student_id.is_some() {
            "Alumno registrado y vinculado automáticamente".to_string()
        } else {
            "Alumno registrado exitosamente".to_string()
        },
        data: Some(UserResponse {
            id: user.id,
            email: user.email.clone(),
            role: user.role.to_string(),
            status: user.status.to_string(),
            profile_data,
        }),
    })
}

/// POST /api/auth/register/apoderado
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

/// POST /api/auth/register/docente
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

/// GET /api/auth/me - Obtener usuario actual
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

#[get("/students/{user_id}/grades")]
pub async fn get_student_grades(path: web::Path<i32>, data: web::Data<AppState>) -> impl Responder {
    let user_id = path.into_inner();

    let rows = sqlx::query(
        r#"
        SELECT 
            s.full_name,
            sec.letter,
            g.number AS grade_number,
            b.name AS bimester_name,
            sess.title AS session_title,
            comp.name AS competency_name,
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
        WHERE s.user_id = $1
        ORDER BY b.year DESC, b.id DESC, sess.number, comp.number
        "#,
    )
    .bind(user_id)
    .fetch_all(&data.pool)
    .await;

    match rows {
        Ok(data) => {
            let grades: Vec<StudentGradeItem> = data
                .into_iter()
                .map(|row| StudentGradeItem {
                    full_name: row.try_get("full_name").unwrap(),
                    section_letter: row.try_get("letter").unwrap(),
                    grade_number: row.try_get("grade_number").unwrap(),
                    bimester_name: row.try_get("bimester_name").unwrap(),
                    session_title: row.try_get("session_title").unwrap(),
                    competency_name: row.try_get("competency_name").unwrap(),
                    value: row.try_get("value").unwrap(),
                    observation: row.try_get("observation").unwrap(),
                    updated_at: row.try_get("updated_at").unwrap(),
                })
                .collect();

            HttpResponse::Ok().json(grades)
        }
        Err(e) => {
            eprintln!("Error fetching grades: {:?}", e);
            HttpResponse::InternalServerError().body("Error al obtener notas")
        }
    }
}

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

/// POST /admin/unlink-student - Desvincular estudiante
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

/// POST /admin/link-student-by-dni - Vincular por DNI
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

/// GET /admin/linking-status - Estado de vinculaciones
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

/// POST /admin/backfill-dni - Ejecutar backfill de DNIs
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

fn normalize_name(name: &str) -> String {
    name.to_uppercase()
        .replace(",", "")
        .split_whitespace()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Intenta vincular automáticamente un estudiante por similitud de nombre
async fn try_link_student_by_name(
    pool: &PgPool,
    user_id: i32,
    full_name: &str,
    dni: &str,
) -> Result<Option<i32>, sqlx::Error> {
    let normalized_name = normalize_name(full_name);

    tracing::info!(
        "🔍 Buscando estudiante con nombre similar a: '{}' (normalizado: '{}')",
        full_name,
        normalized_name
    );

    // ✅ CORRECCIÓN: Usar REGEXP_REPLACE y comparar nombres normalizados
    let student = sqlx::query(
        r#"
        SELECT id, full_name, section_id
        FROM students
        WHERE user_id IS NULL
          AND UPPER(
              REGEXP_REPLACE(
                  REPLACE(full_name, ',', ''),
                  '\s+', ' ', 'g'
              )
          ) = UPPER(
              REGEXP_REPLACE(
                  REPLACE($1, ',', ''),
                  '\s+', ' ', 'g'
              )
          )
        LIMIT 1
        "#,
    )
    .bind(full_name)  // ✅ CORRECCIÓN: Pasar el nombre ORIGINAL, no normalizado
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

        tracing::info!(
            "✅ Estudiante {} vinculado exitosamente: user_id={}, dni={}",
            student_full_name,
            user_id,
            dni
        );

        Ok(Some(student_id))
    } else {
        tracing::warn!(
            "⚠️ No se encontró estudiante sin vincular con nombre: '{}' (normalizado: '{}')",
            full_name,
            normalized_name
        );
        Ok(None)
    }
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(create_bimester)
        .service(list_bimesters)
        .service(create_grade)
        .service(list_grades)
        .service(create_section)
        .service(list_sections)
        .service(create_student)
        .service(list_students)
        .service(update_student)
        .service(delete_student)
        .service(create_session)
        .service(list_sessions)
        .service(delete_grade)
        .service(get_grade)
        .service(delete_section)
        .service(get_section)
        .service(delete_session)
        .service(get_session)
        .service(create_product)
        .service(list_products)
        .service(create_ability)
        .service(list_abilities)
        .service(create_criterion_new)
        .service(list_criteria_new)
        .service(upsert_eval_new)
        .service(get_matrix_new)
        .service(update_session)
        .service(delete_product)
        .service(update_product)
        .service(create_competency)
        .service(list_competencies)
        .service(update_competency)
        .service(delete_competency)
        .service(update_ability)
        .service(delete_ability)
        .service(update_criterion)
        .service(delete_criterion)
        .service(get_evaluation_item)
        .service(delete_evaluation_item)
        .service(evaluation_context)
        .service(get_consolidado_section)
        .service(get_ability)
        .service(list_bimesters_full)
        .service(import_students_json)
        .service(import_students_csv)
        .service(import_students_txt)
        .service(register_alumno)
        .service(register_apoderado)
        .service(register_docente)
        .service(get_current_user)
        .service(get_student_grades)
        .service(get_student_profile)
        .service(get_student_enrollments)
        .service(link_student_to_user)
        .service(list_unlinked_students)
        .service(search_students)
        .service(detect_homonyms)
        .service(unlink_student)
        .service(link_student_by_dni)
        .service(get_linking_status)
        .service(backfill_dni);
}
