use crate::basic::session::competencies::abilities::criterion::models::*;
use crate::AppState;
use actix_web::{delete, get, post, put, web, HttpResponse, Responder};

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

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(create_criterion_new)
        .service(list_criteria_new)
        .service(update_criterion)
        .service(delete_criterion);
}