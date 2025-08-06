mod handlers;

use actix_web::{App, HttpServer};
use common::init_logging;
use config_manager::get_config;
use std::sync::Mutex;
use deploy_service::Deployer;
use handlers::configure_routes;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    init_logging();

    let cfg = get_config();
    let bind_addr = ("0.0.0.0", 8080);

    let deployer = Deployer::new().await.expect("failed to init deployer");
    let deployer_data = actix_web::web::Data::new(Mutex::new(deployer));

    HttpServer::new(move || {
        App::new()
            .app_data(deployer_data.clone())
            .configure(configure_routes)
    })
        .bind(bind_addr)?
        .run()
        .await
}
