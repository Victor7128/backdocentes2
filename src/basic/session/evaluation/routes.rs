use crate::basic::session::evaluation::models::*;
use crate::AppState;
use actix_web::{delete, get, put, web, HttpResponse, Responder};
use sqlx::Row;

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

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(upsert_eval_new)
        .service(get_matrix_new)
        .service(get_evaluation_item)
        .service(delete_evaluation_item)
        .service(evaluation_context);
}
