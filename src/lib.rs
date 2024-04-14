#![allow(rustdoc::bare_urls)]
#![doc = include_str!("../README.md")]

#[macro_use]
extern crate actix_web;

use std::io;
use std::process::exit;

use actix_web::{App, HttpServer, middleware, web};
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};

/// Module for the structs and functions called during startup.
mod constant;
/// Module for all the API entry points.
mod routes;
/// Module to store all the helper functions.
mod squire;

/// Contains entrypoint and initializer settings to trigger the asynchronous `HTTPServer`
///
/// # Examples
///
/// ```no_run
/// #[actix_rt::main]
/// async fn main() {
///     match github::start().await {
///         Ok(_) => {
///             println!("GitHub session terminated")
///         }
///         Err(err) => {
///             eprintln!("Error starting github: {}", err)
///         }
///     }
/// }
/// ```
pub async fn start() -> io::Result<()> {
    let metadata = constant::build_info();
    let config = squire::startup::get_config(&metadata);

    squire::startup::init_logger(config.debug, config.utc_logging, &metadata.crate_name);
    println!("{}[v{}] - {}", &metadata.pkg_name, &metadata.pkg_version, &metadata.description);
    if !squire::command::run("git version") {
        println!("'git' command line is mandatory!!");
        exit(1)
    }
    squire::ascii_art::random();

    if config.secure_session {
        log::warn!(
            "Secure session is turned on! This means that the server can ONLY be hosted via HTTPS or localhost"
        );
    }
    // Create a dedicated clone, since it will be used within closure
    let config_clone = config.clone();
    let session = constant::session_info();
    let host = format!("{}:{}", config.server_host, config.server_port);
    log::info!("{} [workers:{}] running on http://{} (Press CTRL+C to quit)",
        &metadata.pkg_name, &config.workers, &host);
    /*
        || syntax is creating a closure that serves as the argument to the HttpServer::new() method.
        The closure is defining the configuration for the Actix web server.
        The purpose of the closure is to configure the server before it starts listening for incoming requests.
     */
    let application = move || {
        App::new()  // Creates a new Actix web application
            .app_data(web::Data::new(config_clone.clone()))
            .app_data(web::Data::new(metadata.clone()))
            .app_data(web::Data::new(session.clone()))
            .app_data(web::PayloadConfig::default().limit(config_clone.max_payload_size))
            .wrap(squire::middleware::get_cors(config_clone.websites.clone()))
            .wrap(middleware::Logger::default())  // Adds a default logger middleware to the application
            .service(routes::filesystem::save_files)
            .service(routes::filesystem::remove_files)
    };
    let server = HttpServer::new(application)
        .workers(config.workers)
        .max_connections(config.max_connections);
    // Reference: https://actix.rs/docs/http2/
    if config.cert_file.exists() && config.key_file.exists() {
        log::info!("Binding SSL certificate to serve over HTTPS");
        let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
        builder.set_private_key_file(&config.key_file, SslFiletype::PEM).unwrap();
        builder.set_certificate_chain_file(&config.cert_file).unwrap();
        server.bind_openssl(host, builder)?
            .run()
            .await
    } else {
        server.bind(host)?
            .run()
            .await
    }
}
