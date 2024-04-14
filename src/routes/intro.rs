use std::fs;
use std::path::{Path};

use crate::squire::command;

pub struct Status {
    pub ok: bool,
    pub cloned: bool,
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
pub fn validate_repo(repository: &String, storage: &Path) -> Status {
    let destination = &storage.join(repository);
    if destination.exists() {
        log::info!("{:?} exists", destination);
        return Status {
            ok: true,
            cloned: false
        }
    }
    let (org, repo) = {
        let mut parts = repository.split('/');
        (parts.next().unwrap_or(""), parts.next().unwrap_or(""))
    };
    let organization = &storage.join(org);
    log::info!("Creating directory for {:?}", organization);
    if let Err(err) = fs::create_dir_all(organization) {
        log::error!("Error creating directory: {}", err);
        return Status {
            ok: false,
            cloned: false
        }
    }
    log::info!("Cloning '{}' into {:?}", repository, organization);
    // cd into {data_source}/{organization} and then clone the repository
    let cmd = format!("cd {} && git clone https://github.com/{}/{}.git",
                      organization.to_string_lossy(), org, repo);
    let clone_result = command::run(&cmd);
    Status {
        ok: clone_result,
        cloned: clone_result
    }
}
