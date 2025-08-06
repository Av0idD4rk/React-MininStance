use actix_web::{web, HttpResponse, Responder, ResponseError};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use thiserror::Error;

use auth_captcha::CaptchaVerifier;
use common::ServiceError;
use config_manager::get_config;
use data_models::Db;
use deploy_service::Deployer;
use common::TaskInstance;

// ----- Error type for the API -----

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("captcha failed")]
    Captcha,

    #[error("deploy service error: {0}")]
    Deploy(#[from] deploy_service::error::DeployError),

    #[error("db error: {0}")]
    Db(#[from] common::ServiceError),

    #[error("bad request: {0}")]
    BadRequest(String),
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        match self {
            ApiError::Captcha => HttpResponse::Unauthorized().json("Invalid captcha"),
            ApiError::Deploy(e) => HttpResponse::InternalServerError().json(e.to_string()),
            ApiError::Db(e) => HttpResponse::InternalServerError().json(e.to_string()),
            ApiError::BadRequest(msg) => HttpResponse::BadRequest().json(msg.clone()),
        }
    }
}

// ----- Request/Response DTOs -----

#[derive(Deserialize)]
pub struct DeployReq {
    task: String,
    captcha_token: String,
}

#[derive(Deserialize)]
pub struct ActionReq {
    instance_id: i32,
}

#[derive(Serialize)]
pub struct InstanceListItem {
    id: i32,
    task_name: String,
    port: u16,
    expires_in_secs: u64,
    status: String,
}

// ----- Handlers -----

/// POST /deploy
pub async fn deploy(
    body: web::Json<DeployReq>,
    deployer: web::Data<Mutex<Deployer>>,
) -> Result<impl Responder, ApiError> {
    // 1. Verify captcha
  /*  let verifier = CaptchaVerifier::new();
    verifier
        .verify(&body.captcha_token)
        .await
        .map_err(|_| ApiError::Captcha)?;*/

    // 2. Deploy
    let mut d = deployer.lock().unwrap();
    let inst = d.deploy(&body.task).await?;

    // 3. Return the full TaskInstance
    Ok(HttpResponse::Ok().json(inst))
}

/// POST /stop
pub async fn stop(
    body: web::Json<ActionReq>,
    deployer: web::Data<Mutex<Deployer>>,
) -> Result<impl Responder, ApiError> {
    // Lookup in DB to get container_id + port
    let db = Db::new()?;
    let inst = db
        .find_instance_by_id(body.instance_id)?
        .ok_or_else(|| ApiError::BadRequest("instance not found".into()))?;

    // Stop
    let mut d = deployer.lock().unwrap();
    d.stop(&inst).await?;
    Ok(HttpResponse::Ok().finish())
}

/// POST /restart
pub async fn restart(
    body: web::Json<ActionReq>,
    deployer: web::Data<Mutex<Deployer>>,
) -> Result<impl Responder, ApiError> {
    let db = Db::new()?;
    let inst = db
        .find_instance_by_id(body.instance_id)?
        .ok_or_else(|| ApiError::BadRequest("instance not found".into()))?;
    let mut d = deployer.lock().unwrap();
    d.restart(&inst).await?;
    Ok(HttpResponse::Ok().finish())
}

/// POST /extend
pub async fn extend(
    body: web::Json<ActionReq>,
    deployer: web::Data<Mutex<Deployer>>,
) -> Result<impl Responder, ApiError> {
    let cfg = get_config();
    let db = Db::new()?;
    let inst = db
        .find_instance_by_id(body.instance_id)?
        .ok_or_else(|| ApiError::BadRequest("instance not found".into()))?;
    let mut d = deployer.lock().unwrap();
    d.extend(&inst, cfg.ports.extend_time_secs).await?;
    Ok(HttpResponse::Ok().finish())
}

pub async fn list_instances() -> Result<impl Responder, ApiError> {
    let db = Db::new()?;
    let rows = db.list_instances()?;
    let now = chrono::Utc::now();
    let items: Vec<InstanceListItem> = rows
        .into_iter()
        .map(|i| InstanceListItem {
            id: i.id,
            task_name: i.task_name,
            port: i.port,
            expires_in_secs: i.expires_at.signed_duration_since(now).num_seconds().max(0) as u64,
            status: format!("{:?}", i.status),
        })
        .collect();
    Ok(HttpResponse::Ok().json(items))
}


pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/deploy", web::post().to(deploy))
        .route("/stop",   web::post().to(stop))
        .route("/restart",web::post().to(restart))
        .route("/extend", web::post().to(extend))
        .route("/instances", web::get().to(list_instances));
}
