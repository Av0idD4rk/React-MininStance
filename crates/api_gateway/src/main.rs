mod handlers;
mod auth;

use actix_web::{App, HttpServer};
use common::init_logging;
use config_manager::get_config;
use std::sync::Mutex;
use actix_cors::Cors;
use data_models::Db;
use deploy_service::Deployer;
use handlers::configure_routes;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    init_logging();

    let cfg = get_config();
    let bind_addr = ("0.0.0.0", 8080);

    let deployer = Deployer::new().await.expect("failed to init deployer");
    let deployer_data = actix_web::web::Data::new(Mutex::new(deployer));
    let db = Db::new().expect("DB init failed");

    for entry in std::fs::read_dir("./tasks")? {
        let name = entry?.file_name().into_string().unwrap();
        let path = format!("./tasks/{}/Dockerfile", name);
        db.ensure_task(&name, &path).expect("failed to seed task");
    }
    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .supports_credentials();

        App::new()
            .wrap(cors)
            .app_data(deployer_data.clone())
            .configure(configure_routes)
    })
        .bind(bind_addr)?
        .run()
        .await
}
