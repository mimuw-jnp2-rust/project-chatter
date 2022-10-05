use std::{thread};
use std::io::{stdin};
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use reqwest::{Client as ReqwestClient, Response};
use tokio::io::AsyncBufReadExt;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tungstenite::protocol::Message as TungsteniteMsg;
use uuid::Uuid;
use JNP2_Rust_Chatter::common::{*, ReqData::*};

const ADDR: &str = "http://0.0.0.0:8080";
const WS_ADDR: &str = "ws://127.0.0.1:8000/ws";
const EXIT_COMMAND: &str = "/exit"; // exits the entire app
const LEAVE_COMMAND: &str = "/leave"; // goes back to the lobby

type WSStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

fn print_greeting() {
    println!("==========================");
    println!("=   Welcome to Chatter   =");
    println!("==========================");
    println!();
}

fn get_line(prompt: &str) -> Result<String, std::io::Error> {
    println!("{}", prompt);
    let mut line = String::new();
    match stdin().read_line(&mut line) {
        Ok(_) => return Ok(line.trim().to_string()),
        Err(err) => Err(err),
    }
}

fn get_nonempty_line(what: &str) -> String {
    let prompt = format!("Enter {}", what);
    let invalid = format!("Invalid {}. Please try again", what);
    loop {
        match get_line(&*prompt) {
            Ok(res) =>
                if res.is_empty() || res == SERVER_SIGNATURE {
                    println!("{}", invalid);
                } else {
                    break res;
                }
            Err(err) => println!("Error: {}", err)
        }
    }
}

async fn request_login(
    client_name: &str,
    reqwest_client: &ReqwestClient,
) -> anyhow::Result<Option<Uuid>> {
    let data = LoginData(ClientName(client_name.to_string()));
    let data = serde_json::to_string(&data)?;
    let resp = reqwest_client
        .post(ADDR.to_string() + LOGIN_ENDPOINT)
        .body(data)
        .send()
        .await?;
    let client_uuid = resp
        .headers()
        .get(CLIENT_UUID_HEADER)
        .expect("No client_uuid header in server response");
    Ok(serde_json::from_slice(client_uuid.as_bytes())?)
}

async fn request_registration(
    client_name: &str,
    reqwest_client: &ReqwestClient,
    ws_stream: &mut WSStream,
) -> Uuid {
    let fail_msg = "Error in registration!";
    let client_data = RegistrationData(ClientName(client_name.to_string()));
    let client_data = serde_json::to_string(&client_data).unwrap();
    ws_stream
        .send(TungsteniteMsg::Text(client_data))
        .await
        .expect(fail_msg);
    let uuid = request_login(client_name, reqwest_client)
        .await
        .expect(fail_msg)
        .expect(fail_msg);
    uuid
}

async fn request_get_room(
    room_name: &str,
    reqwest_client: &ReqwestClient,
) -> anyhow::Result<Option<Uuid>> {
    let data = GetRoomData(RoomName(room_name.to_string()));
    let data = serde_json::to_string(&data)?;
    let resp = reqwest_client
        .post(ADDR.to_string() + GET_ROOM_ENDPOINT)
        .body(data)
        .send()
        .await?;
    let room_uuid = resp
        .headers()
        .get(ROOM_UUID_HEADER)
        .expect("No room_uuid header in server response");
    Ok(serde_json::from_slice(room_uuid.as_bytes())?)
}

async fn request_create_room(
    room_name: &str,
    reqwest_client: &ReqwestClient,
) -> anyhow::Result<Uuid> {
    let data = CreateRoomData(RoomName(room_name.to_string()));
    let data = serde_json::to_string(&data)?;
    let resp = reqwest_client
        .post(ADDR.to_string() + CREATE_ROOM_ENDPOINT)
        .body(data)
        .send()
        .await?;
    let room_uuid = resp
        .headers()
        .get(ROOM_UUID_HEADER)
        .expect("No room_uuid header in server response");
    Ok(serde_json::from_slice(room_uuid.as_bytes())?)
}

async fn request_join_room(
    client_uuid: Uuid,
    client_name: &str,
    room_uuid: Uuid,
    reqwest_client: &ReqwestClient,
) -> anyhow::Result<bool> {
    let data = JoinRoomData(ClientName(client_name.to_string()), ClientUuid(client_uuid), RoomUuid(room_uuid));
    let data = serde_json::to_string(&data)?;
    let resp = reqwest_client
        .post(ADDR.to_string() + JOIN_ROOM_ENDPOINT)
        .body(data)
        .send()
        .await?;
    let success = resp
        .headers()
        .get(SUCCESS_HEADER)
        .expect("No success header in server response");
    Ok(serde_json::from_slice(success.as_bytes())?)
}

async fn login(reqwest_client: &ReqwestClient, ws_stream: &mut WSStream) -> (String, Uuid) {
    let client_name = get_nonempty_line("username");
    let client_uuid = match request_login(&client_name, reqwest_client)
        .await
        .expect("Error during login")
    {
        Some(uuid) => {
            println!("Welcome back, {}", &client_name);
            uuid
        }
        None => {
            println!("Nice to meet you, {}", &client_name);
            request_registration(&client_name, reqwest_client, ws_stream).await
        }
    };
    (client_name, client_uuid)
}

