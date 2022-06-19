use std::{thread, time};
use std::future::Future;
use std::io::stdin;

use common::{ChatMessage, Data};
use futures::SinkExt;
use reqwest::{Client as ReqwestClient, Response};
use tokio::io::AsyncBufReadExt;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tungstenite::protocol::Message as TungsteniteMsg;
use uuid::Uuid;

const ADDR: &str = "http://0.0.0.0:8080";
const WS_ADDR: &str = "ws://127.0.0.1:8000/ws";

type WSStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

fn greeting() {
    println!("==========================");
    println!("=   Welcome to Chatter   =");
    println!("==========================");
    println!();
}

fn get_line(prompt: &str) -> String {
    println!("{}", prompt);
    let mut line = String::new();
    stdin().read_line(&mut line).expect("Failed to read line");
    return line.trim().to_string();
}

fn get_nonempty_line(what: &str) -> String {
    let prompt = format!("Enter {}", what);
    let invalid = format!("Invalid {}. Please try again", what);
    loop {
        let res = get_line(&prompt);
        if !res.is_empty() {
            break res;
        } else {
            println!("{}", invalid)
        }
    }
}

async fn get_user_uuid(reqwest_client: &ReqwestClient) -> (String, Option<Uuid>) {
    let username = get_nonempty_line("username");
    match try_get_user_uuid(reqwest_client, &username).await {
        Ok(user_uuid) => (username, user_uuid),
        Err(e) => {
            panic!("Error while fetching user's UUID: {}", e)
        }
    }
}

async fn get_room_uuid(reqwest_client: &ReqwestClient) -> (String, Option<Uuid>) {
    let room_name = get_nonempty_line("room name");
    match try_get_room_uuid(reqwest_client, &room_name).await {
        Ok(room_uuid) => (room_name, room_uuid),
        Err(e) => {
            panic!("Error while fetching room's UUID: {}", e)
        }
    }
}

async fn try_get_user_uuid(reqwest_client: &ReqwestClient, username: &str) -> anyhow::Result<Option<Uuid>> {
    let data = serde_json::to_string(username)?;
    let resp = reqwest_client
        .post(ADDR.to_string() + "/login")
        .body(data)
        .send()
        .await?;
    let user_uuid = resp.headers().get("user_uuid").expect("No user_uuid header in server response");
    Ok(serde_json::from_slice(user_uuid.as_bytes())?)
}

async fn try_get_room_uuid(reqwest_client: &ReqwestClient, room_name: &str) -> anyhow::Result<Option<Uuid>> {
    let data = serde_json::to_string(room_name)?;
    let resp = reqwest_client
        .post(ADDR.to_string() + "/pick_room")
        .body(data)
        .send()
        .await?;
    let room_uuid = resp.headers().get("room_uuid").expect("No room_uuid header in server response");
    Ok(serde_json::from_slice(room_uuid.as_bytes())?)
}

async fn login(reqwest_client: &ReqwestClient, ws_stream: &mut WSStream) {
    let fail_msg = "Login failed!";
    let (username, user_uuid) = get_user_uuid(&reqwest_client).await;
    if user_uuid.is_none() {
        println!("Nice to meet you, {}", &username);
        let user_data = Data::NewClientData(username.clone());
        let user_data = serde_json::to_string(&user_data).unwrap();
        ws_stream.send(TungsteniteMsg::Text(user_data)).await.expect(fail_msg);
    } else {
        println!("Welcome back, {}", &username);
    }
    let user_uuid = try_get_user_uuid(&reqwest_client, &username).await.expect(fail_msg).expect(fail_msg);
    tokio::task::spawn(keep_alive(user_uuid));
}

async fn join_room(reqwest_client: &ReqwestClient, ws_stream: &mut WSStream) {
    let fail_msg = "Joining room failed!";
    let (room_name, room_uuid) = get_room_uuid(&reqwest_client).await;
    if room_uuid.is_none() {
        println!("Created room {}", &room_name);
        let room_data = Data::NewRoomData(room_name.clone());
        let room_data = serde_json::to_string(&room_data).unwrap();
        ws_stream.send(TungsteniteMsg::Text(room_data)).await.expect(fail_msg);
        println!("HELLO");
    } else {
        println!("Joined room {}", &room_name);
    }
    let room_uuid = try_get_room_uuid(&reqwest_client, &room_name).await;
    println!("RESULT {:?}", room_uuid)
    //.expect(fail_msg).expect(fail_msg);
}

async fn send_msg(reqwest_client: &ReqwestClient, msg: ChatMessage, room_uuid: Uuid) -> anyhow::Result<Response> {
    let data = serde_json::to_string(&(msg, room_uuid))?;
    Ok(reqwest_client
        .post(ADDR.to_string() + "/send_msg")
        .body(data)
        .send()
        .await?)
}

async fn keep_alive(user_uuid: Uuid) {
    const HEARTBEAT_TIMEOUT: u64 = 2000;
    let heartbeat_data = Data::HeartbeatData(user_uuid);
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

#[tokio::main]
async fn main() {
    greeting();

    let reqwest_client = ReqwestClient::new();
    let (mut ws_stream, _) = connect_async(WS_ADDR)
        .await
        .expect("Failed to connect to the WS server");

    login(&reqwest_client, &mut ws_stream).await;
    join_room(&reqwest_client, &mut ws_stream).await;

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


