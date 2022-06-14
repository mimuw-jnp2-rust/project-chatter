use crate::AppState;
use crate::Arc;
use crate::Mutex;
use crate::WSClient;
use futures::{FutureExt, StreamExt};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
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
        return result
    }));

    while let Some(result) = client_ws_rcv.next().await {

        let msg = match result {
            Ok(msg) => msg,
            Err(_) => return
         };

	    let json_str = match msg.to_str() {
		    Ok(v) => v,
            Err(_) => return
	    };

	    let new_client_data : ClientConnectionData = serde_json::from_str(json_str).expect("");

        let new_client = WSClient {
	        is_alive: true,
            sender: client_sender,
        };

        app.lock()
            .unwrap()
            .ws_clients
            .insert(new_client_data.connecting_user_name, new_client);

        return
    }
	


}
