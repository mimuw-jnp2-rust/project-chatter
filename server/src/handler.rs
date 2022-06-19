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

pub async fn login_handler(mut ctx: Context) -> Response {
    let username: String = match ctx.body_json().await {
        Err(e) => return bad_json_resp(e),
        Ok(v) => v,
    };

    let mut user_is_new = false;

    let mut user_uuid : Option<Uuid> = None;
    for (k, v) in &mut ctx.app_state.clone().lock().unwrap().clients {
        match &v.username {
            None => { // claim this entry
                user_is_new = true;
                v.username = Some(username.clone());
                user_uuid = Some(*k);
                break;
            }
            Some(name) => {
                if *name == username {  // we've found our previously put entry
                    user_uuid = Some(*k);
                    break;
                }
            }
        }
    }

    println!("{}", user_uuid.unwrap());

    match serde_json::to_string(&user_uuid) {
        Err(e) => bad_json_resp(e),
        Ok(user_uuid) => match serde_json::to_string(&user_is_new) {
            Err(e) => bad_json_resp(e),
            Ok(user_is_new) => {
                let resp =
                hyper::Response::builder()
                    .status(StatusCode::OK)
                    .header("user_uuid", user_uuid)
                    .header("user_is_new", user_is_new)
                    .body("OK".into())
                    .unwrap();
                println!("{:?}", resp.headers());
                resp
            }
        },
    }
}

pub async fn pick_room_handler(mut ctx: Context) -> Response {
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

    let room_is_new = room_uuid.is_none();
    if room_is_new {
        ctx.app_state.clone().lock().unwrap().rooms
            .insert(Uuid::new_v4(), Room::new(&room_name));
    }

    match serde_json::to_string(&room_uuid) {
        Err(e) => bad_json_resp(e),
        Ok(room_uuid) => match serde_json::to_string(&room_is_new) {
            Err(e) => bad_json_resp(e),
            Ok(user_is_new) => {
                let resp =
                hyper::Response::builder()
                    .status(StatusCode::OK)
                    .header("room_uuid", room_uuid)
                    .header("room_is_new", user_is_new)
                    .body("OK".into())
                    .unwrap();
                println!("{:?}", resp.headers());
                resp
            }
        },
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
        Err(e) => return bad_json_resp(e),
        Ok(v) => v,
    };

    let app_state = ctx.app_state.clone();

    if !app_state
        .lock()
        .unwrap()
        .clients
        .contains_key(&received_heartbeat.user_uuid)
    {
        eprintln!("Invalid user uuid: {}", received_heartbeat.user_uuid);
    } else {
        app_state
            .lock()
            .unwrap()
            .clients
            .get_mut(&received_heartbeat.user_uuid)
            .unwrap()
            .is_alive = true;
    }

    ok_resp()
}

pub async fn ws_handler(ws: warp::ws::Ws, app: Arc<Mutex<AppState>>) -> ResultWS<impl Reply> {
    Ok(ws.on_upgrade(move |socket| ws::client_connection(socket, app)))
}
