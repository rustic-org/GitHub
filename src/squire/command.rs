use std::process::Command;

/// Runs shell commands, and validates the result.
///
/// * `cmd` - Takes the command as an argument.
///
/// # Returns
///
/// Returns a boolean value to indicate results.
pub fn run(cmd: &str) -> bool {
    log::info!("Executing '{}'", cmd);
    match Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output()
    {
        Ok(output) => {
            log::debug!("Status Code: {}", output.status);
            if output.status.success() {
                if let Some(stdout) = String::from_utf8(output.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                {
                    log::info!("Output: '{}'", stdout);
                }
                // if let Ok(stdout) = String::from_utf8(output.stdout) {
                //     if !stdout.trim().is_empty() {
                //         log::info!("Output: '{}'", stdout.trim());
                //     }
                // }
                true
            } else {
                if let Some(stderr) = String::from_utf8(output.stderr)
                    .ok()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                {
                    log::info!("Error: '{}'", stderr);
                }
                false
            }
        }
        Err(err) => {
            log::error!("Failed to execute command: {}", err);
            false
        }
    }
}
