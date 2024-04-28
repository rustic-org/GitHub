use std::{collections, fs, sync};
use std::io::Write;

use actix_web::{HttpRequest, HttpResponse, web};
use actix_web::http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::{constant, routes, squire};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Payload {
    #[serde(default = "default_hash")]
    // sample: {'src/plain/.keep': 'some text'}
    create: collections::HashMap<String, String>,

    #[serde(default = "default_hash")]
    // sample: {'src/plain/main.py': 'src/main.py'} - move/rename
    modify: collections::HashMap<String, String>,

    #[serde(default = "default_vec")]
    // sample: ['matrix/executor.py', 'src/plain/main.py']
    remove: Vec<String>,

    #[serde(default = "default_vec")]
    // sample: ['src/sample.png'] - since bytes can't be JSON encoded
    download: Vec<String>,
}

fn default_vec() -> Vec<String> { Vec::new() }

fn default_hash() -> collections::HashMap<String, String> { collections::HashMap::new() }


#[post("/backup")]
pub async fn backup_endpoint(request: HttpRequest,
                             payload: web::Json<Payload>,
                             session: web::Data<sync::Arc<constant::Session>>,
                             config: web::Data<sync::Arc<squire::settings::Config>>) -> HttpResponse {
    squire::custom::log_connection(&request, &session);
    let auth_response = routes::auth::verify_token(&request, &config);
    if !auth_response.ok {
        return HttpResponse::Unauthorized().finish();
    }
    if auth_response.repository.is_empty() {
        log::warn!("'content-location' header is invalid");
        return HttpResponse::BadRequest().json("'content-location' header is invalid");
    }
    let repo_validation = routes::helper::validate_repo(
        &auth_response.repository, &config.github_source,
    );
    if !repo_validation.ok {
        return HttpResponse::BadRequest().json("unable to locate or clone repository in data source");
    }
    if repo_validation.cloned {
        log::info!("Repository '{}' was cloned, so no point in proceeding further", &auth_response.repository);
        return HttpResponse::Ok().finish();
    }

    for (filepath, content) in &payload.create {
        let true_path = &config.github_source
            .join(&auth_response.repository)
            .join(filepath);

        // Creates all the directories along the way
        if let Some(parent) = true_path.parent() {
            if let Err(err) = fs::create_dir_all(parent) {
                let error = format!("Error creating directories: {}", err);
                log::error!("{}", error);
                return routes::helper::fallback_clone(&config.github_source,
                                                      &auth_response.repository,
                                                      HttpResponse::ExpectationFailed().json(error));
            }
        }

        let mut file = match fs::File::create(true_path) {
            Ok(file_buf) => file_buf,
            Err(err) => {
                let error = format!("Error creating file: {}", err);
                log::error!("{}", error);
                return routes::helper::fallback_clone(&config.github_source,
                                                      &auth_response.repository,
                                                      HttpResponse::ExpectationFailed().json(error));
            }
        };
        match file.write_all(content.as_bytes()) {
            Ok(_) => log::info!("File content has been updated for {:?}", true_path),
            Err(err) => {
                let error = format!("Error writing to file: {}", err);
                log::error!("{}", error);
                return routes::helper::fallback_clone(&config.github_source,
                                                      &auth_response.repository,
                                                      HttpResponse::ExpectationFailed().json(error));
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
                return routes::helper::fallback_clone(&config.github_source,
                                                      &auth_response.repository,
                                                      HttpResponse::ExpectationFailed().json(error));
            }
        }
    }
    for removable in &payload.remove {
        let destination = &config.github_source
            .join(&auth_response.repository)
            .join(removable);
        let (code, out) = routes::helper::delete_file(destination, &config.github_source);
        if code != 200 {
            return routes::helper::fallback_clone(&config.github_source,
                                                  &auth_response.repository,
                                                  HttpResponse::build(StatusCode::from_u16(code).unwrap()).json(out));
        }
    }
    for downloadable in &payload.download {
        match routes::helper::download_file(&auth_response, &config, downloadable).await {
            Ok(_) => log::info!("Download successful: {}", downloadable),
            Err(err) => {
                let error = format!("Error downloading file: {}", err);
                log::error!("{}", error);
                return routes::helper::fallback_clone(&config.github_source,
                                                      &auth_response.repository,
                                                      HttpResponse::ExpectationFailed().json(error));
            }
        }
    }
    HttpResponse::Ok().finish()
}
