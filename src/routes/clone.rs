use std::{fs, sync};

use actix_web::{HttpRequest, HttpResponse, web};

use crate::{constant, routes, squire};
use crate::routes::helper::validate_repo;

#[get("/clone")]
pub async fn clone_endpoint(request: HttpRequest,
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
    let destination = &&config.github_source.join(&auth_response.repository);
    if destination.exists() {
        log::warn!("Repository {} exists!", &auth_response.repository);
        if let Err(err) = fs::remove_dir_all(destination) {
            let error = format!("Error deleting repo: {:?}", err);
            log::error!("{}", error);
            return HttpResponse::ExpectationFailed().json(error);
        } else {
            log::info!("Deleted repo: {:?}", &destination);
        }
    }
    let repo_validation = validate_repo(
        &auth_response.repository, &config.github_source,
    );
    if repo_validation.ok && repo_validation.cloned {
        return HttpResponse::Ok().finish();
    }
    let error = format!("Error deleting repo: {:?}", repo_validation.response);
    log::error!("{}", error);
    HttpResponse::ExpectationFailed().json(error)
}
