use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt::{self, Display};
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

type WSSender = UnboundedSender<Result<warp::ws::Message, warp::Error>>;

pub const CLIENT_UUID_HEADER: &str = "client_uuid";
pub const ROOM_UUID_HEADER: &str = "room_uuid";
pub const SUCCESS_HEADER: &str = "success";
pub const SERVER_SIGNATURE: &str = "SERVER";

pub const HEALTH_CHECK_ENDPOINT: &str = "/health_check";
pub const SEND_MSG_ENDPOINT: &str = "/send_msg";
pub const LEAVE_ROOM_ENDPOINT: &str = "/leave_room";
pub const EXIT_APP_ENDPOINT: &str = "/exit_app";
pub const LOGIN_ENDPOINT: &str = "/login";
pub const GET_ROOM_ENDPOINT: &str = "/get_room";
pub const CREATE_ROOM_ENDPOINT: &str = "/create_room";
pub const JOIN_ROOM_ENDPOINT: &str = "/join_room";
pub const HEARTBEAT_ENDPOINT: &str = "/heartbeat";

pub const ADDR_HTTP: &str = "127.0.0.1:8080";
pub const ADDR_WS:   &str = "127.0.0.1:8000";
pub const LOCALHOST: &str = "127.0.0.1";

pub const PORT_HTTP: &str = ":8080";
pub const PORT_WS:   &str = ":8000";


#[derive(Serialize, Deserialize)]
pub struct ClientUuid(pub Uuid);
#[derive(Serialize, Deserialize)]
pub struct RoomUuid(pub Uuid);
#[derive(Serialize, Deserialize)]
pub struct ClientName(pub String);
#[derive(Serialize, Deserialize)]
pub struct RoomName(pub String);

#[derive(Serialize, Deserialize)]
pub enum ReqData {
    HeartbeatData(ClientUuid),
    CreateRoomData(RoomName),
    GetRoomData(RoomName),
    JoinRoomData(ClientName, ClientUuid, RoomUuid),
    SendMsgData(ChatMessage, RoomUuid),
    LoginData(ClientName),
    RegistrationData(ClientName),
    LeaveRoomData(RoomUuid, ClientUuid),
    ExitAppData(ClientUuid),
}

#[derive(Serialize, Deserialize)]
pub struct ChatMessage {
    pub author: String,
    pub contents: String,
    pub timestamp: DateTime<Utc>,
}

impl Display for ChatMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}: {}", self.timestamp, self.author, self.contents)
    }
}

impl ChatMessage {
    pub fn new(author: &str, contents: &str) -> ChatMessage {
        ChatMessage {
            author: author.to_string(),
            contents: contents.to_string(),
            timestamp: Utc::now(),
        }
    }
}

pub struct Room {
    pub name: String,
    pub uuid: Uuid,
    pub members: HashSet<Uuid>,
}

impl Room {
    pub fn new(name: &str) -> Self {
        Room {
            name: name.to_string(),
            uuid: Uuid::new_v4(),
            members: HashSet::new(),
        }
    }

    pub fn add_client(&mut self, client_uuid: Uuid) {
        self.members.insert(client_uuid);
    }

    pub fn remove_client(&mut self, client_uuid: Uuid) {
        self.members.remove(&client_uuid);
    }
}

pub struct Client {
    pub is_alive: bool,
    pub name: String,
    pub sender: WSSender,
}

impl Client {
    pub fn new(sender: WSSender, name: &str) -> Self {
        Client {
            is_alive: true,
            name: name.to_string(),
            sender,
        }
    }
}
pub enum Protocol {
    HTTP,WS
}

pub fn get_addr_str(prot : Protocol) -> String{
    let args: Vec<String> = std::env::args().collect();

    let addr = if args.len() == 1 {
        LOCALHOST.to_string()
    } else {
        args[1].clone()
    };

    match prot {
        Protocol::HTTP => addr + PORT_HTTP,
        Protocol::WS =>   addr + PORT_WS,
    }
}
