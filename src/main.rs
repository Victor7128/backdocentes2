// main.rs
mod models;
mod routes;

use actix_cors::Cors;
use actix_web::{http, web};
use routes::{config, AppState};
use shuttle_actix_web::ShuttleActixWeb;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::env;

#[shuttle_runtime::main]
async fn actix_web(
) -> ShuttleActixWeb<impl FnOnce(&mut web::ServiceConfig) + Send + Clone + 'static> {
    // Obtiene la cadena de conexión DATABASE_URL del entorno
    let db_url = String::from("postgres://avnadmin:AVNS__WBhLn_dkf1AWfqU2pu@pg-a675765-vtuesta13-92c1.b.aivencloud.com:11427/defaultdb?sslmode=require");

    // Crea el pool de conexiones
    let pool: PgPool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("No se pudo conectar a la base de datos");

    let state = AppState { pool };

    // Construimos el cierre que creará la aplicación
    let app = move |cfg: &mut web::ServiceConfig| {
        // Configuración CORS - Segura para desarrollo
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

        // Creamos la aplicación Actix Web
        cfg.service(
            web::scope("")
                .wrap(cors) // Aplicamos el middleware CORS aquí
                .app_data(web::Data::new(state.clone()))
                .configure(config), // Configuramos nuestras rutas
        );
    };

    Ok(app.into())
}
