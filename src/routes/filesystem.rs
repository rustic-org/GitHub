use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;

use actix_multipart::Multipart;
use actix_web::{HttpRequest, HttpResponse, web};
use futures_util::StreamExt as _;

use crate::{constant, squire};

/// Struct for the authentication response.
pub struct AuthResponse {
    ok: bool,
    path: String,
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
    AuthResponse {
        ok: false,
        path: String::new(),
    }
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
/// - [Chunk Upload](https://docs.rs/actix-multipart/latest/actix_multipart/struct.Multipart.html)
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
        return HttpResponse::Unauthorized().finish();
    }
    if auth_response.path.is_empty() {
        log::warn!("'content-location' header is missing");
        return HttpResponse::BadRequest().json("'content-location' header is missing");
    }
    let true_path = &config.github_source.join(auth_response.path);
    if let Some(parent) = true_path.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            let error = format!("Error creating directories: {}", err);
            log::error!("{}", error);
            return HttpResponse::ExpectationFailed().json(error);
        }
    }
    let mut destination = File::create(true_path).unwrap();
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

/// Deletes empty directories after removing the requested file.
///
/// # Arguments
///
/// * `path` - Filepath that was removed.
/// * `root` - GitHub source directory that has to be retained.
fn delete_empty_folders(path: &Path, root: &Path) {
    if let Some(parent) = path.parent() {
        // Recursively delete empty directories starting from the parent directory
        if parent.is_dir() && fs::read_dir(parent).map_or(false, |mut dir| dir.next().is_none()) {
            if parent == root {
                return;
            }
            if let Err(err) = fs::remove_dir(parent) {
                log::error!("Error deleting empty directory: {}", err);
            } else {
                log::info!("Deleted empty directory {:?}", parent);
                // Check recursively for more empty directories
                delete_empty_folders(parent, root);
            }
        }
    }
}

/// Deletes files that were removed in GH commits.
///
/// # Arguments
///
/// * `request` - A reference to the Actix web `HttpRequest` object.
/// * `session` - Session struct that holds the `session_mapping` and `session_tracker` to handle sessions.
/// * `config` - Configuration data for the application.
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
        return HttpResponse::Unauthorized().finish();
    }
    if auth_response.path.is_empty() {
        log::warn!("'content-location' header is missing");
        return HttpResponse::BadRequest().json("'content-location' header is missing");
    }
    let destination = &config.github_source.join(&auth_response.path);
    if destination.exists() {
        return match fs::remove_file(destination) {
            Ok(_) => {
                log::info!("Deleted file {:?}", destination);
                delete_empty_folders(destination, &config.github_source);
                HttpResponse::Ok().finish()
            }
            Err(err) => {
                let error = format!("Error deleting file: {}", err);
                log::error!("{}", error);
                HttpResponse::ExpectationFailed().json(error)
            }
        };
    };
    let error = format!("File not found: {:?}", destination);
    log::warn!("{}", error);
    HttpResponse::NotFound().json(error)
}
