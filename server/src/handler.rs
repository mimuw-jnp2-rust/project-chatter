use std::fmt::Display;

use common::{ChatMessage, Room};
use hyper::StatusCode;
use uuid::Uuid;
use warp::Reply;

use crate::{Context, Response, ResultWS, ws};
use crate::AppState;
use crate::Arc;
use crate::Mutex;

fn bad_json_resp(err: impl Display) -> Response {
    hyper::Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .body(format!("could not parse JSON: {}", err).into())
        .unwrap()
}

fn ok_resp() -> Response {
    hyper::Response::builder()
        .status(StatusCode::OK)
        .body("OK".into())
        .unwrap()
}

pub async fn start_handler(mut ctx: Context) -> Response {
    let (username, room_name): (String, String) = match ctx.body_json().await {
        Ok(v) => v,
        Err(e) => return bad_json_resp(e),
    };

    let user_uuid = ctx
        .app_state
        .clone()
        .lock()
        .unwrap()
        .clients_map
        .iter_mut()
        .find_map(|(k, mut v)| match v.username.as_mut() {
            None => {
                v.username = Some(username.clone());
                Some(*k)
            }
            Some(uname) => {
                if *uname == username {
                    Some(*k)
                } else {
                    None
                }
            }
        });

    //TODO: add macros for these long chain calls...
    let room_uuid = ctx
        .app_state
        .clone()
        .lock()
        .unwrap()
        .rooms_map
        .iter()
        .find_map(|(k, v)| {
            if v.name == room_name {
                Some(*k)
            } else {
                None
            }
        });
    if room_uuid.is_none() {
        ctx.app_state
            .clone()
            .lock()
            .unwrap()
            .rooms_map
            .insert(Uuid::new_v4(), Room::new(&room_name));
    }

    match serde_json::to_string(&user_uuid) {
        Ok(user_uuid) => match serde_json::to_string(&room_uuid) {
            Ok(room_uuid) => {
                //TODO: test this out
                hyper::Response::builder()
                    .status(StatusCode::OK)
                    .header("user_uuid", user_uuid)
                    .header("room_uuid", room_uuid)
                    .body("OK".into())
                    .unwrap()
            }
            Err(e) => bad_json_resp(e),
        },
        Err(e) => bad_json_resp(e),
    }
}

pub async fn not_found_handler(_ctx: Context) -> Response {
    hyper::Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body("404: NOT FOUND".into())
        .unwrap()
}

pub async fn test_handler(ctx: Context) -> Response {
    let app = ctx.app_state.lock().unwrap();

    hyper::Response::builder()
        .status(StatusCode::OK)
        .body(format!("I am {} and I am alive!", app.name).into())
        .unwrap()
}

pub async fn send_msg_handler(mut ctx: Context) -> Response {
    let (received_msg, room_uuid): (ChatMessage, Uuid) = match ctx.body_json().await {
        Ok(v) => v,
        Err(e) => return bad_json_resp(e),
    };

    println!("{}", received_msg);

    ctx.app_state
        .clone()
        .lock()
        .unwrap()
        .send_to_room(&received_msg, room_uuid);

    ok_resp()
}

pub async fn heartbeat_handler(mut ctx: Context) -> Response {
    let received_heartbeat: common::HeartbeatData = match ctx.body_json().await {
        Ok(v) => v,
        Err(e) => return bad_json_resp(e),
    };

    let app_state = ctx.app_state.clone();

    if !app_state
        .lock()
        .unwrap()
        .clients_map
        .contains_key(&received_heartbeat.user_uuid)
    {
        eprintln!("Invalid user uuid: {}", received_heartbeat.user_uuid);
    } else {
        app_state
            .lock()
            .unwrap()
            .clients_map
            .get_mut(&received_heartbeat.user_uuid)
            .unwrap()
            .is_alive = true;
    }

    ok_resp()
}

pub async fn ws_handler(ws: warp::ws::Ws, app: Arc<Mutex<AppState>>) -> ResultWS<impl Reply> {
    Ok(ws.on_upgrade(move |socket| ws::client_connection(socket, app)))
}
