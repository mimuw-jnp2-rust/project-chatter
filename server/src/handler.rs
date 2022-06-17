use hyper::StatusCode;
use warp::Reply;

use crate::{Context, Response, ResultWS, ws};
use crate::AppState;
use crate::Arc;
use crate::Mutex;

pub async fn not_found_handler(_ctx: Context) -> Response {
    hyper::Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body("404: NOT FOUND".into())
        .unwrap()
}

pub async fn test_handler(ctx: Context) -> Response {
    let app = ctx.state.lock().unwrap();

    hyper::Response::builder()
        .status(StatusCode::OK)
        .body(format!("I am {} and I am alive!", app.name).into())
        .unwrap()
}

pub async fn send_handler(mut ctx: Context) -> Response {
    let received_msg: common::ChatMessage = match ctx.body_json().await {
        Ok(v) => v,
        Err(e) => {
            return hyper::Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(format!("could not parse JSON: {}", e).into())
                .unwrap();
        }
    };

    println!("{}", received_msg);
    ctx.state.clone().lock().unwrap().send_within_room(&received_msg);

    hyper::Response::builder()
        .status(StatusCode::OK)
        .body("OK".into())
        .unwrap()
}

pub async fn heartbeat_handler(mut ctx: Context) -> Response {
    let received_heartbeat: common::HeartbeatData = match ctx.body_json().await {
        Ok(v) => v,
        Err(e) => {
            return hyper::Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(format!("could not parse JSON: {}", e).into())
                .unwrap();
        }
    };

    let ctx = ctx.state.clone();

    if !ctx
        .lock()
        .unwrap()
        .clients_map
        .contains_key(&received_heartbeat.client_uuid)
    {
        eprintln!("User with uuid {} not found", received_heartbeat.client_uuid);
    } else {
        ctx.lock()
            .unwrap()
            .clients_map
            .get_mut(&received_heartbeat.client_uuid)
            .unwrap()
            .is_alive = true;
    }

    hyper::Response::builder()
        .status(StatusCode::OK)
        .body("OK".into())
        .unwrap()
}

pub async fn ws_handler(ws: warp::ws::Ws, app: Arc<Mutex<AppState>>) -> ResultWS<impl Reply> {
    Ok(ws.on_upgrade(move |socket| ws::client_connection(socket, app)))
}
