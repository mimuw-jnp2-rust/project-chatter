use async_std::{
    prelude::*,
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
async fn send_msg(client: Client, addr: &str, msg: &str) -> Result<Response, Error> {
    let since_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Clock may have gone backwards")
        .as_secs();

    let url = format!("{}{}{}{}{}{}", addr, "/send?",
                      "&time=", since_epoch,
                      "&msg=", msg,
    );

    let resp = client.get(url).send().await?;
    Ok(resp)
}

fn senders_start(iterations: usize) {
    for _ in 0..iterations {
        let input = get_line("Enter a message: ");
        task::block_on(async {
            println!("Message '{}' sent",input);
            let output = send_msg(Client::new(), "https://httpbin.org/", &*input).await;
            println!("{:?}", output);
        });
    }
}

fn displayer_start() {
    println!("Starting displayer...");
    loop {
        println!("display...");
    };
}

fn greeting() {
    println!("=== Welcome to Chatter ===");
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
            //konsole -e /bin/bash --rcfile <(echo "echo hi")
            Command::new( "konsole")
                .arg("-e")
                .arg("/bin/bash")
                .arg("--rcfile")
                .arg("<(echo \"echo hi\"")
                // .arg("--")
                // .arg("cargo")
                // .arg("run")
                // .arg("--")
                // .arg("--display")
                .spawn()?;
            //Command::new("konsole").spawn().expect("");
            senders_start(2);
            Ok(())
        }
    }
}

