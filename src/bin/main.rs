use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;

use serde_json::{json, Value};
use std::fs;
use std::io::{Read, Write};

use rust_http_server::ThreadPool;

fn main() {
    let listener = TcpListener::bind("0.0.0.0:8080").unwrap();

    let pool = ThreadPool::new(4);

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        pool.execute(|| {
            handle_connection(stream);
        });
    }
}

fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0; 1024]; // FEATURE: make buffer of arbitrary size

    stream.read(&mut buffer).unwrap();

    let request = String::from_utf8_lossy(&buffer[..]);

    // Parsing the request method and path
    let (method, path) = parse_request_line(&request);

    let response = match (method, path) {
        ("GET", "/api/movies") => handle_get_movies(),
        ("POST", "/api/movies") => handle_post_movies(&request),
        ("PUT", "/api/movies") => handle_put_movies(&request),
        ("DELETE", "/api/movies") => handle_delete_movies(),

        ("GET", "/api/actors") => handle_get_actors(),
        ("POST", "/api/actors") => handle_post_actors(&request),
        ("PUT", "/api/actors") => handle_put_actors(&request),
        ("DELETE", "/api/actors") => handle_delete_actors(),

        ("GET", "/api/reviews") => handle_get_reviews(),
        ("POST", "/api/reviews") => handle_post_reviews(&request),
        ("PUT", "/api/reviews") => handle_put_reviews(&request),
        ("DELETE", "/api/reviews") => handle_delete_reviews(),

        _ => handle_404(),
    };

    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();
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

// Handlers for /api/movies
fn handle_get_movies() -> String {
    let status_line = "HTTP/1.0 200 OK";
    let contents = fs::read_to_string("api/movies.json").expect("Failed to read file");
    format!(
        "{}\r\nContent-Length: {}\r\n\r\n{}",
        status_line,
        contents.len(),
        contents
    )
}

fn handle_post_movies(request: &str) -> String {
    let status_line = "HTTP/1.0 201 CREATED";
    let contents = "Movie created"; // Implement actual creation logic
    format!(
        "{}\r\nContent-Length: {}\r\n\r\n{}",
        status_line,
        contents.len(),
        contents
    )
}

fn handle_put_movies(request: &str) -> String {
    let status_line = "HTTP/1.0 200 OK";
    let contents = "Movie updated"; // Implement actual update logic
    format!(
        "{}\r\nContent-Length: {}\r\n\r\n{}",
        status_line,
        contents.len(),
        contents
    )
}

fn handle_delete_movies() -> String {
    let status_line = "HTTP/1.0 200 OK";
    let contents = "Movie deleted"; // Implement actual deletion logic
    format!(
        "{}\r\nContent-Length: {}\r\n\r\n{}",
        status_line,
        contents.len(),
        contents
    )
}

// Handlers for /api/actors
fn handle_get_actors() -> String {
    let status_line = "HTTP/1.0 200 OK";
    let contents = fs::read_to_string("api/actors.json").expect("Failed to read file");
    format!(
        "{}\r\nContent-Length: {}\r\n\r\n{}",
        status_line,
        contents.len(),
        contents
    )
}

fn handle_post_actors(request: &str) -> String {
    let status_line = "HTTP/1.0 201 CREATED";
    let contents = "Actor created"; // Implement actual creation logic
    format!(
        "{}\r\nContent-Length: {}\r\n\r\n{}",
        status_line,
        contents.len(),
        contents
    )
}

fn handle_put_actors(request: &str) -> String {
    let status_line = "HTTP/1.0 200 OK";
    let contents = "Actor updated"; // Implement actual update logic
    format!(
        "{}\r\nContent-Length: {}\r\n\r\n{}",
        status_line,
        contents.len(),
        contents
    )
}

fn handle_delete_actors() -> String {
    let status_line = "HTTP/1.0 200 OK";
    let contents = "Actor deleted"; // Implement actual deletion logic
    format!(
        "{}\r\nContent-Length: {}\r\n\r\n{}",
        status_line,
        contents.len(),
        contents
    )
}

// Handlers for /api/reviews
fn handle_get_reviews() -> String {
    let status_line = "HTTP/1.0 200 OK";
    let contents = fs::read_to_string("api/reviews.json").expect("Failed to read file");
    format!(
        "{}\r\nContent-Length: {}\r\n\r\n{}",
        status_line,
        contents.len(),
        contents
    )
}

fn handle_post_reviews(request: &str) -> String {
    let status_line = "HTTP/1.0 201 CREATED";
    let contents = "Review created"; // Implement actual creation logic
    format!(
        "{}\r\nContent-Length: {}\r\n\r\n{}",
        status_line,
        contents.len(),
        contents
    )
}

fn handle_put_reviews(request: &str) -> String {
    let status_line = "HTTP/1.0 200 OK";
    let contents = "Review updated"; // Implement actual update logic
    format!(
        "{}\r\nContent-Length: {}\r\n\r\n{}",
        status_line,
        contents.len(),
        contents
    )
}

fn handle_delete_reviews() -> String {
    let status_line = "HTTP/1.0 200 OK";
    let contents = "Review deleted"; // Implement actual deletion logic
    format!(
        "{}\r\nContent-Length: {}\r\n\r\n{}",
        status_line,
        contents.len(),
        contents
    )
}

// 404 handler
fn handle_404() -> String {
    let status_line = "HTTP/1.0 404 NOT FOUND";
    let contents = "404 - Not Found";
    format!(
        "{}\r\nContent-Length: {}\r\n\r\n{}",
        status_line,
        contents.len(),
        contents
    )
}

fn extract_json_body(request: &str) -> &str {
    // Assuming the body is directly after a double newline, adjust accordingly if needed
    request.split("\r\n\r\n").nth(1).unwrap_or("")
}
