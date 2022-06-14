use common::ChatMessage;
use common::HeartbeatData;
use common::ClientConnectionData;
use reqwest::{Client, Response};
use std::io::stdin;
use std::{thread, time};

use tokio::sync::mpsc;

use futures::StreamExt;
use tokio::io::AsyncBufReadExt;
use tokio_stream::wrappers::ReceiverStream;
use tokio_tungstenite::connect_async;
use tungstenite::protocol::Message as TungsteniteMsg;
use futures::SinkExt;


async fn keep_alive(addr: &str, user_name : String)
{
	let heartbeat_data  = HeartbeatData {alive_user_name:user_name}; 
	loop
	{	
		let data_str = serde_json::to_string(&heartbeat_data).expect("Parsing failed");
		
        // TODO: info on status != 200
        let _resp =  Client::new()
			.post(addr.to_string() + "/heartbeat")
			.body(data_str)
			.send()
			.await
			.expect("Reqwest failed");


        
		thread::sleep(time::Duration::from_millis(2000));
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
    let nickname = get_line("Enter a nickname:");
    println!("Your nickname is \"{}\"", &nickname);


    let connect_addr = "ws://127.0.0.1:8000/ws";
    let (mut ws_stream, _) = connect_async(connect_addr)
        .await
        .expect("Failed to connect to the WS server");

    println!("Connected!");

    let (tx_stdin, rx) = mpsc::channel::<String>(1);
    let mut rx = ReceiverStream::new(rx); // <-- this
    

    let stdin_loop = async move {
        loop {
            let mut line = String::new();
            let mut buf_stdin = tokio::io::BufReader::new(tokio::io::stdin());

            buf_stdin.read_line(&mut line).await.unwrap();
            tx_stdin.send(line.trim().to_string()).await.unwrap();
        }
    };
    tokio::task::spawn(stdin_loop);
    tokio::task::spawn(keep_alive("http://0.0.0.0:8080",nickname.clone()));

    
    let user_data = ClientConnectionData {connecting_user_name:String::from(&nickname)}; 
    
    let _ = ws_stream.send(TungsteniteMsg::Text(serde_json::to_string(&user_data).unwrap())).await;

    loop {
        tokio::select! {

            // TODO: ogarnac to
            ws_msg = ws_stream.next() => {
                match ws_msg {
                    Some(msg) => match msg {
                        Ok(msg) => match msg {
                            // Tu mozna zmienic na from_str() poprostu
                            TungsteniteMsg::Text(json_str) => {
                                let mut msg = serde_json::from_slice::<ChatMessage>(json_str.as_bytes()).unwrap();

                                if msg.author == nickname {
                                    
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

                        let msg = common::ChatMessage::new(&nickname, &msg);
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
