use std::{thread, time};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::ops::Deref;
use std::sync::{Arc, Mutex};

use common::{ChatMessage, Client, Room};
use hyper::{
    Body,
    body::to_bytes,
    Request, Server, service::{make_service_fn, service_fn},
};
use route_recognizer::Params;
use uuid::Uuid;
use warp::{Filter, Rejection};

use router::Router;

mod handler;
mod router;
mod ws;

type Response = hyper::Response<hyper::Body>;
type Error = Box<dyn std::error::Error + Send + Sync + 'static>;
type ResultWS<T> = std::result::Result<T, Rejection>;

type ClientMap = HashMap<Uuid, Client>;
type RoomMap = HashMap<Uuid, Room>;

pub struct AppState {
    pub name: String,
    pub routing_map: Arc<Router>,
    pub clients_map: ClientMap,
    pub rooms_map: RoomMap,
}

impl AppState {
    fn new() -> Arc<Mutex<AppState>> {
        Arc::new(Mutex::new(AppState {
            name: "Pre-websocket server".to_string(),
            clients_map: ClientMap::new(),
            rooms_map: RoomMap::new(),
            routing_map: {
                let mut router: Router = Router::new();
                // Register endpoints under the router
                router.get("/test", Box::new(handler::test_handler)); //TODO: remove
                router.post("/send_msg", Box::new(handler::send_msg_handler));
                router.post("/start", Box::new(handler::start_handler));
                router.post("/heartbeat", Box::new(handler::heartbeat_handler));
                Arc::new(router)
            },
        }))
    }

    fn send_to_room(&mut self, msg: &ChatMessage, room_uuid: Uuid) {
        let msg_json = serde_json::to_string(&msg).unwrap();
        let room = self.rooms_map.get(&room_uuid).unwrap();
        for user_uuid in &room.members {
            let msg_for_user = Ok(warp::ws::Message::text(msg_json.clone()));
            let user_conn = self.clients_map.get(&user_uuid).unwrap();
            user_conn
                .sender
                .send(msg_for_user)
                .expect("Sending message failed!");
        }
    }

    fn remove_user(&mut self, uuid: Uuid) {
        self.clients_map.remove(&uuid);
    }
}

pub struct Context {
    pub app_state: Arc<Mutex<AppState>>,
    pub req: Request<Body>,
    pub params: Params,
    body_bytes: Option<hyper::body::Bytes>,
}

impl Context {
    pub fn new(state: Arc<Mutex<AppState>>, req: Request<Body>, params: Params) -> Context {
        Context {
            app_state: state,
            req,
            params,
            body_bytes: None,
        }
    }

    pub async fn body_json<T: serde::de::DeserializeOwned>(&mut self) -> Result<T, Error> {
        let body_bytes = match self.body_bytes {
            Some(ref v) => v,
            _ => {
                let body = to_bytes(self.req.body_mut()).await?;
                self.body_bytes = Some(body);
                self.body_bytes.as_ref().expect("body_bytes was set above")
            }
        };
        Ok(serde_json::from_slice(body_bytes)?)
    }
}

#[tokio::main]
async fn main() {
    let app = AppState::new();

    let http = tokio::spawn(run_http(app.clone()));
    let ws = tokio::spawn(run_ws(app.clone()));
    let heartbeat = tokio::spawn(run_heartbeat_service(app.clone()));

    heartbeat.await.expect("Heartbeat service died!");
    ws.await.expect("WS server died!");
    http.await.expect("HTTP server died!");
}

async fn route_and_handle(
    router: Arc<Router>,
    req_body: Request<hyper::Body>,
    app_state: Arc<Mutex<AppState>>,
) -> Result<Response, Error> {
    let found_handler = router.route(req_body.uri().path(), req_body.method());
    let resp = found_handler
        .handler
        .invoke(Context::new(app_state, req_body, found_handler.params))
        .await;
    Ok(resp)
}

async fn run_heartbeat_service(app: Arc<Mutex<AppState>>) {
    const KILL_TIMEOUT: u64 = 5000;
    println!("Heartbeat service running");

    loop {
        thread::sleep(time::Duration::from_millis(KILL_TIMEOUT));
        let mut app = app.lock().unwrap();

        // flip users' status to dead
        let mut users_to_remove = Vec::with_capacity(app.clients_map.keys().len());
        for (user_uuid, user_conn) in &mut app.clients_map {
            if user_conn.is_alive {
                user_conn.is_alive = false;
            } else {
                users_to_remove.push(user_uuid.clone());
            }
        }

        // remove dead users
        for user_uuid in users_to_remove {
            let user = app.clients_map.get(&user_uuid).unwrap();
            if let Some(room_uuid) = user.room_uuid {
                let goodbye_msg = format!("{} has left the chat", user.username.as_ref().unwrap());
                let msg = common::ChatMessage::new("Server", &*goodbye_msg);
                app.send_to_room(&msg, room_uuid);
                app.remove_user(user_uuid);
            }
        }
    }
}

async fn run_ws(app: Arc<Mutex<AppState>>) {
    println!("Preparing WS...");

    let ws_route = warp::path("ws")
        .and(warp::ws())
        .and(warp::any().map(move || app.clone())) //TODO: co to robi?
        .and_then(handler::ws_handler);

    let routes = ws_route.with(warp::cors().allow_any_origin());
    println!("WS open on {}", "127.0.0.1:8000/ws");
    warp::serve(routes).run(([127, 0, 0, 1], 8000)).await;
}

async fn run_http(app: Arc<Mutex<AppState>>) {
    println!("Preparing HTTP...");

    let new_service = make_service_fn(move |_| {
        let app_capture = app.clone();
        async {
            Ok::<_, Error>(service_fn(move |req| {
                let router = app_capture.deref().lock().unwrap().routing_map.clone();
                route_and_handle(router, req, app_capture.clone())
            }))
        }
    });

    let addr = "127.0.0.1:8080"
        .parse::<SocketAddr>()
        .expect("http address creation failed");
    let server = Server::bind(&addr).serve(new_service);

    println!("HTTP open on {}", addr);
    let _ = server.await;
}
