use std::{thread, time};
use std::io::stdin;

use common::{ChatMessage, ClientConnectionData, HeartbeatData};
use futures::{SinkExt, StreamExt};
use reqwest::{Client, Response};
use tokio::io::AsyncBufReadExt;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_tungstenite::connect_async;
use tungstenite::protocol::Message as TungsteniteMsg;
use uuid::Uuid;

async fn keep_alive(addr: &str, username: &str) {
    const HEARTBEAT_TIMEOUT: u64 = 2000;
    let heartbeat_data = HeartbeatData::new(username);
    loop {
        thread::sleep(time::Duration::from_millis(HEARTBEAT_TIMEOUT));

        let data_str = serde_json::to_string(&heartbeat_data).expect("Parsing heartbeat failed");

        // TODO: info on status != 200
        let _resp = Client::new()
            .post(addr.to_string() + "/heartbeat")
            .body(data_str)
            .send()
            .await
            .expect("Reqwest failed");
    }
}

/* Gets line from the standard input with a `prompt` and error-checks.*/
fn get_line(prompt: &str) -> String {
    println!("{}", prompt);
    let mut line = String::new();
    stdin().read_line(&mut line).expect("Failed to read line");
    return line.trim().to_string();
}

/**
* Sends a `msg` with a timestamp to the given `addr` by the `client`.
* @return: a future of the response
*/
async fn send_msg(addr: &str, msg: common::ChatMessage) -> anyhow::Result<Response> {
    let client = Client::new();
    let data = serde_json::to_string(&msg)?;
    let resp = client
        .post(addr.to_string() + "/post")
        .body(data)
        .send()
        .await?;
    Ok(resp)
}

/* Greeting displayed at the start of the application. */
fn greeting() {
    println!("==========================");
    println!("=   Welcome to Chatter   =");
    println!("==========================");
    println!();
}

#[tokio::main]
async fn main() {
    greeting();
    let username = get_line("Enter a username:");
    println!("Your username is \"{}\"", &username);

    let connect_addr = "ws://127.0.0.1:8000/ws";
    let (mut ws_stream, _) = connect_async(connect_addr)
        .await
        .expect("Failed to connect to the WS server");

    println!("Connected!");

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
    tokio::task::spawn(keep_alive("http://0.0.0.0:8080", &username));

    let user_data = ClientConnectionData::new(&*username);

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
                            // Tu mozna zmienic na from_str() poprostu
                            TungsteniteMsg::Text(json_str) => {
                                let mut msg = serde_json::from_slice::<ChatMessage>(json_str.as_bytes()).unwrap();

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
                        let msg = common::ChatMessage::new(&username, &msg, 1);
                        let response = send_msg("http://0.0.0.0:8080", msg).await;

                        // Print err on send failure -> fails only on request fail, does not read the response!
                        let status = response.expect("Failed to send message").status();
                        println!("Message sent successfully. Server code: {}", status);
                    },
                    None => break
                }
            }
        }
    }
}