async fn get_room(reqwest_client: &ReqwestClient) -> (String, Uuid) {
    let room_name = get_nonempty_line("room name");
    let room_uuid = match request_get_room(&room_name, reqwest_client)
        .await
        .expect("Error finding room")
    {
        Some(uuid) => uuid,
        None => {
            println!("Created room '{}'", &room_name);
            request_create_room(&room_name, reqwest_client)
                .await
                .expect("Error creating room")
        }
    };
    (room_name, room_uuid)
}

async fn join_room(
    client_uuid: Uuid,
    client_name: &str,
    room_uuid: Uuid,
    room_name: &str,
    reqwest_client: &ReqwestClient,
) {
    let success = request_join_room(client_uuid, client_name, room_uuid, reqwest_client)
        .await
        .expect("Error joining room");
    if success {
        println!("Joined room '{}'", room_name);
    } else {
        panic!("Error joining room");
    }
}

async fn send_msg(
    reqwest_client: &ReqwestClient,
    msg: ChatMessage,
    room_uuid: Uuid,
) -> anyhow::Result<Response> {
    let data = SendMsgData(msg, RoomUuid(room_uuid));
    let data = serde_json::to_string(&data)?;
    let resp = reqwest_client
        .post(ADDR.to_string() + SEND_MSG_ENDPOINT)
        .body(data)
        .send()
        .await?;
    Ok(resp)
}

async fn leave_room(
    reqwest_client: &ReqwestClient,
    client_uuid: Uuid,
    room_uuid: Uuid,
) -> anyhow::Result<Response> {
    let data = LeaveRoomData(RoomUuid(room_uuid), ClientUuid(client_uuid));
    let data = serde_json::to_string(&data)?;
    let resp = reqwest_client
        .post(ADDR.to_string() + LEAVE_ROOM_ENDPOINT)
        .body(data)
        .send()
        .await?;
    Ok(resp)
}

async fn exit_app(reqwest_client: &ReqwestClient, client_uuid: Uuid) -> anyhow::Result<Response> {
    let data = ExitAppData(ClientUuid(client_uuid));
    let data = serde_json::to_string(&data)?;
    let resp = reqwest_client
        .post(ADDR.to_string() + EXIT_APP_ENDPOINT)
        .body(data)
        .send()
        .await?;
    Ok(resp)
}

async fn keep_alive(client_uuid: Uuid) {
    const HEARTBEAT_TIMEOUT: u64 = 1000;
    let heartbeat_data = HeartbeatData(ClientUuid(client_uuid));
    let client = ReqwestClient::new();
    loop {
        thread::sleep(Duration::from_millis(HEARTBEAT_TIMEOUT));
        let data_str = serde_json::to_string(&heartbeat_data).expect("Parsing heartbeat failed");

        // TODO: info on status != 200
        let _resp = client
            .post(ADDR.to_string() + HEARTBEAT_ENDPOINT)
            .body(data_str)
            .send()
            .await
            .expect("Heartbeat request failed");
    }
}

async fn chat_client() {
    print_greeting();
    let reqwest_client = ReqwestClient::new();
    let (mut ws_stream, _) = connect_async(WS_ADDR)
        .await
        .expect("Failed to connect to the WS server");

    let (client_name, client_uuid) = login(&reqwest_client, &mut ws_stream).await; //TODO: check if client already exists, maybe check for a passwd
    let _ = tokio::task::spawn(keep_alive(client_uuid));

    // the lobby loop - select your room here
    loop {
        let (room_name, room_uuid) = get_room(&reqwest_client).await;
        let _ = join_room(
            client_uuid,
            &client_name,
            room_uuid,
            &room_name,
            &reqwest_client,
        )
        .await; //TODO: dodać exitowanie z lobby, nie tylko z pokoju

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
        let stdin_loop = tokio::task::spawn(stdin_loop);

        loop {
            tokio::select! {
                ws_msg = ws_stream.next() => {
                    match ws_msg {
                        Some(msg) => match msg {
                            Ok(msg) => match msg {
                                TungsteniteMsg::Text(json_str) => {
                                    let mut msg = serde_json::from_str::<ChatMessage>(&json_str).unwrap();
                                    if msg.author == client_name {
                                        msg.author = String::from("YOU");
                                    }
                                    println!("{}", msg);
                                }
                                _ => {eprintln!("Received an invalid type of message");}
                            }
                            Err(_) => {eprintln!("WS server went away"); return;}
                        }
                        None => {eprintln!("No message"); return;}
                    }
                },
                stdin_msg = rx.next() => {
                    match stdin_msg {
                        Some(msg) => {
                            let msg = ChatMessage::new(&client_name, &msg);
                            let mut should_break = false;
                            let mut should_return = false;
                            let response;

                            if msg.contents == EXIT_COMMAND {
                                response = exit_app(&reqwest_client, client_uuid).await;
                                ws_stream.close(None);
                                should_return = true;
                            } else if msg.contents == LEAVE_COMMAND {
                                response = leave_room(&reqwest_client, client_uuid, room_uuid).await;
                                should_break = true;
                            } else {
                                response = send_msg(&reqwest_client, msg, room_uuid).await;
                            }

                            let status = response.expect("Failed to send message").status(); //TODO: expect, eprintln ->>> logging
                            if should_break {
                                break;
                            }
                            if should_return {
                                return;
                            }
                        },
                        None => return
                    }
                }
            }
        }
        stdin_loop.abort(); // end listening for messages on this room
    }
}

#[tokio::main]
async fn main() {
    chat_client().await;
}
