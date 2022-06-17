use std::{thread, time};
use std::io::stdin;

use common::{ChatMessage, ClientConnectionData, HeartbeatData};
use futures::{SinkExt, StreamExt};
use reqwest::{Client as ReqwestClient, Response};
use tokio::io::AsyncBufReadExt;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_tungstenite::connect_async;
use tungstenite::protocol::Message as TungsteniteMsg;
use uuid::Uuid;

const ADDR: &str = "http://0.0.0.0:8080";
const WS_ADDR: &str = "ws://127.0.0.1:8000/ws";

async fn keep_alive(user_uuid: Uuid) {
    const HEARTBEAT_TIMEOUT: u64 = 2000;
    let heartbeat_data = HeartbeatData::new(user_uuid);
    let client = ReqwestClient::new();
    loop {
        thread::sleep(time::Duration::from_millis(HEARTBEAT_TIMEOUT));
        let data_str = serde_json::to_string(&heartbeat_data).expect("Parsing heartbeat failed");

        // TODO: info on status != 200
        let _resp = client
            .post(ADDR.to_string() + "/heartbeat")
            .body(data_str)
            .send()
            .await
            .expect("Heartbeat request failed");
    }
}

fn get_line(prompt: &str) -> String {
    println!("{}", prompt);
    let mut line = String::new();
    stdin().read_line(&mut line).expect("Failed to read line");
    return line.trim().to_string();
}

async fn get_uuids(username: &str, room_name: &str) -> anyhow::Result<(Uuid, Uuid)> {
    let client = ReqwestClient::new();
    let data = serde_json::to_string(&(username, room_name))?;
    Ok(client
        .post(ADDR.to_string() + "/start")
        .body(data)
        .send()
        .await?
        .json::<(Uuid, Uuid)>()
        .await?)
}

async fn send_msg(msg: ChatMessage, room_uuid: Uuid) -> anyhow::Result<Response> {
    let client = ReqwestClient::new();
    let data = serde_json::to_string(&(msg, room_uuid))?;
    Ok(client
        .post(ADDR.to_string() + "/send_msg")
        .body(data)
        .send()
        .await?)
}

fn greeting() {
    println!("==========================");
    println!("=   Welcome to Chatter   =");
    println!("==========================");
    println!();
}

#[tokio::main]
async fn main() {
    greeting();

    let mut username;
    let mut room_name;
    let (user_uuid, room_uuid): (Uuid, Uuid) = loop {
        username = get_line("Enter a username:");
        room_name = get_line("Enter room:");
        if let Ok((u, r)) = get_uuids(&username, &room_name).await {
            break (u, r);
        } else {
            println!("Please try again.");
        }
    };

    let (mut ws_stream, _) = connect_async(WS_ADDR)
        .await
        .expect("Failed to connect to the WS server");
    println!("Connected to room {}!", room_name);

    let (tx_stdin, rx) = mpsc::channel::<String>(1);
    let mut rx = ReceiverStream::new(rx);
    let stdin_loop = async move {
        loop {
            let mut line = String::new();
            let mut buf_stdin = tokio::io::BufReader::new(tokio::io::stdin());

            buf_stdin.read_line(&mut line).await.unwrap();
            tx_stdin.send(line.trim().to_string()).await.unwrap();
        }
    };
    tokio::task::spawn(stdin_loop);
    tokio::task::spawn(keep_alive(user_uuid));

    let user_data = ClientConnectionData::new(user_uuid);
    let _ = ws_stream
        .send(TungsteniteMsg::Text(
            serde_json::to_string(&user_data).unwrap(),
        ))
        .await;

    loop {
        tokio::select! {
            ws_msg = ws_stream.next() => {
                match ws_msg {
                    Some(msg) => match msg {
                        Ok(msg) => match msg {
                            TungsteniteMsg::Text(json_str) => {
                                let mut msg = serde_json::from_str::<ChatMessage>(&json_str).unwrap();
                                if msg.author == username {
                                    msg.author = String::from("YOU");
                                }
                                println!("{}", msg);
                            }
                            _ => {eprintln!("Received an invalid type of message");}
                        }
                        Err(_) => {eprintln!("WS server went away"); break;}
                    }
                    None => {eprintln!("No message"); break;}
                }
            },
            stdin_msg = rx.next() => {
                match stdin_msg {
                    Some(msg) => {
                        let msg = ChatMessage::new(&username, &msg);
                        let response = send_msg(msg, room_uuid).await;
                        // TODO: Print err on send failure -> fails only on request fail, does not read the response!
                        let status = response.expect("Failed to send message").status();
                        println!("Message sent successfully. Server code: {}", status);
                    },
                    None => break
                }
            }
        }
    }
}
