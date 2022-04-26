use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Serialize,Deserialize};



#[derive(Serialize, Deserialize)]
pub struct Message {
    pub author: String,
    pub contents: String,
    pub timestamp: u64,
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
                .as_secs()
        }
    }
}
