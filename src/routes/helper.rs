use std::{fs, io, path};

use actix_web::HttpResponse;

use crate::{routes, squire};
use crate::squire::command;

pub struct Status {
    pub ok: bool,
    pub cloned: bool,
    pub response: String
}

pub fn fallback_clone(github_source: &path::Path,
                      repository: &String,
                      default_response: HttpResponse) -> HttpResponse {
    let dest = github_source.join(repository);
    if let Err(err) = fs::remove_dir_all(&dest) {
        log::error!("Error deleting out of sync repo: {:?}", err);
        return default_response;
    } else {
        log::info!("Deleted out of sync repo: {:?}", &dest);
    }
    let repo_validation = validate_repo(
        repository, github_source,
    );
    if repo_validation.ok && repo_validation.cloned {
        return HttpResponse::Ok().finish();
    }
    default_response
}

/// Validates the repository in data source, clones repo if unavailable.
///
/// # Arguments
///
/// * `repo` - Repository information.
/// * `config` - Configuration data for the application.
///
/// # Returns
///
/// Returns a boolean value to indicate results.
pub fn validate_repo(repository: &String, storage: &path::Path) -> Status {
    let destination = &storage.join(repository);
    if destination.exists() {
        let response = format!("{:?} exists", destination);
        log::info!("{}", response);
        return Status {
            ok: true,
            cloned: false,
            response
        };
    }
    let (org, repo) = {
        let mut parts = repository.split('/');
        (parts.next().unwrap_or(""), parts.next().unwrap_or(""))
    };
    let organization = &storage.join(org);
    log::info!("Creating directory for {:?}", organization);
    if let Err(err) = fs::create_dir_all(organization) {
        let response = format!("Error creating directory: {}", err);
        log::error!("{}", response);
        return Status {
            ok: false,
            cloned: false,
            response
        };
    }
    log::info!("Cloning '{}' into {:?}", repository, organization);
    // cd into {data_source}/{organization} and then clone the repository
    let cmd = format!("cd {} && git clone https://github.com/{}/{}.git",
                      organization.to_string_lossy(), org, repo);
    let clone_result = command::run(&cmd);
    Status {
        ok: clone_result,
        cloned: clone_result,
        response: format!("Failed to clone repo: {}", repository)
    }
}


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

/// Deletes a file.
///
/// # Arguments
///
/// * `destination` - Filepath that has to be removed.
/// * `source` - GitHub source directory.
///
/// # Returns
///
/// Returns a tuple of response code (as `u16`) and response message (as `String`)
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

/// Downloads a file.
///
/// # Arguments
///
/// * `auth_response` - Authentication response.
/// * `config` - Configuration data for the application.
/// * `downloadable` - File that has to be downloaded.
///
/// # Returns
///
/// Returns a `Result` object.
pub async fn download_file(auth_response: &routes::auth::AuthResponse,
                           config: &squire::settings::Config,
                           downloadable: &String) -> Result<(), io::Error> {
    let destination = &config.github_source
        .join(&auth_response.repository)
        .join(downloadable);
    let url = format!("https://raw.githubusercontent.com/{}/{}/{}",
                      auth_response.repository, auth_response.branch, downloadable);
    let response = match reqwest::get(url).await {
        Ok(res) => res,
        Err(err) => return Err(io::Error::new(io::ErrorKind::Other, err)),
    };
    let response = match response.error_for_status() {
        Ok(res) => res,
        Err(err) => return Err(io::Error::new(io::ErrorKind::Other, err)),
    };
    let mut dest_file = match fs::File::create(destination) {
        Ok(file) => file,
        Err(err) => return Err(err),
    };
    let bytes = response.bytes().await.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    match io::copy(&mut bytes.as_ref(), &mut dest_file) {
        Ok(_) => Ok(()),
        Err(err) => Err(err),
    }
}
