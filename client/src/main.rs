mod common;

use crate::common::Message;
use async_std::{
    task,
};
use reqwest::{
    Response,
    Error,
    Client,
};
use std::{
    io::stdin,
    process::Command,
    str::FromStr,
    time::{
        SystemTime,
        UNIX_EPOCH,
    },
};


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
            _ => Err(())
        }
    }
}

/* Gets line from the standard input with a `prompt` and error-checks.*/
fn get_line(prompt: &str) -> String {
    println!("{}", prompt);
    let mut line = String::new();
    stdin().
        read_line(&mut line)
        .expect("Failed to read line");
    line.trim().to_string()
}

/**
* Sends a `msg` with a timestamp to the given `addr` by the `client`.
* @return: a future of the response
*/
async fn send_msg(client: Client, addr: &str, msg: Message) -> Result<Response, anyhow::Error> {
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
    println!("=== Welcome to Chatter ===");
}

/* */
fn senders_start(iterations: usize) {
    for i in 0..iterations {
        let input = get_line("Enter a message: ");
        task::block_on(async {
            println!("Message '{}' sent", input);
            let msg = Message::new(&*format!("sender_{}", i), &input);
            let output = send_msg(Client::new(), "http://0.0.0.0:8080", msg).await;
            println!("{:?}", output);
        });
    }
}

/* */
fn displayer_start() {
    println!("Starting displayer...");
    loop {
        println!("display...");
    };
}


#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let args= std::env::args().collect::<Vec<_>>();
    greeting();

    if args.len() < 2 {
        panic!("Not enough arguments");
    }

    match Party::from_str(args[1].trim()).unwrap() {
        Party::Displayer => {
            displayer_start();
            Ok(())
        }
        Party::Sender => {
            senders_start(2);
            Ok(())
        }
    }
}

