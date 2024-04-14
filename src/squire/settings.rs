use std::{path, thread};
use std::net::ToSocketAddrs;

/// Represents the configuration parameters for GitHub.
pub struct Config {
    /// Dictionary of key-value pairs for authorization (username and password).
    pub authorization: String,
    /// Directory path for source control.
    pub github_source: path::PathBuf,

    /// Debug flag to enable debug level logging.
    pub debug: bool,
    /// Boolean flag to enable UTC timezone in logging. Defaults to local timezone.
    pub utc_logging: bool,
    /// Server IP address.
    pub server_host: String,
    /// Server port number.
    pub server_port: u16,

    /// Number of worker threads to spin up the server.
    pub workers: usize,
    /// Maximum number of concurrent connections.
    pub max_connections: usize,
    /// Max payload allowed by the server in request body.
    pub max_payload_size: usize,
    /// List of websites (supports regex) to add to CORS configuration.
    pub websites: Vec<String>,

    /// Path to the private key file for SSL certificate
    pub key_file: path::PathBuf,
    /// Path to the full certificate chain file for SSL certificate
    pub cert_file: path::PathBuf,
}

/// Returns the default value for debug flag.
pub fn default_debug() -> bool { false }

/// Returns the default value for UTC logging.
pub fn default_utc_logging() -> bool { true }

/// Returns the default value for SSL files.
pub fn default_ssl() -> path::PathBuf { path::PathBuf::new() }

/// Returns the default server host based on the local machine's IP address.
pub fn default_server_host() -> String {
    let hostname = "localhost";
    match (hostname, 0).to_socket_addrs() {
        Ok(mut addrs) => {
            if let Some(addr) = addrs.find(|a| a.is_ipv4()) {
                return addr.ip().to_string();
            }
        }
        Err(err) => {
            log::error!("Error resolving hostname: {}", err);
        }
    }
    "localhost".to_string()
}

/// Returns the default server port (8000)
pub fn default_server_port() -> u16 { 8000 }

/// Returns the default number of worker threads (half of logical cores)
pub fn default_workers() -> usize {
    let logical_cores = thread::available_parallelism();
    match logical_cores {
        Ok(cores) => cores.get() / 2,
        Err(err) => {
            log::error!("{}", err);
            3
        }
    }
}

/// Returns the default maximum number of concurrent connections (3)
pub fn default_max_connections() -> usize { 3 }

/// Returns the default max payload size (100 MB)
pub fn default_max_payload_size() -> usize { 100 * 1024 * 1024 }

/// Returns an empty list as the default website (CORS configuration)
pub fn default_websites() -> Vec<String> { Vec::new() }
