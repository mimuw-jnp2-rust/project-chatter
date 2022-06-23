use std::fs::{self, File};
use std::io;
use std::path::PathBuf;

use common::ChatMessage;
use uuid::Uuid;

const APP_DIR: &str = "chatter";
const ROOM_LOGS_DIR: &str = "room_logs";
//const USERS_DATA_DIR: &str = "users_data";
//const ROOMS_DATA_DIR: &str = "rooms_data";

pub fn setup_app_dir() -> io::Result<()> {
    let app_dir_path = dirs::home_dir()
        .ok_or("Cannot locate user's home directory")?
        .join(APP_DIR);
    if !app_dir_path.exists() {
        fs::create_dir(app_dir_path)?;
    }

    let room_logs_path = app_dir_path.join(ROOM_LOGS_DIR);
    if !room_logs_path.exists() {
        fs::create_dir(room_logs_path)?;
    }

    Ok(())
}



pub fn log_msg(msg: &ChatMessage, room_uuid: Uuid) {
    let mut file = File::open("");

    if let Err(e) = writeln!(file, "{}", line) {
        eprintln!("Couldn't write to file: {}", e);
    }
}