use common::Client;
use common::ClientConnectionData;
use futures::{FutureExt, StreamExt};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::ws::WebSocket;

use crate::AppState;
use crate::Arc;
use crate::Mutex;

pub async fn client_connection(ws: WebSocket, app: Arc<Mutex<AppState>>) {
    println!("establishing client connection... {:?}", ws);

    let (client_ws_sender, mut client_ws_rcv) = ws.split();
    let (client_sender, client_rcv) = mpsc::unbounded_channel();
    let client_rcv = UnboundedReceiverStream::new(client_rcv);

    // Keep stream open until disconnected
    tokio::task::spawn(client_rcv.forward(client_ws_sender).map(|result| {
        if let Err(ref e) = result {
            eprintln!("error sending websocket msg: {}", e);
        };
        result
    }));

    while let Some(result) = client_ws_rcv.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(_) => return,
        };
        let msg_json = match msg.to_str() {
            Ok(msg_json) => msg_json,
            Err(_) => return,
        };

        let new_client_data: ClientConnectionData =
            serde_json::from_str(msg_json).expect("Error parsing client connection message");

        let new_client = Client::new(client_sender);

        app.lock()
            .unwrap()
            .clients
            .insert(new_client_data.user_uuid, new_client);

        return; //FIXME: o co tu chodzi? Przecież przez to nie ma pętli
    }
}
