use crate::basic::session::models::*;
use crate::AppState;
use actix_web::{delete, get, post, put, web, HttpResponse, Responder};

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

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(create_session)
        .service(list_sessions)
        .service(delete_session)
        .service(get_session)
        .service(update_session);
}
