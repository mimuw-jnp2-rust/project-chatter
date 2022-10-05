use std::collections::hash_map::Entry;
use std::fmt::Display;

use common::{ChatMessage, ReqData, Room};
use hyper::StatusCode;
use uuid::Uuid;
use warp::Reply;

use crate::{Context, Response, ResultWS, SERVER_SIGNATURE, ws};
use crate::AppState;
use crate::Arc;
use crate::Mutex;
use crate::utils::log_msg;

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

pub async fn registration_handler(ws: warp::ws::Ws, app: Arc<Mutex<AppState>>) -> ResultWS<impl Reply> {
    Ok(ws.on_upgrade(move |socket| ws::new_client_connection(socket, app)))
}

pub async fn login_handler(mut ctx: Context) -> Response {
    match ctx.body_json().await {
        Err(e) => bad_json_resp(e),
        Ok(v) => match v {
            ReqData::LoginData(username) => {
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
            },
            _ => bad_json_resp("Invalid login request received"),
        }
    }
}

pub async fn create_room_handler(mut ctx: Context) -> Response {
    match ctx.body_json().await {
        Err(e) => bad_json_resp(e),
        Ok(v) => match v {
            ReqData::CreateRoomData(room_name) => {
                let room_uuid = Uuid::new_v4();
                //TODO: check if room already exists, respond with an Option<Uuid>
                ctx.app_state.clone().lock().unwrap().rooms.insert(
                    room_uuid, Room::new(&*room_name)
                );

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
            _ => bad_json_resp("Invalid room creation request received"),
        }
    }
}

pub async fn get_room_handler(mut ctx: Context) -> Response {
    match ctx.body_json().await {
        Err(e) => bad_json_resp(e),
        Ok(v) => match v {
            ReqData::GetRoomData(room_name) => {
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
            },
            _ => bad_json_resp("Invalid room creation request received"),
        }
    }
}

pub async fn join_room_handler(mut ctx: Context) -> Response {
    match ctx.body_json().await {
        Err(e) => bad_json_resp(e),
        Ok(v) => match v {
            ReqData::JoinRoomData(user_name, user_uuid, room_uuid) => {
                let mut success = false;
                if let Some(room) = ctx.app_state.clone().lock().unwrap().rooms.get_mut(&room_uuid) {
                    room.add_user(user_uuid);
                    success = true;
                }
                if success {
                    let hello_msg = format!("{} has joined the chat", &user_name);
                    let hello_msg = ChatMessage::new(SERVER_SIGNATURE, &hello_msg);
                    ctx.app_state.lock().unwrap().send_to_room(&hello_msg, room_uuid);
                }
                match serde_json::to_string(&success) {
                    Err(e) => bad_json_resp(e),
                    Ok(success) =>
                        hyper::Response::builder()
                            .status(StatusCode::OK)
                            .header("success", success)
                            .body("OK".into())
                            .unwrap()
                }
            },
            _ => bad_json_resp("Invalid room joining request received"),
        }
    }
}

pub async fn send_msg_handler(mut ctx: Context) -> Response {
    //TODO: think of adding the room name to `SendMsgData` to create a file with that name
    match ctx.body_json().await {
        Err(e) => bad_json_resp(e),
        Ok(v) => match v {
            ReqData::SendMsgData(msg, room_uuid) => {
                println!("{}", msg);
                log_msg(&msg, room_uuid).expect(&*format!("Error logging message for room {}", room_uuid));
                ctx.app_state.clone().lock().unwrap()
                    .send_to_room(&msg, room_uuid);
                ok_resp()
            },
            _ => bad_json_resp("Invalid message sending request received")
        }
    }
}

pub async fn leave_room_handler(mut ctx: Context) -> Response {
    match ctx.body_json().await {
        Err(e) => bad_json_resp(e),
        Ok(v) => match v {
            ReqData::LeaveRoomData(user_uuid, room_uuid) => {
                ctx.app_state.clone().lock().unwrap()
                    .disconnect_user_from_one(user_uuid, room_uuid);
                ok_resp()
            },
            _ => bad_json_resp("Invalid room leaving request received")
        }
    }
}

pub async fn exit_app_handler(mut ctx: Context) -> Response {
    match ctx.body_json().await {
        Err(e) => bad_json_resp(e),
        Ok(v) => match v {
            ReqData::ExitAppData(user_uuid) => {
                ctx.app_state.clone().lock().unwrap()
                    .disconnect_user_from_all(user_uuid);
                ok_resp()
            },
            _ => bad_json_resp("Invalid app exit request received")
        }
    }
}

pub async fn heartbeat_handler(mut ctx: Context) -> Response {
    match ctx.body_json().await {
        Err(e) => bad_json_resp(e),
        Ok(v) => match v {
            ReqData::HeartbeatData(user_uuid) => {
                match ctx.app_state.clone().lock().unwrap().clients.entry(user_uuid) {
                    Entry::Occupied(mut entry) => {
                        println!("Received heartbeat from {}: {}", user_uuid,entry.get().username);
                        entry.get_mut().is_alive = true;
                        ok_resp()
                    },
                    Entry::Vacant(_) => not_found_resp()
                }
            }
            _ => bad_json_resp("Invalid heartbeat request received"),
        }
    }
}

