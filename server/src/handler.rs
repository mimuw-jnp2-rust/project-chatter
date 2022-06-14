use crate::AppState;
use crate::Arc;
use crate::Mutex;
use crate::{ws, Context, Response, ResultWS};
use hyper::StatusCode;
use warp::Reply;

pub async fn not_found_handler(_ctx: Context) -> Response {
    hyper::Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body("404: NOT FOUND".into())
        .unwrap()
}

pub async fn test_handler(ctx: Context) -> Response {
    let app = ctx.state.lock().unwrap();

    hyper::Response::builder()
        .status(StatusCode::OK)
        .body(format!("I am {} and I am alive.", app.name).into())
        .unwrap()
}

pub async fn send_handler(mut ctx: Context) -> Response {
    let received_msg: common::ChatMessage = match ctx.body_json().await {
        Ok(v) => v,
        Err(e) => {
            return hyper::Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(format!("could not parse JSON: {}", e).into())
                .unwrap();
        }
    };

    println!("{}", received_msg);

    let sent_str = serde_json::to_string(&received_msg).unwrap();

    ctx.state.clone().lock().unwrap().send_to_all(&sent_str);

    hyper::Response::builder()
        .status(StatusCode::OK)
        .body("OK".into())
        .unwrap()
}

pub async fn heartbeat_handler(mut ctx: Context) -> Response {
    let received_heartbeat: common::HeartbeatData = match ctx.body_json().await {
        Ok(v) => v,
        Err(e) => {
            return hyper::Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(format!("could not parse JSON: {}", e).into())
                .unwrap();
        }
    };
    println!("BEAT from {}", received_heartbeat.aliveUserName);
 
    if !ctx.state.clone().lock().unwrap().ws_clients.contains_key(&received_heartbeat.aliveUserName){

	    println!("User {} not found",received_heartbeat.aliveUserName);
    }
    else{

	    ctx.state.clone().lock().unwrap().ws_clients.get_mut(&received_heartbeat.aliveUserName).unwrap().isAlive = true;
    }

	

    hyper::Response::builder()
        .status(StatusCode::OK)
        .body("OK".into())
        .unwrap()
}


pub async fn ws_handler(ws: warp::ws::Ws, app: Arc<Mutex<AppState>>) -> ResultWS<impl Reply> {
    Ok(ws.on_upgrade(move |socket| ws::client_connection(socket, app)))
}
