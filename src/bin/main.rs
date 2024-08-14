use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;
use std::sync::Arc;

use firebase_rs::Firebase;
use log::{debug, error, info, warn};
use rust_http_server::ThreadPool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::runtime::Runtime;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Movie {
    title: String,
    director: String,
    release_year: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Actor {
    name: String,
    date_of_birth: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Review {
    movie_id: String,
    reviewer: String,
    rating: u32,
    comment: String,
}

const FIREBASE_URL: &str =
    "https://rust-movie-project-default-rtdb.europe-west1.firebasedatabase.app/";

fn main() {
    env_logger::init(); // Initialize the logger

    let listener = TcpListener::bind("0.0.0.0:8080").unwrap();
    let pool = ThreadPool::new(4);
    info!("Server started on port 8080");

    // Initialize a single Tokio runtime
    let runtime = Arc::new(Runtime::new().unwrap());

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let runtime = Arc::clone(&runtime);
                pool.execute(move || {
                    if let Err(e) = handle_connection(stream, runtime) {
                        error!("Connection handling failed: {}", e);
                    }
                });
            }
            Err(e) => {
                error!("Failed to establish a connection: {}", e);
            }
        }
    }
}

fn handle_connection(
    mut stream: TcpStream,
    runtime: Arc<Runtime>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = [0; 1024];
    stream.read(&mut buffer)?;

    let request = String::from_utf8_lossy(&buffer[..]);

    let (method, path) = parse_request_line(&request);
    info!("Received request: {} {}", method, path);

    let response = runtime.block_on(async {
        match (method, path) {
            ("GET", "/api/movies") => handle_get("movies").await,
            ("POST", "/api/movies") => handle_post("movies", &request).await,
            ("PUT", "/api/movies") => handle_put("movies", &request).await,
            ("DELETE", "/api/movies") => handle_delete("movies", &request).await,
            ("GET", "/api/actors") => handle_get("actors").await,
            ("POST", "/api/actors") => handle_post("actors", &request).await,
            ("PUT", "/api/actors") => handle_put("actors", &request).await,
            ("DELETE", "/api/actors") => handle_delete("actors", &request).await,
            ("GET", "/api/reviews") => handle_get("reviews").await,
            ("POST", "/api/reviews") => handle_post("reviews", &request).await,
            ("PUT", "/api/reviews") => handle_put("reviews", &request).await,
            ("DELETE", "/api/reviews") => handle_delete("reviews", &request).await,
            _ => {
                warn!("Unknown path: {}", path);
                handle_404()
            }
        }
    });

    stream.write_all(response.as_bytes())?;
    stream.flush()?;
    Ok(())
}

fn parse_request_line(request: &str) -> (&str, &str) {
    let mut lines = request.lines();
    if let Some(request_line) = lines.next() {
        let parts: Vec<&str> = request_line.split_whitespace().collect();
        if parts.len() >= 2 {
            return (parts[0], parts[1]);
        }
    }
    ("", "")
}

fn format_response(status_line: &str, contents: &str) -> String {
    format!(
        "{}\r\nContent-Length: {}\r\n\r\n{}",
        status_line,
        contents.len(),
        contents
    )
}

async fn firebase_request(
    path: &str,
    request_type: &str,
    data: Option<&serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let firebase = Firebase::new(FIREBASE_URL).map_err(|e| e.to_string())?;
    debug!("Firebase request: {} {}", request_type, path);

    match request_type {
        "GET" => firebase
            .at(path)
            .get::<serde_json::Value>()
            .await
            .map_err(|e| {
                error!("Firebase GET request failed: {}", e);
                e.to_string()
            }),
        "POST" => {
            if let Some(data) = data {
                if let Err(e) = firebase.at(path).set(data).await {
                    error!("Firebase POST request failed: {}", e);
                    return Err(e.to_string());
                }
            }
            Ok(json!({"status": "created"}))
        }
        "PUT" => {
            if let Some(data) = data {
                if let Err(e) = firebase.at(path).update(data).await {
                    error!("Firebase PUT request failed: {}", e);
                    return Err(e.to_string());
                }
            }
            Ok(json!({"status": "updated"}))
        }
        "DELETE" => {
            if let Err(e) = firebase.at(path).delete().await {
                error!("Firebase DELETE request failed: {}", e);
                return Err(e.to_string());
            }
            Ok(json!({"status": "deleted"}))
        }
        _ => {
            error!("Unsupported request type: {}", request_type);
            Err("Unsupported request type".to_string())
        }
    }
}

