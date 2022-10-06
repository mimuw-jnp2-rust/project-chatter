use futures::{FutureExt, StreamExt};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use uuid::Uuid;
use warp::ws::WebSocket;

use JNP2_Rust_Chatter::common::{Client, ClientUuid, ReqData};

use crate::AppState;
use crate::Arc;
use crate::Mutex;

pub async fn new_client_connection(ws: WebSocket, app: Arc<Mutex<AppState>>) {
    let (client_ws_sender, mut client_ws_rcv) = ws.split();
    let (client_sender, client_rcv) = mpsc::unbounded_channel();
    let client_rcv = UnboundedReceiverStream::new(client_rcv);

    // Keep stream open until disconnected
    tokio::task::spawn(client_rcv.forward(client_ws_sender).map(|result| {
        if let Err(e) = &result {
            eprintln!("Stream closed: {}", e);
        };
        result
    }));

    if let Some(result) = client_ws_rcv.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(_) => return,
        };
        let msg_json = match msg.to_str() {
            Ok(msg_json) => msg_json,
            Err(_) => return,
        };
        match serde_json::from_str(msg_json) {
            Err(e) => eprintln!("Invalid client registration request: {}", e),
            Ok(v) => match v {
                ReqData::RegistrationData(name) => {
                    let new_client = Client::new(client_sender, &*name.0);
                    app.lock()
                        .unwrap()
                        .clients
                        .insert(ClientUuid(Uuid::new_v4()), new_client);
                }
                _ => eprintln!("Invalid client registration request"),
            },
        }
    }
}
