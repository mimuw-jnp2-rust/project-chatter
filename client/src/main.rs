use common::Message;
use async_std::task;
use reqwest::{Response, Client};
use std::{io::stdin, str::FromStr};

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
async fn send_msg(addr: &str, msg: common::Message) -> Result<Response, anyhow::Error> {
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

/* */
fn senders_start() {

    let nickname = get_line("Enter a nickname:");
	
    println!("Your nickname is {}",&nickname);
    loop{
        let input = get_line("Enter a message:");
        task::block_on(async {
            let msg = common::Message::new(&nickname, &input);
            let response = send_msg("http://0.0.0.0:8080", msg).await;
            
            // Print err on send failure -> fails only on request fail, does not read the response!
	    let status = response.expect("Msg failed to be send").status();
	    println!("Sent - Server code: {}",status);
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
            senders_start();
            Ok(())
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