async fn handle_get(resource: &str) -> String {
    match firebase_request(resource, "GET", None).await {
        Ok(data) => {
            let contents = if data.is_null() {
                "[]".to_string()
            } else {
                data.to_string()
            };
            format_response("HTTP/1.0 200 OK", &contents)
        }
        Err(_) => {
            error!("Failed to handle GET request for {}", resource);
            format_response(
                "HTTP/1.0 500 INTERNAL SERVER ERROR",
                "Failed to retrieve data",
            )
        }
    }
}

async fn handle_post(resource: &str, request: &str) -> String {
    if let Some(body) = request.split("\r\n\r\n").nth(1) {
        let sanitized_body = body.replace('\0', "").trim().to_string();
        let path = format!("{}/", resource);

        match serde_json::from_str::<serde_json::Value>(&sanitized_body) {
            Ok(data) => match firebase_request(&path, "POST", Some(&data)).await {
                Ok(_) => format_response("HTTP/1.0 201 CREATED", "Resource created"),
                Err(_) => {
                    error!("Failed to create resource in {}", resource);
                    handle_500()
                }
            },
            Err(_) => {
                warn!("Failed to parse POST request body");
                handle_400()
            }
        }
    } else {
        warn!("POST request missing body");
        handle_400()
    }
}

async fn handle_put(resource: &str, request: &str) -> String {
    if let Some(body) = request.split("\r\n\r\n").nth(1) {
        let sanitized_body = body.replace('\0', "").trim().to_string();

        match serde_json::from_str::<serde_json::Value>(&sanitized_body) {
            Ok(data) => {
                if let Some(id) = data.get("id").and_then(|id| id.as_str()) {
                    let path = format!("{}/{}", resource, id);
                    match firebase_request(&path, "PUT", Some(&data)).await {
                        Ok(_) => format_response("HTTP/1.0 200 OK", "Resource updated"),
                        Err(_) => {
                            error!("Failed to update resource in {}", resource);
                            handle_500()
                        }
                    }
                } else {
                    warn!("PUT request missing id");
                    handle_400()
                }
            }
            Err(_) => {
                warn!("Failed to parse PUT request body");
                handle_400()
            }
        }
    } else {
        warn!("PUT request missing body");
        handle_400()
    }
}

async fn handle_delete(resource: &str, request: &str) -> String {
    if let Some(body) = request.split("\r\n\r\n").nth(1) {
        let sanitized_body = body.replace('\0', "").trim().to_string();

        match serde_json::from_str::<serde_json::Value>(&sanitized_body) {
            Ok(data) => {
                if let Some(id) = data.get("id").and_then(|id| id.as_str()) {
                    let path = format!("{}/{}", resource, id);
                    match firebase_request(&path, "DELETE", None).await {
                        Ok(_) => format_response("HTTP/1.0 200 OK", "Resource deleted"),
                        Err(_) => {
                            error!("Failed to delete resource in {}", resource);
                            handle_500()
                        }
                    }
                } else {
                    warn!("DELETE request missing id");
                    handle_400()
                }
            }
            Err(_) => {
                warn!("Failed to parse DELETE request body");
                handle_400()
            }
        }
    } else {
        warn!("DELETE request missing body");
        handle_400()
    }
}

fn handle_400() -> String {
    format_response("HTTP/1.0 400 BAD REQUEST", "Invalid request")
}

fn handle_404() -> String {
    format_response("HTTP/1.0 404 NOT FOUND", "Resource not found")
}

fn handle_500() -> String {
    format_response("HTTP/1.0 500 INTERNAL SERVER ERROR", "An error occurred")
}
