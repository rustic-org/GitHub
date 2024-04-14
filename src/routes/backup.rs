use std::{fs, path, sync, collections};
use std::io::Write;

use actix_web::{HttpRequest, HttpResponse, web};
use actix_web::http::StatusCode;


use serde::{Deserialize, Serialize};

use crate::{constant, routes, squire};

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
            return if let Some(path) = headers.get("content-location") {
                if let Ok(location_str) = path.to_str() {
                    AuthResponse { ok: true, repository: location_str.to_string() }
                } else {
                    AuthResponse { ok: false, repository: "".to_string() }
                }
            } else {
                AuthResponse { ok: false, repository: "".to_string() }
            };
        } else {
            log::error!("Invalid token: {}", auth);
            AuthResponse { ok: false, repository: String::new() }
        }
    } else {
        log::error!("No auth header received");
        AuthResponse { ok: false, repository: String::new() }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Payload {
    #[serde(default = "default_hash")]
    create: collections::HashMap<String, String>,
    // sample: {'src/plain/.keep': ''}
    #[serde(default = "default_hash")]
    modify: collections::HashMap<String, String>,
    // sample: {'src/plain/main.py': 'src/main.py'}
    #[serde(default = "default_vec")]
    remove: Vec<String>,  // sample: ['matrix/executor.py', 'src/plain/main.py']
}

/// Struct for the authentication response.
pub struct AuthResponse {
    pub ok: bool,
    pub repository: String,
}

fn default_vec() -> Vec<String> { Vec::new() }

fn default_hash() -> collections::HashMap<String, String> { collections::HashMap::new() }


/// Deletes empty directories after removing the requested file.
///
/// # Arguments
///
/// * `path` - Filepath that was removed.
/// * `root` - GitHub source directory that has to be retained.
fn delete_empty_folders(path: &path::Path, root: &path::Path) {
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

pub fn delete_file(destination: &path::PathBuf, source: &path::Path) -> (u16, String) {
    if destination.exists() {
        return match fs::remove_file(destination) {
            Ok(_) => {
                let out = format!("Deleted file {:?}", destination);
                log::info!("{}", out);
                delete_empty_folders(destination, source);
                (200, out)
            }
            Err(err) => {
                let error = format!("Error deleting file: {}", err);
                log::error!("{}", error);
                (417, error)
            }
        };
    };
    let error = format!("File not found: {:?}", destination);
    log::warn!("{}", error);
    (404, error)
}

#[post("/backup")]
pub async fn backup_endpoint(request: HttpRequest,
                             payload: web::Json<Payload>,
                             session: web::Data<sync::Arc<constant::Session>>,
                             config: web::Data<sync::Arc<squire::settings::Config>>) -> HttpResponse {
    squire::custom::log_connection(&request, &session);
    let auth_response = verify_token(&request, &config);
    if !auth_response.ok {
        return HttpResponse::Unauthorized().finish();
    }
    if auth_response.repository.is_empty() {
        log::warn!("'content-location' header is invalid");
        return HttpResponse::BadRequest().json("'content-location' header is invalid");
    }
    let repo_validation = routes::intro::validate_repo(
        &auth_response.repository, &config.github_source,
    );
    if !repo_validation.ok {
        return HttpResponse::BadRequest().json("unable to locate or clone repository in data source");
    }
    if repo_validation.cloned {
        log::info!("Repository '{}' was cloned, so no point in proceeding further", &auth_response.repository);
        return HttpResponse::Ok().finish();
    }
    // todo: implement threading
    for (filepath, content) in &payload.create {
        let true_path = &config.github_source
            .join(&auth_response.repository)
            .join(filepath);

        // Creates all the directories along the way
        if let Some(parent) = true_path.parent() {
            if let Err(err) = fs::create_dir_all(parent) {
                let error = format!("Error creating directories: {}", err);
                log::error!("{}", error);
                return HttpResponse::ExpectationFailed().json(error);
            }
        }

        let mut file = match fs::File::create(true_path) {
            Ok(file_buf) => file_buf,
            Err(err) => {
                let error = format!("Error creating file: {}", err);
                log::error!("{}", error);
                return HttpResponse::ExpectationFailed().json(error);
            }
        };
        match file.write_all(content.as_bytes()) {
            Ok(_) => log::info!("File content has been updated for {:?}", true_path),
            Err(err) => {
                let error = format!("Error writing to file: {}", err);
                log::error!("{}", error);
                return HttpResponse::ExpectationFailed().json(error);
            }
        }
    }
    for (old_name, new_name) in &payload.modify {
        let src = &config.github_source
            .join(&auth_response.repository)
            .join(old_name);
        let dst = &config.github_source
            .join(&auth_response.repository)
            .join(new_name);
        match fs::rename(src, dst) {
            Ok(()) => log::info!("File [{:?}] has been moved to [{:?}]", src, dst),
            Err(err) => {
                let error = format!("Failed to move file [{:?}] to [{:?}] - {}", src, dst, err);
                log::error!("{}", error);
                return HttpResponse::ExpectationFailed().json(error);
            }
        }
    }
    for removable in &payload.remove {
        let destination = &config.github_source
            .join(&auth_response.repository)
            .join(removable);
        let (code, out) = delete_file(destination, &config.github_source);
        if code != 200 {
            return HttpResponse::build(StatusCode::from_u16(code).unwrap()).json(out);
        }
    }
    HttpResponse::Ok().finish()
}
