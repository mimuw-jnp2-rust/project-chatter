use async_std::task;
use common::{ChatMessage};
use reqwest::{Client, Response, Url};
use std::{io::stdin, str::FromStr};

use futures::{SinkExt, StreamExt};
use tokio::{io::AsyncBufReadExt, sync::mpsc};
use tokio_tungstenite::connect_async;
use tungstenite::protocol::Message as TungsteniteMsg;

#[derive(Debug)]
enum Party {
    Sender = 0,
    Displayer = 1,
}

impl FromStr for Party {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "--displayer" => Ok(Party::Displayer),
            "--sender" => Ok(Party::Sender),
            _ => Err(()),
        }
    }
}

/* Gets line from the standard input with a `prompt` and error-checks.*/
fn get_line(prompt: &str) -> String {
    println!("{}", prompt);
    let mut line = String::new();
    stdin().read_line(&mut line).expect("Failed to read line");
    line.trim().to_string()
}

/**
* Sends a `msg` with a timestamp to the given `addr` by the `client`.
* @return: a future of the response
*/
async fn send_msg(addr: &str, msg: common::ChatMessage) -> Result<Response, anyhow::Error> {
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
    println!("Press CTRL + C to exit.");
    println!();
}

/* TODO */
fn sender_start() {
    let nickname = get_line("Enter a nickname:");

    println!("Your nickname is {}\n", &nickname);
    loop {
        let input = get_line("Enter a message:");
        task::block_on(async {
            let msg = common::ChatMessage::new(&nickname, &input);
            let response = send_msg("http://0.0.0.0:8080", msg).await;

            // Print err on send failure -> fails only on request fail, does not read the response!
            let status = response.expect("Failed to send message").status();
            println!("Message sent successfully. Server code: {}", status);
        });
    }
}



/* TODO */
async fn displayer_start() {
    println!("Starting displayer...");

    let connect_addr = "ws://127.0.0.1:8000/ws";
    let (mut ws_stream, _) = connect_async(connect_addr)
        .await
        .expect("Failed to connect to the WS server");

    println!("WS server handshake established");

    loop {
        tokio::select!  {
            ws_msg = ws_stream.next() => {
                match ws_msg {
                    Some(msg) => match msg {
                        Ok(msg) => match msg {
                            TungsteniteMsg::Text(json_str) => {
                                let msg = serde_json::from_slice::<ChatMessage>(json_str.as_bytes()).unwrap();
                                println!("{}", msg);
                            }
                            _ => {println!("Received an invalid type of message");}
                        }
                        Err(_) => {println!("WS server went away"); break;}
                    }
                    None => {println!("No message"); break;}
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    greeting();

    if args.len() < 2 {
        panic!("Not enough arguments");
    }

    match Party::from_str(args[1].trim()).unwrap() {
        Party::Displayer => {
            let _ = tokio::spawn(displayer_start()).await;
            //displayer_start();
        }
        Party::Sender => {
            //handle = tokio::spawn(sender_start());
            sender_start();
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(1, 1);
    }
}
