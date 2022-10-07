use std::collections::hash_map::Entry;
use std::fmt::Display;

use chatter::common::{
    ChatMessage, ReqData, Room, RoomUuid, CLIENT_UUID_HEADER, ROOM_UUID_HEADER, SUCCESS_HEADER,
};
use hyper::StatusCode;
use uuid::Uuid;
use warp::Reply;

use crate::logging::log_msg;
use crate::AppState;
use crate::Arc;
use crate::Mutex;
use crate::{ws, Context, Response, ResultWS, SERVER_SIGNATURE};

fn bad_json_resp(err: impl Display) -> Response {
    hyper::Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .body(format!("could not parse JSON: {}", err).into())
        .unwrap()
}

pub fn response_with_code(code: StatusCode) -> Response {
    let reason = code
        .canonical_reason()
        .map_or("".to_string(), |reason| format!(": {}", reason));
    hyper::Response::builder()
        .status(code)
        .body((code.to_string() + &*reason).into())
        .unwrap()
}

pub async fn handle_health_check(ctx: Context) -> Response {
    let app = ctx.app_state.lock().unwrap();
    hyper::Response::builder()
        .status(StatusCode::OK)
        .body(format!("I am {} and I am alive!", app.name).into())
        .unwrap()
}

pub async fn handle_registration(
    ws: warp::ws::Ws,
    app: Arc<Mutex<AppState>>,
) -> ResultWS<impl Reply> {
    Ok(ws.on_upgrade(move |socket| ws::new_client_connection(socket, app)))
}

fn response_with_header<T>(value: &T, header: &str) -> Response
where
    T: ?Sized + serde::Serialize,
{
    match serde_json::to_string(value) {
        Err(e) => bad_json_resp(e),
        Ok(value_str) => hyper::Response::builder()
            .status(StatusCode::OK)
            .header(header, value_str)
            .body("200: Ok".into())
            .unwrap(),
    }
}

async fn request<F>(mut ctx: Context, f: F, request_type: &str) -> Response
where
    F: Fn(ReqData) -> Result<StatusCode, ()>,
{
    match ctx.body_json::<ReqData>().await {
        Err(e) => bad_json_resp(e),
        Ok(v) => match f(v) {
            Ok(code) => response_with_code(code),
            Err(_) => bad_json_resp(format!("Invalid {} request received", request_type)),
        },
    }
}

async fn request_with_header<F, H>(
    mut ctx: Context,
    f: F,
    header: &str,
    request_type: &str,
) -> Response
where
    F: Fn(ReqData) -> Result<H, ()>,
    H: serde::Serialize,
{
    match ctx.body_json::<ReqData>().await {
        Err(e) => bad_json_resp(e),
        Ok(v) => match f(v) {
            Ok(v) => response_with_header(&v, header),
            Err(_) => bad_json_resp(format!("Invalid {} request received", request_type)),
        },
    }
}

pub async fn handle_login(ctx: Context) -> Response {
    let app_state = ctx.app_state.clone();
    let f = |req_data| match req_data {
        ReqData::LoginData(client_name) => {
            let client_uuid = app_state.lock().unwrap().clients.iter().find_map(|(k, v)| {
                if v.name == client_name {
                    Some(*k)
                } else {
                    None
                }
            });
            Ok(client_uuid)
        }
        _ => Err(()),
    };
    request_with_header(ctx, f, CLIENT_UUID_HEADER, "login").await
}

pub async fn handle_create_room(ctx: Context) -> Response {
    let app_state = ctx.app_state.clone();
    let f = |req_data| match req_data {
        ReqData::CreateRoomData(room_name) => {
            let room_uuid = RoomUuid(Uuid::new_v4());
            app_state
                .lock()
                .unwrap()
                .rooms
                .insert(room_uuid, Room::new(&*room_name.0));
            Ok(room_uuid)
        }
        _ => Err(()),
    };
    request_with_header(ctx, f, ROOM_UUID_HEADER, "create_room").await
}

pub async fn handle_get_room(ctx: Context) -> Response {
    let app_state = ctx.app_state.clone();
    let f = |req_data| match req_data {
        ReqData::GetRoomData(room_name) => {
            let room_uuid = app_state.lock().unwrap().rooms.iter().find_map(|(k, v)| {
                if v.name == room_name {
                    Some(*k)
                } else {
                    None
                }
            });
            Ok(room_uuid)
        }
        _ => Err(()),
    };
    request_with_header(ctx, f, ROOM_UUID_HEADER, "get_room").await
}

pub async fn handle_join_room(ctx: Context) -> Response {
    let app_state = ctx.app_state.clone();
    let f = |req_data| match req_data {
        ReqData::JoinRoomData(client_name, client_uuid, room_uuid) => {
            let mut success = false;
            if let Some(room) = app_state.lock().unwrap().rooms.get_mut(&room_uuid) {
                room.add(client_uuid);
                success = true;
            }
            if success {
                let hello_msg = format!("{} has joined the chat", &client_name.0);
                let hello_msg = ChatMessage::new(SERVER_SIGNATURE, &hello_msg);
                app_state
                    .lock()
                    .unwrap()
                    .send_to_room(&hello_msg, room_uuid);
            }
            Ok(success)
        }
        _ => Err(()),
    };
    request_with_header(ctx, f, SUCCESS_HEADER, "join_room").await
}

pub async fn handle_send_msg(ctx: Context) -> Response {
    let app_state = ctx.app_state.clone();
    let f = |req_data| match req_data {
        ReqData::SendMsgData(msg, room_uuid) => {
            println!("{}", msg);
            if log_msg(&msg, room_uuid).is_err() {
                eprintln!("Error logging message for room {}", room_uuid.0);
            }
            app_state.lock().unwrap().send_to_room(&msg, room_uuid);
            Ok(StatusCode::OK)
        }
        _ => Err(()),
    };
    request(ctx, f, "send_msg").await
}

pub async fn handle_leave_room(ctx: Context) -> Response {
    let app_state = ctx.app_state.clone();
    let f = |req_data| match req_data {
        ReqData::LeaveRoomData(room_uuid, client_uuid) => {
            app_state
                .lock()
                .unwrap()
                .disconnect_client_from_one(client_uuid, room_uuid);
            Ok(StatusCode::OK)
        }
        _ => Err(()),
    };
    request(ctx, f, "leave_room").await
}

pub async fn handle_exit_app(ctx: Context) -> Response {
    let app_state = ctx.app_state.clone();
    let f = |req_data| match req_data {
        ReqData::ExitAppData(client_uuid) => {
            app_state
                .lock()
                .unwrap()
                .disconnect_client_from_all(client_uuid);
            Ok(StatusCode::OK)
        }
        _ => Err(()),
    };
    request(ctx, f, "exit_app").await
}

pub async fn handle_heartbeat(ctx: Context) -> Response {
    let app_state = ctx.app_state.clone();
    let f = |req_data| match req_data {
        ReqData::HeartbeatData(client_uuid) => {
            match app_state.lock().unwrap().clients.entry(client_uuid) {
                Entry::Occupied(mut entry) => {
                    println!(
                        "Received heartbeat from {} ({})",
                        client_uuid.0,
                        entry.get().name.0
                    );
                    entry.get_mut().is_alive = true;
                    Ok(StatusCode::OK)
                }
                Entry::Vacant(_) => Ok(StatusCode::NOT_FOUND),
            }
        }
        _ => Err(()),
    };
    request(ctx, f, "heartbeat").await
}
