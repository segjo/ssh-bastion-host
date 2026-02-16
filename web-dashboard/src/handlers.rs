use actix_web::{web, HttpResponse};
use serde_json::json;
use crate::AppState;
use crate::models::ConnectionStats;

pub async fn index(_state: web::Data<AppState>) -> HttpResponse {
    let html = include_str!("templates/index.html");

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
}

pub async fn get_connections(state: web::Data<AppState>) -> HttpResponse {
    let monitor = state.monitor.lock().unwrap();
    let connections = monitor.get_connections();
    let timestamp = monitor.last_update();
    drop(monitor);

    let stats = ConnectionStats {
        total_connections: connections.len(),
        timestamp,
        connections,
    };

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(render_connections_html(&stats))
}

pub async fn get_connections_json(state: web::Data<AppState>) -> HttpResponse {
    let monitor = state.monitor.lock().unwrap();
    let connections = monitor.get_connections();
    let timestamp = monitor.last_update();
    drop(monitor);

    let stats = ConnectionStats {
        total_connections: connections.len(),
        timestamp,
        connections,
    };

    HttpResponse::Ok()
        .content_type("application/json")
        .json(stats)
}

pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().json(json!({
        "status": "ok",
        "version": "0.1.0"
    }))
}

pub async fn style_css() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/css")
        .body(include_str!("templates/style.css"))
}

pub async fn script_js() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/javascript")
        .body(include_str!("templates/script.js"))
}

fn render_connections_html(stats: &ConnectionStats) -> String {
    if stats.connections.is_empty() {
        return "<div class='no-connections'>No active connections</div>".to_string();
    }

    let mut html = String::from("<div class='connections-list'>");
    
    for conn in &stats.connections {
        html.push_str(&format!(
            r#"
            <div class='connection-card'>
                <div class='connection-header'>
                    <span class='status-badge'>{}</span>
                    <span class='pid'>PID: {}</span>
                </div>
                <div class='connection-body'>
                    <div class='connection-row'>
                        <span class='label'>User:</span>
                        <span class='value'>{}</span>
                    </div>
                    <div class='connection-row'>
                        <span class='label'>Bastion:</span>
                        <span class='value'>{}:{}</span>
                    </div>
                    <div class='connection-row'>
                        <span class='label'>Reverse Port:</span>
                        <span class='value highlight'>{}</span>
                    </div>
                    <div class='connection-row'>
                        <span class='label'>Local:</span>
                        <span class='value'>{}:{}</span>
                    </div>
                    <div class='connection-row'>
                        <span class='label'>Status:</span>
                        <span class='value'>{}</span>
                    </div>
                </div>
            </div>
            "#,
            conn.status,
            conn.pid,
            conn.user,
            conn.bastion_host,
            conn.bastion_port,
            conn.remote_port,
            conn.local_bind,
            conn.local_port,
            conn.status
        ));
    }

    html.push_str("</div>");
    html
}
