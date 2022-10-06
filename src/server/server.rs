mod handler;
mod logging;
mod router;
mod ws;

use crate::logging::{log_msg, setup_app_dir};
use crate::router::Router;
use hyper::{
    body::to_bytes,
    service::{make_service_fn, service_fn},
    Body, Request, Server,
};
use route_recognizer::Params;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use std::{thread, time};
use warp::{Filter, Rejection};
use JNP2_Rust_Chatter::common::*;

type Response = hyper::Response<Body>;
type Error = Box<dyn std::error::Error + Send + Sync + 'static>;
type ResultWS<T> = Result<T, Rejection>;

type ClientMap = HashMap<ClientUuid, Client>;
type RoomMap = HashMap<RoomUuid, Room>;

pub struct AppState {
    pub name: String,
    pub routing_map: Arc<Router>,
    pub clients: ClientMap,
    pub rooms: RoomMap,
}

impl AppState {
    fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(AppState {
            name: "Pre-websocket server".to_string(),
            clients: ClientMap::new(),
            rooms: RoomMap::new(),
            routing_map: {
                let mut router: Router = Router::new();
                router.get(
                    HEALTH_CHECK_ENDPOINT,
                    Box::new(handler::handle_health_check),
                );
                router.post(SEND_MSG_ENDPOINT, Box::new(handler::handle_send_msg));
                router.post(LEAVE_ROOM_ENDPOINT, Box::new(handler::handle_leave_room));
                router.post(EXIT_APP_ENDPOINT, Box::new(handler::handle_exit_app));
                router.post(LOGIN_ENDPOINT, Box::new(handler::handle_login));
                router.post(GET_ROOM_ENDPOINT, Box::new(handler::handle_get_room));
                router.post(CREATE_ROOM_ENDPOINT, Box::new(handler::handle_create_room));
                router.post(JOIN_ROOM_ENDPOINT, Box::new(handler::handle_join_room));
                router.post(HEARTBEAT_ENDPOINT, Box::new(handler::handle_heartbeat));
                Arc::new(router)
            },
        }))
    }

    fn send_to_room(&self, msg: &ChatMessage, room_uuid: RoomUuid) {
        let msg_json = serde_json::to_string(&msg).unwrap();
        let room = self.rooms.get(&room_uuid).unwrap();

        for client_uuid in &room.members {
            let msg_for_client = Ok(warp::ws::Message::text(msg_json.clone()));

            if let Some(client_conn) = self.clients.get(client_uuid) {
                client_conn
                    .sender
                    .send(msg_for_client)
                    .expect("Sending message failed!");
            }
        }
    }

    fn get_dead_clients(&self) -> Vec<ClientUuid> {
        self.clients
            .iter()
            .filter(|(_, v)| !v.is_alive)
            .map(|(k, _)| *k)
            .collect::<Vec<_>>()
    }

    fn get_client_rooms(&self, client_uuid: ClientUuid) -> Vec<RoomUuid> {
        self.rooms
            .iter()
            .filter(|(_, v)| v.contains(&client_uuid))
            .map(|(k, _)| *k)
            .collect::<Vec<_>>()
    }

    fn remove(&mut self, client_uuid: ClientUuid) {
        self.clients.remove(&client_uuid);
    }

    fn disconnect_client_from_one(&mut self, client_uuid: ClientUuid, room_uuid: RoomUuid) {
        let goodbye_msg_content = format!(
            "{} has left the chat",
            &self.clients.get(&client_uuid).unwrap().name.0
        );
        let goodbye_msg = ChatMessage::new(SERVER_SIGNATURE, &*goodbye_msg_content);
        self.send_to_room(&goodbye_msg, room_uuid);
        self.rooms.get_mut(&room_uuid).unwrap().remove(client_uuid);
        if log_msg(&goodbye_msg, room_uuid).is_err() {
            eprintln!("Error logging message for room {}", room_uuid.0);
        }
    }

    fn disconnect_client_from_all(&mut self, client_uuid: ClientUuid) {
        let goodbye_msg_content = format!(
            "{} has left the chat",
            &self.clients.get(&client_uuid).unwrap().name.0
        );
        let goodbye_msg = ChatMessage::new(SERVER_SIGNATURE, &*goodbye_msg_content);

        let client_rooms = self.get_client_rooms(client_uuid);
        for room in client_rooms {
            self.send_to_room(&goodbye_msg, room);
            self.rooms.get_mut(&room).unwrap().remove(client_uuid);
            if log_msg(&goodbye_msg, room).is_err() {
                eprintln!("Error logging message for room {}", room.0);
            }
        }
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
                self.body_bytes.as_ref().unwrap()
            }
        };
        Ok(serde_json::from_slice(body_bytes)?)
    }
}

#[tokio::main]
async fn main() {
    let app = AppState::new();

    setup_app_dir().expect("App's directory setup failed!");
    let http = tokio::spawn(run_http(app.clone()));
    let ws = tokio::spawn(run_ws(app.clone()));
    let heartbeat = tokio::spawn(run_heartbeat_service(app.clone()));

    heartbeat.await.expect("Heartbeat service died!");
    ws.await.expect("WS server died!");
    http.await.expect("HTTP server died!");
}

async fn route_and_handle(
    router: Arc<Router>,
    req_body: Request<Body>,
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
    println!("Heartbeat service running!");

    loop {
        thread::sleep(time::Duration::from_millis(KILL_TIMEOUT));
        let dead_clients = app.lock().unwrap().get_dead_clients();

        app.lock()
            .unwrap()
            .clients
            .values_mut()
            .for_each(|client| client.is_alive = false); // flip clients' status to dead

        if !dead_clients.is_empty() {
            eprintln!("Some clients died!");
            for dead_client_id in dead_clients {
                app.lock()
                    .unwrap()
                    .disconnect_client_from_all(dead_client_id);
                app.lock().unwrap().remove(dead_client_id);
            }
        }
    }
}

fn build_addr(addr_str: String) -> SocketAddr {
    addr_str
        .as_str()
        .parse::<SocketAddr>()
        .expect("Address creation failed!")
}

async fn run_ws(app: Arc<Mutex<AppState>>) {
    let addr = build_addr(get_addr_str(Protocol::WS));

    let ws_route = warp::ws()
        .and(warp::any().map(move || app.clone()))
        .and_then(handler::handle_registration);

    let routes = ws_route.with(warp::cors().allow_any_origin());
    println!("WS open on {}", addr);
    warp::serve(routes).run(addr).await;
}

async fn run_http(app: Arc<Mutex<AppState>>) {
    let new_service = make_service_fn(move |_| {
        let app_capture = app.clone();
        async {
            Ok::<_, Error>(service_fn(move |req| {
                let router = app_capture.deref().lock().unwrap().routing_map.clone();
                route_and_handle(router, req, app_capture.clone())
            }))
        }
    });

    let addr = build_addr(get_addr_str(Protocol::HTTP));
    let server = Server::bind(&addr).serve(new_service);

    println!("HTTP open on {}", addr);
    let _ = server.await;
}
