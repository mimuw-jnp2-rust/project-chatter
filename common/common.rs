use std::time::{SystemTime, UNIX_EPOCH};
use chrono::{DateTime};

pub struct Message {
    pub author: String,
    pub contents: String,
    pub timestamp: u64,
}

impl Message {
    pub fn new(author: &str, contents: &str) -> Message {
        Message {
            author: author.to_string(),
            contents: contents.to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Clock may have gone backwards")
                .as_secs()
        }
    }
    pub fn to_get_string(&self, addr: &str) -> String {
        format!("{}{}{}{}{}{}{}{}", addr, "/send?",
                "&time=", self.timestamp,
                "&author=", self.author,
                "&msg=", self.contents,
        )
    }
}