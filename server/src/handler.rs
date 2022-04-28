use crate::{Context, Response, ws, ResultWS};
use hyper::StatusCode;
use warp::Reply;
use crate::common::Message;
use crate::Arc;
use crate::Mutex;
use crate::AppState;

pub async fn test_handler(ctx: Context) -> String {
	
    let app = ctx.state.lock().unwrap();
    format!("test called, state_thing was: {}, counter value: {}", app.name, app.counter)
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

    let sent_str = serde_json::to_string(&received_msg).unwrap();

    // TODO SEND MSG TO ALL CLIENTS HERE, EXTRACT SENDING OBJ FROM ctx.state !
    for (user,connection) in &ctx.state.clone().lock().unwrap().ws_clients {

        let splash_msg = Ok(warp::ws::Message::text( sent_str.clone() ));
        let _ = connection.sender.as_ref().unwrap().send(splash_msg);
    }
    // ctx.state.lock(). i tutaj teges

    return hyper::Response::builder()
        .status(StatusCode::OK)
        .body("OK".into())
        .unwrap();
}


pub async fn ws_handler(ws: warp::ws::Ws, app: Arc<Mutex<AppState>>) -> ResultWS<impl Reply> {
    println!("ws_handler");
    Ok(ws.on_upgrade(move |socket| ws::client_connection(socket,app)))
}
