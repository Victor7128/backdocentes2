use crate::basic::session::products::models::*;
use crate::AppState;
use actix_web::{delete, get, post, put, web, HttpResponse, Responder};

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

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(create_product)
        .service(list_products)
        .service(delete_product)
        .service(update_product);
}
