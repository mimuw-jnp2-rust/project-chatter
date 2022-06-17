use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct ChatMessage {
    pub author: String,
    pub contents: String,
    pub room_uuid: Uuid,
    pub timestamp: DateTime<Utc>,
}

impl Display for ChatMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}: {}", self.timestamp, self.author, self.contents)
    }
}

impl ChatMessage {
    pub fn new(author: &str, contents: &str, room_uuid: &Uuid) -> ChatMessage {
        ChatMessage {
            author: author.to_string(),
            contents: contents.to_string(),
            room_uuid: *room_uuid,
            timestamp: Utc::now()
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct HeartbeatData {
    pub client_uuid: Uuid,
}

impl HeartbeatData {
    pub fn new(username: &str) -> HeartbeatData {
        HeartbeatData {
            client_uuid: Default::default()
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ClientConnectionData {
    pub client_uuid: Uuid,
}

impl ClientConnectionData {
    pub fn new(username: &str) -> ClientConnectionData {
        ClientConnectionData {
            client_uuid: Default::default()
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Room {
    pub name: String,
    pub uuid: Uuid,
    pub members: Vec<Uuid>,
}


