use crate::AppState;
use crate::Arc;
use crate::Mutex;
use crate::WSClient;
use futures::{FutureExt, StreamExt};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use uuid::Uuid;
use warp::ws::WebSocket;

pub async fn client_connection(ws: WebSocket, app: Arc<Mutex<AppState>>) {
    println!("establishing client connection... {:?}", ws);

    let (client_ws_sender, _) = ws.split();
    let (client_sender, client_rcv) = mpsc::unbounded_channel();
    let client_rcv = UnboundedReceiverStream::new(client_rcv);

    // Keep stream open until disconnected
    tokio::task::spawn(client_rcv.forward(client_ws_sender).map(|result| {
        if let Err(ref e) = result {
            eprintln!("error sending websocket msg: {}", e);
        };
        result
    }));

    let mut new_client = WSClient {
	isAlive: true,
        sender: client_sender,
    };

    app.lock()
        .unwrap()
        .ws_clients
        .insert(Uuid::new_v4().as_simple().to_string(), new_client);
}
