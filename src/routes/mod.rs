/// Module to check for repository and clone if not present.
mod helper;
/// Backup endpoint to update files that were modified.
pub mod backup;
/// Clone endpoint to re-clone the repository.
pub mod clone;
/// Module to validate authentication.
mod auth;
