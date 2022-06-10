use crate::AppState;
use crate::Arc;
use crate::Mutex;
use crate::WSClient;
use futures::{FutureExt, StreamExt};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use uuid::Uuid;
use warp::ws::WebSocket;
use common::ClientConnectionData;

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
                Err(e) => {
		    return 
                }
         };

	    let jsonStr = match msg.to_str() {
		Ok(v) => v,
                Err(e) => {
		    return 
                }
	    };

	let newClientData : ClientConnectionData = serde_json::from_str(jsonStr).expect("");

            let new_client = WSClient {
	isAlive: true,
        sender: client_sender,

    };

    app.lock()
        .unwrap()
        .ws_clients
        .insert(newClientData.connectingUserName, new_client);
	return
    }
	


}
