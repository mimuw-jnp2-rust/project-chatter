use crate::AppState;
use crate::Arc;
use crate::Mutex;
use crate::{ws, Context, Response, ResultWS};
use hyper::StatusCode;
use warp::Reply;

pub async fn test_handler(ctx: Context) -> String {
    let app = ctx.state.lock().unwrap();
    format!("test called, state_thing was: {}", app.name)
}

pub async fn send_handler(mut ctx: Context) -> Response {
    let received_msg: common::ChatMessage = match ctx.body_json().await {
        Ok(v) => v,
        Err(e) => {
            println!("FAIL");
            return hyper::Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(format!("could not parse JSON: {}", e).into())
                .unwrap();
        }
    };

    println!("{}", received_msg);

    let sent_str = serde_json::to_string(&received_msg).unwrap();

    // TODO SEND MSG TO ALL CLIENTS HERE, EXTRACT SENDING OBJ FROM ctx.state !
    for connection in ctx.state.clone().lock().unwrap().ws_clients.values() {
        let splash_msg = Ok(warp::ws::Message::text(sent_str.clone()));
        let _ = connection.sender.send(splash_msg); //TODO: make this checked
    }

    hyper::Response::builder()
        .status(StatusCode::OK)
        .body("OK".into())
        .unwrap()
}

pub async fn ws_handler(ws: warp::ws::Ws, app: Arc<Mutex<AppState>>) -> ResultWS<impl Reply> {
    println!("ws_handler");
    Ok(ws.on_upgrade(move |socket| ws::client_connection(socket, app)))
}
