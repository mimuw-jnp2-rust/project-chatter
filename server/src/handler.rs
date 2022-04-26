use crate::{Context, Response};
use hyper::StatusCode;

use crate::common::Message;

pub async fn test_handler(ctx: Context) -> String {
    format!("test called, state_thing was: {}", ctx.state.name)
}

pub async fn send_handler(mut ctx: Context) -> Response {
    let received_msg: Message = match ctx.body_json().await {
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
        received_msg.author, received_msg.contents, received_msg.timestamp 
    );

    // TODO SEND MSG TO ALL CLIENTS HERE, EXTRACT SENDING OBJ FROM ctx.state !

    return hyper::Response::builder()
        .status(StatusCode::OK)
        .body("OK".into())
        .unwrap();
}
