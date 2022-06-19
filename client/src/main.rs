use std::{thread, time};
use std::io::stdin;

use common::{ChatMessage, ClientConnectionData, HeartbeatData};
use futures::SinkExt;
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

async fn send_msg(reqwest_client: &ReqwestClient, msg: ChatMessage, room_uuid: Uuid) -> anyhow::Result<Response> {
    let data = serde_json::to_string(&(msg, room_uuid))?;
    Ok(reqwest_client
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

async fn login(reqwest_client: &ReqwestClient) -> (String, String, Uuid, Uuid) {
    loop {
        let username = loop {
            let res = get_line("Enter username:");
            if !res.is_empty() {
                break res;
            }
        };
        let room_name = loop {
            let res = get_line("Enter room name:");
            if !res.is_empty() {
                break res;
            }
        };
        match get_user_uuid(reqwest_client, &username).await {
            Ok(user_uuid) =>  match get_room_uuid(reqwest_client, &room_name).await {
                Ok(room_uuid) => return (username, room_name, user_uuid, room_uuid),
                Err(_) => {
                    println!("Invalid room name. Please try again.");
                }
            },
            Err(_) => {
                println!("Invalid username. Please try again.");
            }
        }
    }
}

async fn get_user_uuid(reqwest_client: &ReqwestClient, username: &str) -> anyhow::Result<Uuid> {
    let data = serde_json::to_string(username)?;
    let resp = reqwest_client
        .post(ADDR.to_string() + "/login")
        .body(data)
        .send()
        .await?;
    let headers = resp.headers();
    for (k, v) in headers {
        println!("({:?}, {:?}", k, v);
    }

    let user_is_new : bool = serde_json::from_slice( headers.get("room_is_new").expect("No field user_is_new in server response").as_bytes())?;
    if user_is_new {
        println!("Nice to meet you, '{}'", username);
    } else {
        println!("Welcome back,  '{}'", username);
    }

    let user_uuid: Uuid = serde_json::from_slice( headers.get("user_uuid").expect("No field user_uuid in server response").as_bytes())?;
    Ok(user_uuid)
}

async fn get_room_uuid(reqwest_client: &ReqwestClient, room_name: &str) -> anyhow::Result<Uuid> {
    let data = serde_json::to_string(room_name)?;
    let resp = reqwest_client
        .post(ADDR.to_string() + "/pick_room")
        .body(data)
        .send()
        .await?;
    let headers = resp.headers();

    let room_is_new : bool = serde_json::from_slice( headers.get("room_is_new").expect("No field room_is_new in server response").as_bytes())?;
    if room_is_new {
        println!("Created room '{}'", room_name);
    }

    let room_uuid: Uuid = serde_json::from_slice( headers.get("room_uuid").expect("No field room_uuid in server response").as_bytes())?;
    Ok(room_uuid)
}

#[tokio::main]
async fn main() {
    greeting();
    let reqwest_client = ReqwestClient::new();

    let (mut ws_stream, _) = connect_async(WS_ADDR)
        .await
        .expect("Failed to connect to the WS server");
    //println!("Connected to websocket");
    let (username, room_name, user_uuid, room_uuid) = login(&reqwest_client).await;

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

    let user_data = ClientConnectionData::new(user_uuid, room_uuid);
    let _ = ws_stream
        .send(TungsteniteMsg::Text(
            serde_json::to_string(&user_data).unwrap(),
        ))
        .await;
    println!("Joined room `{}`", room_name);

   /* loop {
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
                        let response = send_msg(reqwest_client, msg, room_uuid).await;
                        // TODO: Print err on send failure -> fails only on request fail, does not read the response!
                        let status = response.expect("Failed to send message").status();
                        println!("Message sent successfully. Server code: {}", status);
                    },
                    None => break
                }
            }
        }
    }*/
}
