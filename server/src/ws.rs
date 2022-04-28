use warp::ws::WebSocket;
use crate::Mutex;
use crate::AppState;
use crate::Arc;
use uuid::Uuid;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use crate::WSClient;
use futures::{FutureExt, StreamExt};

pub async fn client_connection(ws: WebSocket,app: Arc<Mutex<AppState>>) {

    println!("establishing client connection... {:?}", ws);

    let (client_ws_sender, mut client_ws_rcv) = ws.split();

    let (client_sender, client_rcv) = mpsc::unbounded_channel();

    let client_rcv = UnboundedReceiverStream::new(client_rcv);

    // Keep stream open until disconnected
    tokio::task::spawn(client_rcv.forward(client_ws_sender).map(|result| {
        if let Err(e) = result {
            println!("error sending websocket msg: {}", e);
        }
    }));

    let new_client = WSClient {
        client_id: Uuid::new_v4().as_simple().to_string(),
        sender: Some(client_sender),
    };


    let splashMsg = Ok(warp::ws::Message::text("Welcome"));
    let _ = new_client.sender.as_ref().unwrap().send(splashMsg);

    app.lock().unwrap().ws_clients.insert(new_client.client_id.clone(), new_client);
}
