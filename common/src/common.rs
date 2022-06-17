use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};
use chrono::{DateTime, Utc};

#[derive(Serialize, Deserialize)]
pub struct ChatMessage {
    pub author: String,
    pub contents: String,
    pub room: String,
    pub timestamp: DateTime<Utc>,
}

impl Display for ChatMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}: {}", self.timestamp, self.author, self.contents)
    }
}

impl ChatMessage {
    pub fn new(author: &str, contents: &str, room: &Room) -> ChatMessage {
        ChatMessage {
            author: author.to_string(),
            contents: contents.to_string(),
            room: room.name.clone(),
            timestamp: Utc::now()
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct HeartbeatData {
    pub alive_user_name: String,
}

#[derive(Serialize, Deserialize)]
pub struct ClientConnectionData {
    pub connecting_user_name: String,
}

#[derive(Serialize, Deserialize)]
pub struct Room {
    pub name: String,
    pub members: Vec<String>,
}


