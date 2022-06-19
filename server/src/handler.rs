use std::collections::hash_map::Entry;
use std::fmt::Display;

use common::{ChatMessage, Data, Room};
use hyper::StatusCode;
use uuid::Uuid;
use warp::{head, Reply};

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

fn not_found_resp() -> Response {
    hyper::Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body("404: NOT FOUND".into())
        .unwrap()
}

pub async fn new_client_handler(ws: warp::ws::Ws, app: Arc<Mutex<AppState>>) -> ResultWS<impl Reply> {
    Ok(ws.on_upgrade(move |socket| ws::new_client_connection(socket, app)))
}

pub async fn login_handler(mut ctx: Context) -> Response {
    let username: String = match ctx.body_json().await {
        Err(e) => return bad_json_resp(e),
        Ok(v) => v,
    };
    let user_uuid = ctx.app_state.clone().lock().unwrap().clients.iter()
        .find_map(|(k, v)| {
            if v.username == username {
                Some(*k)
            } else {
                None
            }
        });
    match serde_json::to_string(&user_uuid) {
        Err(e) => bad_json_resp(e),
        Ok(user_uuid) =>
            hyper::Response::builder()
                .status(StatusCode::OK)
                .header("user_uuid", user_uuid)
                .body("OK".into())
                .unwrap()
    }
}

pub async fn new_room_handler(mut ctx: Context) -> Response {
    let room_name: String = match ctx.body_json().await {
        Err(e) => return bad_json_resp(e),
        Ok(v) => v,
    };

    //TODO: check if room already exists
    ctx.app_state.clone().lock().unwrap().rooms.insert(
        Uuid::new_v4(), Room::new(&*room_name)
    );

    match serde_json::to_string(&room_uuid) {
        Err(e) => bad_json_resp(e),
        Ok(room_uuid) =>
            hyper::Response::builder()
                .status(StatusCode::OK)
                .body("OK".into())
                .unwrap()
    }
}

pub async fn join_room_handler(mut ctx: Context) -> Response {
    let room_name: String = match ctx.body_json().await {
        Err(e) => return bad_json_resp(e),
        Ok(v) => v,
    };
    let room_uuid = ctx.app_state.clone().lock().unwrap().rooms.iter()
        .find_map(|(k, v)| {
            if v.name == room_name {
                Some(*k)
            } else {
                None
            }
        });
    match serde_json::to_string(&room_uuid) {
        Err(e) => bad_json_resp(e),
        Ok(room_uuid) =>
            hyper::Response::builder()
                .status(StatusCode::OK)
                .header("room_uuid", room_uuid)
                .body("OK".into())
                .unwrap()
    }
}

pub async fn not_found_handler(_ctx: Context) -> Response {
    not_found_resp()
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

    ctx.app_state.clone().lock().unwrap()
        .send_to_room(&received_msg, room_uuid);
    ok_resp()
}

pub async fn heartbeat_handler(mut ctx: Context) -> Response {
    match ctx.body_json().await {
        Err(e) => bad_json_resp(e),
        Ok(v) => {
            match v {
                Data::HeartbeatData(user_uuid) => {
                    return match ctx.app_state.clone().lock().unwrap().clients.entry(user_uuid) {
                        Entry::Occupied(mut entry) => {
                            entry.get_mut().is_alive = true;
                            ok_resp()
                        },
                        Entry::Vacant(_) => not_found_resp()
                    }
                }
                _ => bad_json_resp("Invalid heartbeat received"),
            }
        }
    }
}

