use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize)]
pub struct Message {
    pub author: String,
    pub contents: String,
    pub timestamp: u64,
}

impl Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", 1, 2)
    }
}

impl Message {
    // Used only in client
    #[allow(dead_code)]
    pub fn new(author: &str, contents: &str) -> Message {
        Message {
            author: author.to_string(),
            contents: contents.to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Clock may have gone backwards")
                .as_secs(),
        }
    }
}
