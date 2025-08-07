use crate::auth::AuthUser;
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, Responder, ResponseError, web};
use chrono::{Duration, Utc};
use common::TaskInstance;
use config_manager::get_config;
use data_models::Db;
use deploy_service::Deployer;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use thiserror::Error;
use uuid::Uuid;

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

impl ApiError {
    fn forbidden(msg: &str) -> actix_web::Error {
        actix_web::error::InternalError::new(msg.to_string(), StatusCode::FORBIDDEN).into()
    }
}

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
    expires_in_secs: u64,
    endpoint: String,
    status: String,
}

#[derive(Serialize)]
pub struct DeployResp {
    pub instance: TaskInstance,
}

pub async fn deploy(
    auth: AuthUser,
    body: web::Json<DeployReq>,
    deployer: web::Data<Mutex<Deployer>>,
) -> Result<impl Responder, ApiError> {
    let cfg = get_config();
    let user_id = auth.0.id;
    let db = Db::new()?;
    let running = db.count_running_instances_for_user(user_id)?;
    if running >= cfg.sessions.clone().max_instances.into() {
        return Err(ApiError::BadRequest("instance limit reached".into()));
    }

    let mut d = deployer.lock().unwrap();
    let dr = d.deploy(&body.task).await?;

    // persist under user:
    let saved = db.create_instance_for_user(&dr.instance, auth.0.id)?;
    Ok(HttpResponse::Ok().json(DeployResp { instance: saved }))
}

pub async fn stop(
    auth: AuthUser,
    body: web::Json<ActionReq>,
    deployer: web::Data<Mutex<Deployer>>,
) -> Result<impl Responder, actix_web::Error> {
    let db = Db::new().map_err(ApiError::Db)?;
    let inst = db
        .find_instance_by_id(body.instance_id)
        .map_err(ApiError::Db)?
        .ok_or_else(|| ApiError::BadRequest("Instance not found".into()))?;

    if inst.user_id != auth.0.id {
        return Err(ApiError::forbidden("Not your instance"));
    }

    let mut d = deployer.lock().unwrap();
    d.stop(&inst).await.map_err(ApiError::Deploy)?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn restart(
    auth: AuthUser,
    body: web::Json<ActionReq>,
    deployer: web::Data<Mutex<Deployer>>,
) -> Result<impl Responder, actix_web::Error> {
    let db = Db::new().map_err(ApiError::Db)?;
    let inst = db
        .find_instance_by_id(body.instance_id)
        .map_err(ApiError::Db)?
        .ok_or_else(|| ApiError::BadRequest("Instance not found".into()))?;

    if inst.user_id != auth.0.id {
        return Err(ApiError::forbidden("Not your instance"));
    }

    let mut d = deployer.lock().unwrap();
    d.restart(&inst).await.map_err(ApiError::Deploy)?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn extend(
    auth: AuthUser,
    body: web::Json<ActionReq>,
    deployer: web::Data<Mutex<Deployer>>,
) -> Result<impl Responder, actix_web::Error> {
    let db = Db::new().map_err(ApiError::Db)?;
    let inst = db
        .find_instance_by_id(body.instance_id)
        .map_err(ApiError::Db)?
        .ok_or_else(|| ApiError::BadRequest("Instance not found".into()))?;

    if inst.user_id != auth.0.id {
        return Err(ApiError::forbidden("Not your instance"));
    }

    let mut d = deployer.lock().unwrap();
    let ttl = get_config().ports.default_ttl_secs;
    d.extend(&inst, ttl).await.map_err(ApiError::Deploy)?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn list_instances(auth: AuthUser) -> Result<impl Responder, actix_web::Error> {
    let db = Db::new().map_err(ApiError::Db)?;
    let rows = db
        .list_instances_for_user(auth.0.id)
        .map_err(ApiError::Db)?;
    let now = chrono::Utc::now();
    let items: Vec<InstanceListItem> = rows
        .into_iter()
        .map(|i| InstanceListItem {
            id: i.id,
            task_name: i.task_name,
            expires_in_secs: i.expires_at.signed_duration_since(now).num_seconds().max(0) as u64,
            endpoint: i.endpoint,
            status: format!("{:?}", i.status),
        })
        .collect();
    Ok(HttpResponse::Ok().json(items))
}

#[derive(Deserialize)]
struct TokenReq {
    username: String,
}

#[derive(Serialize)]
struct TokenResp {
    token: String,
    expires_at: i64,
}

pub async fn token(body: web::Json<TokenReq>) -> Result<impl Responder, ApiError> {
    let db = Db::new()?;

    let user = db.find_or_create_user(&body.username)?;

    if let Some(existing_token) = db.find_valid_session_for_user(user.id)? {
        if let Some(sess) = db.get_session(&existing_token)? {
            return Ok(HttpResponse::Ok().json(TokenResp {
                token: existing_token,
                expires_at: sess.expires_at.timestamp(),
            }));
        }
    }

    let cfg = get_config();
    let ttl_hours = cfg.sessions.clone().ttl_hours;
    let expires = Utc::now() + Duration::hours(ttl_hours.into());

    let new_token = Uuid::new_v4().to_string();
    db.create_session(&new_token, user.id, expires)?;

    Ok(HttpResponse::Ok().json(TokenResp {
        token: new_token,
        expires_at: expires.timestamp(),
    }))
}

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/token", web::post().to(token))
        .route("/deploy", web::post().to(deploy))
        .route("/stop", web::post().to(stop))
        .route("/restart", web::post().to(restart))
        .route("/extend", web::post().to(extend))
        .route("/instances", web::get().to(list_instances))
        .route("/tasks", web::get().to(list_tasks));
}

#[derive(Serialize)]
pub struct TaskInfo {
    pub name: String,
    pub protocol: String,
    pub container_port: u16,
}

pub async fn list_tasks() -> Result<impl Responder, ApiError> {
    let cfg = get_config();
    let tasks: Vec<TaskInfo> = cfg
        .tasks
        .iter()
        .filter(|(name, _)| name.as_str() != "_default")
        .map(|(name, tc)| TaskInfo {
            name: name.clone(),
            protocol: tc.protocol.clone(),
            container_port: tc.container_port,
        })
        .collect();
    Ok(HttpResponse::Ok().json(tasks))
}

