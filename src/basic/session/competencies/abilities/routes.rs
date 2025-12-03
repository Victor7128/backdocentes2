use crate::basic::session::competencies::abilities::models::*;
use crate::AppState;
use actix_web::{delete, get, post, put, web, HttpResponse, Responder};

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

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(create_ability)
        .service(list_abilities)
        .service(update_ability)
        .service(delete_ability)
        .service(get_ability);
}