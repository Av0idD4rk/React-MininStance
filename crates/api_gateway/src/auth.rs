use actix_web::{dev::Payload, Error as ActixError, FromRequest, HttpRequest};
use futures_util::future::{ready, Ready};
use crate::handlers::ApiError;
use data_models::Db;
use common::User;

pub struct AuthUser(pub User);

impl FromRequest for AuthUser {
    type Error = ActixError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        // 1) Extract Bearer token
        let token_opt = req
            .headers()
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer ").map(str::to_string));

        let token = match token_opt {
            Some(t) => t,
            None => return ready(Err(ApiError::BadRequest("Missing token".into()).into())),
        };

        // 2) Validate via Db
        let db = match Db::new() {
            Ok(db) => db,
            Err(e) => return ready(Err(ApiError::Db(e).into())),
        };

        match db.validate_session(&token) {
            Err(e) => ready(Err(ApiError::Db(e).into())),
            Ok(None) => ready(Err(ApiError::BadRequest("Invalid or expired token".into()).into())),
            Ok(Some(user)) => ready(Ok(AuthUser(user))),
        }
    }
}
