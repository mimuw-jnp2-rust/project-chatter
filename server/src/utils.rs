use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;

use common::ChatMessage;
use uuid::Uuid;

const APP_DIR: &str = ".chatter";
const LOGS_DIR: &str = "logs";
//const USERS_DATA_DIR: &str = "users_data";
//const ROOMS_DATA_DIR: &str = "rooms_data";

fn app_dir_path() -> PathBuf {
    dirs::home_dir()
        .expect("Cannot locate user's home directory")
        .join(APP_DIR)
}

fn logs_dir_path() -> PathBuf {
    app_dir_path().join(LOGS_DIR)
}

fn room_log_path(room_uuid: Uuid) -> PathBuf {
    logs_dir_path().join(room_uuid.to_string())
}

pub fn setup_app_dir() -> io::Result<()> {
    let app_dir_path = app_dir_path();
    if !app_dir_path.exists() {
        println!("Creating app directory under {:?}", &app_dir_path);
        fs::create_dir(app_dir_path)?;
    } else {
        println!("Located app directory under {:?}", app_dir_path);
    }

    let room_logs_path = logs_dir_path();
    if !room_logs_path.exists() {
        fs::create_dir(room_logs_path)?;
    }

    Ok(())
}

pub fn log_msg(msg: &ChatMessage, room_uuid: Uuid) -> io::Result<()> {
    let path = room_log_path(room_uuid).with_extension(".log");
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{}", msg)?;
    Ok(())
}