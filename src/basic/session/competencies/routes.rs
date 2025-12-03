use crate::basic::session::competencies::models::*;
use crate::AppState;
use actix_web::{delete, get, post, put, web, HttpResponse, Responder};

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

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(create_competency)
        .service(list_competencies)
        .service(update_competency)
        .service(delete_competency);
}