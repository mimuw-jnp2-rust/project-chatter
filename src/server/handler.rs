use std::collections::hash_map::Entry;
use std::fmt::Display;

use hyper::StatusCode;
use uuid::Uuid;
use warp::Reply;
use JNP2_Rust_Chatter::common::{
    ChatMessage, ReqData, Room, CLIENT_UUID_HEADER, ROOM_UUID_HEADER, SUCCESS_HEADER,
};

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

fn ok_resp() -> Response {
    hyper::Response::builder()
        .status(StatusCode::OK)
        .body("OK".into())
        .unwrap()
}

pub fn not_found_resp() -> Response {
    hyper::Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body("404: NOT FOUND".into())
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

pub async fn handle_login(mut ctx: Context) -> Response {
    match ctx.body_json().await {
        Err(e) => bad_json_resp(e),
        Ok(v) => match v {
            ReqData::LoginData(client_name) => {
                let client_uuid = ctx
                    .app_state
                    .clone()
                    .lock()
                    .unwrap()
                    .clients
                    .iter()
                    .find_map(|(k, v)| {
                        if v.name == client_name.0 {
                            Some(*k)
                        } else {
                            None
                        }
                    });
                match serde_json::to_string(&client_uuid) {
                    Err(e) => bad_json_resp(e),
                    Ok(client_uuid) => hyper::Response::builder()
                        .status(StatusCode::OK)
                        .header(CLIENT_UUID_HEADER, client_uuid)
                        .body("OK".into())
                        .unwrap(),
                }
            }
            _ => bad_json_resp("Invalid login request received"),
        },
    }
}

pub async fn handle_create_room(mut ctx: Context) -> Response {
    match ctx.body_json().await {
        Err(e) => bad_json_resp(e),
        Ok(v) => match v {
            ReqData::CreateRoomData(room_name) => {
                let room_uuid = Uuid::new_v4();
                ctx.app_state
                    .clone()
                    .lock()
                    .unwrap()
                    .rooms
                    .insert(room_uuid, Room::new(&*room_name.0));

                match serde_json::to_string(&room_uuid) {
                    Err(e) => bad_json_resp(e),
                    Ok(room_uuid) => hyper::Response::builder()
                        .status(StatusCode::OK)
                        .header(ROOM_UUID_HEADER, room_uuid)
                        .body("OK".into())
                        .unwrap(),
                }
            }
            _ => bad_json_resp("Invalid room creation request received"),
        },
    }
}

pub async fn handle_get_room(mut ctx: Context) -> Response {
    match ctx.body_json().await {
        Err(e) => bad_json_resp(e),
        Ok(v) => match v {
            ReqData::GetRoomData(room_name) => {
                let room_uuid =
                    ctx.app_state
                        .clone()
                        .lock()
                        .unwrap()
                        .rooms
                        .iter()
                        .find_map(|(k, v)| {
                            if v.name == room_name.0 {
                                Some(*k)
                            } else {
                                None
                            }
                        });
                match serde_json::to_string(&room_uuid) {
                    Err(e) => bad_json_resp(e),
                    Ok(room_uuid) => hyper::Response::builder()
                        .status(StatusCode::OK)
                        .header(ROOM_UUID_HEADER, room_uuid)
                        .body("OK".into())
                        .unwrap(),
                }
            }
            _ => bad_json_resp("Invalid room creation request received"),
        },
    }
}

pub async fn handle_join_room(mut ctx: Context) -> Response {
    match ctx.body_json().await {
        Err(e) => bad_json_resp(e),
        Ok(v) => match v {
            ReqData::JoinRoomData(client_name, client_uuid, room_uuid) => {
                let mut success = false;
                if let Some(room) = ctx
                    .app_state
                    .clone()
                    .lock()
                    .unwrap()
                    .rooms
                    .get_mut(&room_uuid.0)
                {
                    room.add_client(client_uuid.0);
                    success = true;
                }
                if success {
                    let hello_msg = format!("{} has joined the chat", &client_name.0);
                    let hello_msg = ChatMessage::new(SERVER_SIGNATURE, &hello_msg);
                    ctx.app_state
                        .lock()
                        .unwrap()
                        .send_to_room(&hello_msg, room_uuid.0);
                }
                match serde_json::to_string(&success) {
                    Err(e) => bad_json_resp(e),
                    Ok(success) => hyper::Response::builder()
                        .status(StatusCode::OK)
                        .header(SUCCESS_HEADER, success)
                        .body("OK".into())
                        .unwrap(),
                }
            }
            _ => bad_json_resp("Invalid room joining request received"),
        },
    }
}

pub async fn handle_send_msg(mut ctx: Context) -> Response {
    match ctx.body_json().await {
        Err(e) => bad_json_resp(e),
        Ok(v) => match v {
            ReqData::SendMsgData(msg, room_uuid) => {
                println!("{}", msg);
                log_msg(&msg, room_uuid.0)
                    .expect(&*format!("Error logging message for room {}", room_uuid.0));
                ctx.app_state
                    .clone()
                    .lock()
                    .unwrap()
                    .send_to_room(&msg, room_uuid.0);
                ok_resp()
            }
            _ => bad_json_resp("Invalid message sending request received"),
        },
    }
}

pub async fn handle_leave_room(mut ctx: Context) -> Response {
    match ctx.body_json().await {
        Err(e) => bad_json_resp(e),
        Ok(v) => match v {
            ReqData::LeaveRoomData(room_uuid, client_uuid) => {
                println!("HELLO 2");
                ctx.app_state
                    .clone()
                    .lock()
                    .unwrap()
                    .disconnect_client_from_one(client_uuid.0, room_uuid.0);
                ok_resp()
            }
            _ => bad_json_resp("Invalid room leaving request received"),
        },
    }
}

pub async fn handle_exit_app(mut ctx: Context) -> Response {
    match ctx.body_json().await {
        Err(e) => bad_json_resp(e),
        Ok(v) => match v {
            ReqData::ExitAppData(client_uuid) => {
                ctx.app_state
                    .clone()
                    .lock()
                    .unwrap()
                    .disconnect_client_from_all(client_uuid.0);
                ok_resp()
            }
            _ => bad_json_resp("Invalid app exit request received"),
        },
    }
}

pub async fn handle_heartbeat(mut ctx: Context) -> Response {
    match ctx.body_json().await {
        Err(e) => bad_json_resp(e),
        Ok(v) => match v {
            ReqData::HeartbeatData(client_uuid) => {
                match ctx
                    .app_state
                    .clone()
                    .lock()
                    .unwrap()
                    .clients
                    .entry(client_uuid.0)
                {
                    Entry::Occupied(mut entry) => {
                        println!(
                            "Received heartbeat from {} ({})",
                            client_uuid.0,
                            entry.get().name
                        );
                        entry.get_mut().is_alive = true;
                        ok_resp()
                    }
                    Entry::Vacant(_) => not_found_resp(),
                }
            }
            _ => bad_json_resp("Invalid heartbeat request received"),
        },
    }
}
