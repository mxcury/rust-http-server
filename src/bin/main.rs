use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;

use firebase_rs::Firebase;
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

#[tokio::main]
async fn main() {
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
    let mut buffer = [0; 1024];
    stream.read(&mut buffer).unwrap();
    let request = String::from_utf8_lossy(&buffer[..]);

    let (method, path) = parse_request_line(&request);

    let response = Runtime::new().unwrap().block_on(async {
        match (method, path) {
            ("GET", "/api/movies") => handle_get_movies().await,
            ("POST", "/api/movies") => handle_post_movies(&request).await,
            ("PUT", "/api/movies") => handle_put_movies(&request).await,
            ("DELETE", "/api/movies") => handle_delete_movies(&request).await,
            ("GET", "/api/actors") => handle_get_actors().await,
            ("POST", "/api/actors") => handle_post_actors(&request).await,
            ("PUT", "/api/actors") => handle_put_actors(&request).await,
            ("DELETE", "/api/actors") => handle_delete_actors(&request).await,
            ("GET", "/api/reviews") => handle_get_reviews().await,
            ("POST", "/api/reviews") => handle_post_reviews(&request).await,
            ("PUT", "/api/reviews") => handle_put_reviews(&request).await,
            ("DELETE", "/api/reviews") => handle_delete_reviews(&request).await,
            _ => handle_404(),
        }
    });

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

async fn handle_get_movies() -> String {
    let firebase = Firebase::new(FIREBASE_URL).unwrap();
    let movies_result = firebase.at("movies").get::<serde_json::Value>().await;

    let status_line;
    let contents;

    match movies_result {
        Ok(movies) => {
            if movies.is_null() {
                status_line = "HTTP/1.0 200 OK";
                contents = "[]".to_string(); // Return an empty list if no movies are found
            } else {
                status_line = "HTTP/1.0 200 OK";
                contents = movies.to_string();
            }
        }
        Err(_) => {
            status_line = "HTTP/1.0 500 INTERNAL SERVER ERROR";
            contents = "Failed to retrieve movies".to_string();
        }
    }

    format!(
        "{}\r\nContent-Length: {}\r\n\r\n{}",
        status_line,
        contents.len(),
        contents
    )
}

async fn handle_post_movies(request: &str) -> String {
    println!("Received POST request: {}", request);

    let firebase = Firebase::new(FIREBASE_URL).unwrap();

    if let Some(body) = request.split("\r\n\r\n").nth(1) {
        let sanitized_body = body.replace('\0', "").trim().to_string();

        match serde_json::from_str::<Movie>(&sanitized_body) {
            Ok(movie) => {
                let movie_json = json!(movie);
                let path = format!("movies/");
                match firebase.at(&path).set(&movie_json).await {
                    Ok(_) => {
                        let status_line = "HTTP/1.0 201 CREATED";
                        let contents = "Movie created";
                        return format!(
                            "{}\r\nContent-Length: {}\r\n\r\n{}",
                            status_line,
                            contents.len(),
                            contents
                        );
                    }
                    Err(e) => {
                        println!("Error setting movie in Firebase: {:?}", e);
                    }
                }
            }
            Err(e) => {
                println!("Failed to parse movie JSON: {:?}", e);
            }
        }
    } else {
        println!("Failed to extract body from request");
    }

    handle_400()
}

async fn handle_put_movies(request: &str) -> String {
    println!("Received PUT request: {}", request);

    let firebase = Firebase::new(FIREBASE_URL).unwrap();
    let body = request
        .split("\r\n\r\n")
        .nth(1)
        .unwrap_or("")
        .replace('\0', "")
        .trim()
        .to_string();

    match serde_json::from_str::<serde_json::Value>(&body) {
        Ok(movie_update) => {
            if let Some(id) = movie_update.get("id").and_then(|id| id.as_str()) {
                let path = format!("movies/{}", id);
                match firebase.at(&path).update(&movie_update).await {
                    Ok(_) => {
                        let status_line = "HTTP/1.0 200 OK";
                        let contents = "Movie updated";
                        return format!(
                            "{}\r\nContent-Length: {}\r\n\r\n{}",
                            status_line,
                            contents.len(),
                            contents
                        );
                    }
                    Err(e) => {
                        println!("Error updating movie in Firebase: {:?}", e);
                    }
                }
            } else {
                println!("Movie ID not provided in the request");
            }
        }
        Err(e) => {
            println!("Failed to parse movie JSON: {:?}", e);
        }
    }

    handle_400()
}

async fn handle_delete_movies(request: &str) -> String {
    println!("Received DELETE request: {}", request);

    let firebase = Firebase::new(FIREBASE_URL).unwrap();
    let body = request
        .split("\r\n\r\n")
        .nth(1)
        .unwrap_or("")
        .replace('\0', "")
        .trim()
        .to_string();

    match serde_json::from_str::<serde_json::Value>(&body) {
        Ok(movie) => {
            if let Some(id) = movie.get("id").and_then(|id| id.as_str()) {
                let path = format!("movies/{}", id);
                match firebase.at(&path).delete().await {
                    Ok(_) => {
                        let status_line = "HTTP/1.0 200 OK";
                        let contents = "Movie deleted";
                        return format!(
                            "{}\r\nContent-Length: {}\r\n\r\n{}",
                            status_line,
                            contents.len(),
                            contents
                        );
                    }
                    Err(e) => {
                        println!("Error deleting movie in Firebase: {:?}", e);
                    }
                }
            } else {
                println!("Movie ID not provided in the request");
            }
        }
        Err(e) => {
            println!("Failed to parse movie JSON: {:?}", e);
        }
    }

    handle_400()
}

async fn handle_get_actors() -> String {
    let firebase = Firebase::new(FIREBASE_URL).unwrap();
    let actors_result = firebase.at("actors").get::<serde_json::Value>().await;

    let status_line;
    let contents;

    match actors_result {
        Ok(actors) => {
            if actors.is_null() {
                status_line = "HTTP/1.0 200 OK";
                contents = "[]".to_string(); // Return an empty list if no actors are found
            } else {
                status_line = "HTTP/1.0 200 OK";
                contents = actors.to_string();
            }
        }
        Err(_) => {
            status_line = "HTTP/1.0 500 INTERNAL SERVER ERROR";
            contents = "Failed to retrieve actors".to_string();
        }
    }

    format!(
        "{}\r\nContent-Length: {}\r\n\r\n{}",
        status_line,
        contents.len(),
        contents
    )
}

async fn handle_post_actors(request: &str) -> String {
    println!("Received POST request: {}", request);

    let firebase = Firebase::new(FIREBASE_URL).unwrap();

    if let Some(body) = request.split("\r\n\r\n").nth(1) {
        let sanitized_body = body.replace('\0', "").trim().to_string();

        match serde_json::from_str::<Actor>(&sanitized_body) {
            Ok(actor) => {
                let actor_json = json!(actor);
                let path = format!("actors/");
                match firebase.at(&path).set(&actor_json).await {
                    Ok(_) => {
                        let status_line = "HTTP/1.0 201 CREATED";
                        let contents = "Actor created";
                        return format!(
                            "{}\r\nContent-Length: {}\r\n\r\n{}",
                            status_line,
                            contents.len(),
                            contents
                        );
                    }
                    Err(e) => {
                        println!("Error setting actor in Firebase: {:?}", e);
                    }
                }
            }
            Err(e) => {
                println!("Failed to parse actor JSON: {:?}", e);
            }
        }
    } else {
        println!("Failed to extract body from request");
    }

    handle_400()
}

async fn handle_put_actors(request: &str) -> String {
    println!("Received PUT request: {}", request);

    let firebase = Firebase::new(FIREBASE_URL).unwrap();
    let body = request
        .split("\r\n\r\n")
        .nth(1)
        .unwrap_or("")
        .replace('\0', "")
        .trim()
        .to_string();

    match serde_json::from_str::<serde_json::Value>(&body) {
        Ok(actor_update) => {
            if let Some(id) = actor_update.get("id").and_then(|id| id.as_str()) {
                let path = format!("actors/{}", id);
                match firebase.at(&path).update(&actor_update).await {
                    Ok(_) => {
                        let status_line = "HTTP/1.0 200 OK";
                        let contents = "Actor updated";
                        return format!(
                            "{}\r\nContent-Length: {}\r\n\r\n{}",
                            status_line,
                            contents.len(),
                            contents
                        );
                    }
                    Err(e) => {
                        println!("Error updating actor in Firebase: {:?}", e);
                    }
                }
            } else {
                println!("Actor ID not provided in the request");
            }
        }
        Err(e) => {
            println!("Failed to parse actor JSON: {:?}", e);
        }
    }

    handle_400()
}

async fn handle_delete_actors(request: &str) -> String {
    println!("Received DELETE request: {}", request);

    let firebase = Firebase::new(FIREBASE_URL).unwrap();
    let body = request
        .split("\r\n\r\n")
        .nth(1)
        .unwrap_or("")
        .replace('\0', "")
        .trim()
        .to_string();

    match serde_json::from_str::<serde_json::Value>(&body) {
        Ok(actor) => {
            if let Some(id) = actor.get("id").and_then(|id| id.as_str()) {
                let path = format!("actors/{}", id);
                match firebase.at(&path).delete().await {
                    Ok(_) => {
                        let status_line = "HTTP/1.0 200 OK";
                        let contents = "Actor deleted";
                        return format!(
                            "{}\r\nContent-Length: {}\r\n\r\n{}",
                            status_line,
                            contents.len(),
                            contents
                        );
                    }
                    Err(e) => {
                        println!("Error deleting actor in Firebase: {:?}", e);
                    }
                }
            } else {
                println!("Actor ID not provided in the request");
            }
        }
        Err(e) => {
            println!("Failed to parse actor JSON: {:?}", e);
        }
    }

    handle_400()
}

async fn handle_get_reviews() -> String {
    let firebase = Firebase::new(FIREBASE_URL).unwrap();
    let reviews_result = firebase.at("reviews").get::<serde_json::Value>().await;

    let status_line;
    let contents;

    match reviews_result {
        Ok(reviews) => {
            if reviews.is_null() {
                status_line = "HTTP/1.0 200 OK";
                contents = "[]".to_string(); // Return an empty list if no reviews are found
            } else {
                status_line = "HTTP/1.0 200 OK";
                contents = reviews.to_string();
            }
        }
        Err(_) => {
            status_line = "HTTP/1.0 500 INTERNAL SERVER ERROR";
            contents = "Failed to retrieve reviews".to_string();
        }
    }

    format!(
        "{}\r\nContent-Length: {}\r\n\r\n{}",
        status_line,
        contents.len(),
        contents
    )
}

async fn handle_post_reviews(request: &str) -> String {
    println!("Received POST request: {}", request);

    let firebase = Firebase::new(FIREBASE_URL).unwrap();

    if let Some(body) = request.split("\r\n\r\n").nth(1) {
        let sanitized_body = body.replace('\0', "").trim().to_string();

        match serde_json::from_str::<Review>(&sanitized_body) {
            Ok(review) => {
                let review_json = json!(review);
                let path = format!("reviews/");
                match firebase.at(&path).set(&review_json).await {
                    Ok(_) => {
                        let status_line = "HTTP/1.0 201 CREATED";
                        let contents = "Review created";
                        return format!(
                            "{}\r\nContent-Length: {}\r\n\r\n{}",
                            status_line,
                            contents.len(),
                            contents
                        );
                    }
                    Err(e) => {
                        println!("Error setting review in Firebase: {:?}", e);
                    }
                }
            }
            Err(e) => {
                println!("Failed to parse review JSON: {:?}", e);
            }
        }
    } else {
        println!("Failed to extract body from request");
    }

    handle_400()
}

async fn handle_put_reviews(request: &str) -> String {
    println!("Received PUT request: {}", request);

    let firebase = Firebase::new(FIREBASE_URL).unwrap();
    let body = request
        .split("\r\n\r\n")
        .nth(1)
        .unwrap_or("")
        .replace('\0', "")
        .trim()
        .to_string();

    match serde_json::from_str::<serde_json::Value>(&body) {
        Ok(review_update) => {
            if let Some(id) = review_update.get("id").and_then(|id| id.as_str()) {
                let path = format!("reviews/{}", id);
                match firebase.at(&path).update(&review_update).await {
                    Ok(_) => {
                        let status_line = "HTTP/1.0 200 OK";
                        let contents = "Review updated";
                        return format!(
                            "{}\r\nContent-Length: {}\r\n\r\n{}",
                            status_line,
                            contents.len(),
                            contents
                        );
                    }
                    Err(e) => {
                        println!("Error updating review in Firebase: {:?}", e);
                    }
                }
            } else {
                println!("Review ID not provided in the request");
            }
        }
        Err(e) => {
            println!("Failed to parse review JSON: {:?}", e);
        }
    }

    handle_400()
}

async fn handle_delete_reviews(request: &str) -> String {
    println!("Received DELETE request: {}", request);

    let firebase = Firebase::new(FIREBASE_URL).unwrap();
    let body = request
        .split("\r\n\r\n")
        .nth(1)
        .unwrap_or("")
        .replace('\0', "")
        .trim()
        .to_string();

    match serde_json::from_str::<serde_json::Value>(&body) {
        Ok(review) => {
            if let Some(id) = review.get("id").and_then(|id| id.as_str()) {
                let path = format!("reviews/{}", id);
                match firebase.at(&path).delete().await {
                    Ok(_) => {
                        let status_line = "HTTP/1.0 200 OK";
                        let contents = "Review deleted";
                        return format!(
                            "{}\r\nContent-Length: {}\r\n\r\n{}",
                            status_line,
                            contents.len(),
                            contents
                        );
                    }
                    Err(e) => {
                        println!("Error deleting review in Firebase: {:?}", e);
                    }
                }
            } else {
                println!("Review ID not provided in the request");
            }
        }
        Err(e) => {
            println!("Failed to parse review JSON: {:?}", e);
        }
    }

    handle_400()
}

fn handle_400() -> String {
    let status_line = "HTTP/1.0 400 BAD REQUEST";
    let contents = "400 - Bad Request";
    format!(
        "{}\r\nContent-Length: {}\r\n\r\n{}",
        status_line,
        contents.len(),
        contents
    )
}

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
