use crate::{Context, Response};
use hyper::StatusCode;
use serde::{Serialize,Deserialize};

pub async fn test_handler(ctx: Context) -> String {
    format!("test called, state_thing was: {}", ctx.state.state_thing)
}

#[derive(Serialize,Deserialize)]
pub struct Message {
    pub author: String,
    pub contents: String,
    pub timestamp: u64,
}
pub async fn send_handler(mut ctx: Context) -> Response {
    let body: Message = match ctx.body_json().await {
        Ok(v) => v,
        Err(e) => {
            println!("FAIL");
            return hyper::Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(format!("could not parse JSON: {}", e).into())
                .unwrap()
        }
    };

    println!(
        "NEW MSG FROM {}: {} received at {}",
        body.author, body.contents, body.timestamp 
    );

    Response::new(
        "OK".into(),
    )
}
