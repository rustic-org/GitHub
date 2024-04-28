use std::sync;
use actix_web::{HttpRequest, web};
use crate::squire;

/// Struct for the authentication response.
pub struct AuthResponse {
    pub ok: bool,
    pub repository: String,
    pub branch: String,
}

/// Verifies the token received against the one set in env vars.
///
/// * `request` - A reference to the Actix web `HttpRequest` object.
/// * `config` - Configuration data for the application.
///
/// # Returns
///
/// A configured `AuthResponse` instance.
pub fn verify_token(request: &HttpRequest,
                    config: &web::Data<sync::Arc<squire::settings::Config>>) -> AuthResponse {
    let headers = request.headers();
    if let Some(authorization) = headers.get("authorization") {
        let auth = authorization.to_str().unwrap().to_string();
        if format!("Bearer {}", config.authorization) == auth {
            let mut location = String::new();
            if let Some(header_value) = headers.get("content-location") {
                if let Ok(location_str) = header_value.to_str() {
                    location = location_str.to_string();
                } else {
                    log::error!("Failed to convert 'content-location' header to string");
                }
            }
            let (repository, branch) = {
                let mut parts = location.split(';');
                let repository = parts.next().unwrap_or("");
                let branch = parts.next().unwrap_or("");
                (repository.to_string(), branch.to_string())
            };
            AuthResponse { ok: true, repository, branch }
        } else {
            log::error!("Invalid token: {}", auth);
            AuthResponse { ok: false, repository: String::new(), branch: String::new() }
        }
    } else {
        log::error!("No auth header received");
        AuthResponse { ok: false, repository: String::new(), branch: String::new() }
    }
}
