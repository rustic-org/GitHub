use std::fs;
use std::fs::File;
use std::io::Write;
use std::sync::Arc;

use actix_multipart::Multipart;
use actix_web::{http, HttpRequest, HttpResponse, web};
use futures_util::StreamExt as _;

use crate::{constant, squire};

/// Constructs an `HttpResponse` for failed `session_token` verification.
///
/// # Arguments
///
/// * `auth_response` - The authentication response containing details of the failure.
/// * `config` - Configuration data for the application.
///
/// # Returns
///
/// Returns an `HttpResponse` with a redirect, setting a cookie with the failure detail.
pub fn failed_auth() -> HttpResponse {
    let mut response = HttpResponse::build(http::StatusCode::UNAUTHORIZED);
    response.finish()
}

pub struct AuthResponse {
    ok: bool,
    path: String,
}

pub fn verify_token(request: &HttpRequest,
                    config: &web::Data<Arc<squire::settings::Config>>) -> AuthResponse {
    let headers = request.headers();
    if let Some(authorization) = headers.get("authorization") {
        let auth = authorization.to_str().unwrap().to_string();
        if format!("Bearer {}", config.authorization) == auth {
            let mut file_path = String::new();
            if let Some(path) = headers.get("content-location") {
                if let Ok(path_str) = path.to_str() {
                    file_path = path_str.to_string();
                } else {
                    log::error!("Failed to convert 'path' header to string");
                }
            }
            return AuthResponse {
                ok: true,
                path: file_path,
            };
        }
        log::error!("Invalid token: received {}", auth)
    }
    log::error!("No auth header received");
    return AuthResponse {
        ok: false,
        path: String::new(),
    };
}

/// Saves files locally by breaking them into chunks.
///
/// # Arguments
///
/// * `request` - A reference to the Actix web `HttpRequest` object.
/// * `payload` - Mutable multipart struct that is sent from the UI as `FormData`.
/// * `session` - Session struct that holds the `session_mapping` and `session_tracker` to handle sessions.
/// * `config` - Configuration data for the application.
///
/// ## References
/// - [Server Side](https://docs.rs/actix-multipart/latest/actix_multipart/struct.Multipart.html)
/// - [Client Side (not implemented)](https://accreditly.io/articles/uploading-large-files-with-chunking-in-javascript)
///
/// # Returns
///
/// * `200` - Plain HTTPResponse indicating that the file was uploaded.
/// * `422` - HTTPResponse with JSON object indicating that the payload was incomplete.
/// * `400` - HTTPResponse with JSON object indicating that the payload was invalid.
#[post("/upload")]
pub async fn save_files(request: HttpRequest,
                        mut payload: Multipart,
                        session: web::Data<Arc<constant::Session>>,
                        config: web::Data<Arc<squire::settings::Config>>) -> HttpResponse {
    squire::custom::log_connection(&request, &session);
    let auth_response = verify_token(&request, &config);
    if !auth_response.ok {
        return failed_auth();
    }

    if auth_response.path.is_empty() {
        log::warn!("No path received!!");
        return HttpResponse::BadRequest().finish();
    }

    let true_path = &config.github_source.join(auth_response.path);
    if let Some(parent) = true_path.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            log::error!("Error creating directories: {}", err);
            return HttpResponse::BadRequest().finish();
        }
    }
    let mut destination = File::create(&true_path).unwrap();
    while let Some(item) = payload.next().await {
        match item {
            Ok(mut field) => {
                let filename = field.content_disposition().get_filename().unwrap();
                log::info!("Downloading '{}'", &filename);
                while let Some(fragment) = field.next().await {
                    match fragment {
                        Ok(chunk) => {
                            destination.write_all(&chunk).unwrap();
                        }
                        Err(err) => {
                            // User might have aborted file upload
                            let error = format!("Error processing chunk: {}", err);
                            log::warn!("{}", &error);
                            return HttpResponse::UnprocessableEntity().json(error);
                        }
                    }
                }
            }
            Err(err) => {
                let error = format!("Error processing field: {}", err);
                log::error!("{}", &error);
                return HttpResponse::BadRequest().json(error);
            }
        }
    }
    HttpResponse::Ok().finish()
}

/// Saves files locally by breaking them into chunks.
///
/// # Arguments
///
/// * `request` - A reference to the Actix web `HttpRequest` object.
/// * `payload` - Mutable multipart struct that is sent from the UI as `FormData`.
/// * `session` - Session struct that holds the `session_mapping` and `session_tracker` to handle sessions.
/// * `config` - Configuration data for the application.
///
/// ## References
/// - [Server Side](https://docs.rs/actix-multipart/latest/actix_multipart/struct.Multipart.html)
/// - [Client Side (not implemented)](https://accreditly.io/articles/uploading-large-files-with-chunking-in-javascript)
///
/// # Returns
///
/// * `200` - Plain HTTPResponse indicating that the file was uploaded.
/// * `422` - HTTPResponse with JSON object indicating that the payload was incomplete.
/// * `400` - HTTPResponse with JSON object indicating that the payload was invalid.
#[delete("/delete")]
pub async fn remove_files(request: HttpRequest,
                          session: web::Data<Arc<constant::Session>>,
                          config: web::Data<Arc<squire::settings::Config>>) -> HttpResponse {
    squire::custom::log_connection(&request, &session);
    let auth_response = verify_token(&request, &config);
    if !auth_response.ok {
        return failed_auth();
    }
    if auth_response.path.is_empty() {
        log::warn!("No path received!!");
        return HttpResponse::BadRequest().finish();
    }
    let destination = &config.github_source.join(&auth_response.path);
    if destination.exists() {
        return match fs::remove_file(destination) {
            Ok(_) => {
                log::info!("Deleted file {:?}", destination);
                HttpResponse::Ok().finish()
            }
            Err(err) => {
                log::error!("Error deleting file: {}", err);
                HttpResponse::ExpectationFailed().finish()
            }
        };
    };
    log::warn!("File not found: {:?}", destination);
    HttpResponse::NotFound().finish()
}
