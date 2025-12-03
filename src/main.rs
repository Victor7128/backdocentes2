mod auth;
mod basic;
mod links;
mod models;
use crate::models::AppState;

use actix_cors::Cors;
use actix_web::{http, web};
use shuttle_actix_web::ShuttleActixWeb;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::env;

#[shuttle_runtime::main]
async fn actix_web(
) -> ShuttleActixWeb<impl FnOnce(&mut web::ServiceConfig) + Send + Clone + 'static> {
    let db_url = String::from("postgres://avnadmin:AVNS__WBhLn_dkf1AWfqU2pu@pg-a675765-vtuesta13-92c1.b.aivencloud.com:11427/defaultdb?sslmode=require");

    let pool: PgPool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("No se pudo conectar a la base de datos");

    let state = AppState { pool };

    let app = move |cfg: &mut web::ServiceConfig| {
        let cors = Cors::default()
            .allow_any_origin()
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
            .allowed_headers(vec![
                http::header::CONTENT_TYPE,
                http::header::AUTHORIZATION,
                http::header::ACCEPT,
                http::header::HeaderName::from_static("x-firebase-uid"),
            ])
            .max_age(3600);

        cfg.service(
            web::scope("")
                .wrap(cors)
                .app_data(web::Data::new(state.clone()))
                .configure(auth::routes::config)
                .configure(links::routes::config)
                .configure(basic::routes::config)
                .configure(basic::students::routes::config)
                .configure(basic::session::routes::config)
                .configure(basic::session::products::routes::config)
                .configure(basic::session::evaluation::routes::config)
                .configure(basic::session::competencies::routes::config)
                .configure(basic::session::competencies::abilities::routes::config)
                .configure(basic::session::competencies::abilities::criterion::routes::config)
        );
    };
    Ok(app.into())
}
