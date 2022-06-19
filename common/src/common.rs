use std::collections::HashSet;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

type WSSender = UnboundedSender<Result<warp::ws::Message, warp::Error>>;

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

#[derive(Serialize, Deserialize)]
pub struct HeartbeatData {
    pub user_uuid: Uuid,
}

impl HeartbeatData {
    pub fn new(user_uuid: Uuid) -> Self {
        HeartbeatData {
            user_uuid,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ClientConnectionData {
    pub user_uuid: Uuid,
    pub room_uuid: Uuid,
}

impl ClientConnectionData {
    pub fn new(user_uuid: Uuid, room_uuid: Uuid) -> Self {
        ClientConnectionData {
            user_uuid,
            room_uuid,
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

    pub fn remove_user(&mut self, user_uuid: Uuid) {
        self.members.remove(&user_uuid);
    }
}

pub struct Client {
    pub is_alive: bool,
    pub username: Option<String>,
    pub sender: WSSender,
}

impl Client {
    pub fn new(sender: WSSender) -> Self {
        Client {
            is_alive: true,
            username: None,
            sender,
        }
    }
}
