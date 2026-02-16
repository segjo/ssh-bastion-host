use actix_web::{web, App, HttpServer};
use std::sync::{Arc, Mutex};

mod ssh_monitor;
mod models;
mod handlers;

use ssh_monitor::SSHMonitor;

#[derive(Clone)]
pub struct AppState {
    monitor: Arc<Mutex<SSHMonitor>>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Starting SSH Bastion Dashboard...");

    let monitor = Arc::new(Mutex::new(SSHMonitor::new()));
    let app_state = AppState {
        monitor: monitor.clone(),
    };

    // Spawn a task to monitor SSH connections periodically
    let monitor_clone = monitor.clone();
    tokio::spawn(async move {
        loop {
            if let Ok(mut m) = monitor_clone.lock() {
                if let Err(e) = m.refresh() {
                    eprintln!("Failed to refresh SSH connections: {}", e);
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    });

    let app_state_clone = app_state.clone();

    println!("Starting HTTP server on 0.0.0.0:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(app_state_clone.clone()))
            .route("/", web::get().to(handlers::index))
            .route("/api/connections", web::get().to(handlers::get_connections))
            .route("/api/connections/json", web::get().to(handlers::get_connections_json))
            .route("/api/health", web::get().to(handlers::health_check))
            .route("/style.css", web::get().to(handlers::style_css))
            .route("/script.js", web::get().to(handlers::script_js))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
