use hyper::{
    body::to_bytes,
    service::{make_service_fn, service_fn},
    Body, Request, Server,
};

use common::Room;
use route_recognizer::Params;
use router::Router;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use std::{thread, time};
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;
use warp::{Filter, Rejection};
mod handler;
mod router;
mod ws;

type Response = hyper::Response<hyper::Body>;
type Error = Box<dyn std::error::Error + Send + Sync + 'static>;
type ResultWS<T> = std::result::Result<T, Rejection>;

#[derive(Debug, Clone)]
pub struct WSClient {
    pub is_alive: bool,
    pub sender: UnboundedSender<Result<warp::ws::Message, warp::Error>>,
}

type WSClientsMap = HashMap<String, WSClient>;
type RoomsMap = HashMap<String, Room>;
type UserNamesMap = HashMap<Uuid, String>; //TODO: use these Uuids whenever they can replace a String
type RoomNamesMap = HashMap<Uuid, String>; //TODO: as areqUsersMapbove

pub struct AppState {
    pub name: String,
    pub routing_map: Arc<Router>,
    pub clients_map: WSClientsMap,
    pub rooms_map: RoomsMap,
    pub usernames: UserNamesMap,
    pub room_names: RoomNamesMap,
}

impl AppState {
    fn new() -> Arc<Mutex<AppState>> {
        Arc::new(Mutex::new(AppState {
            name: "Pre-websocket server".to_string(),
            clients_map: WSClientsMap::new(),
            rooms_map: RoomsMap::new(),
            usernames: UserNamesMap::new(),
            room_names: RoomNamesMap::new(),
            routing_map: {
                let mut router: Router = Router::new();
                // Register endpoints under the router
                router.get("/test", Box::new(handler::test_handler));
                router.post("/post", Box::new(handler::send_handler));
                router.post("/heartbeat", Box::new(handler::heartbeat_handler));
                Arc::new(router)
            },
        }))
    }
    pub fn send_to_all(&mut self, msg: &String) {
        for connection in self.clients_map.values() {
            let splash_msg = Ok(warp::ws::Message::text(msg.clone()));
            connection
                .sender
                .send(splash_msg)
                .expect("Sending splash message failed!");
        }
    }
    pub fn send_to_room(&mut self, _msg: &String, _room: &String) {
        todo!("Ty juz wiesz");
    }
}


pub struct Context {
    pub state: Arc<Mutex<AppState>>,
    pub req: Request<Body>,
    pub params: Params,
    body_bytes: Option<hyper::body::Bytes>,
}

impl Context {
    pub fn new(state: Arc<Mutex<AppState>>, req: Request<Body>, params: Params) -> Context {
        Context {
            state,
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

    heartbeat.await.expect("Heartbeat service thread died!");
    ws.await.expect("WS server thread died!");
    http.await.expect("HTTP server thread died!");
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
    println!("Heartbeat service running");

    loop {
        thread::sleep(time::Duration::from_millis(5000));

        let mut app_ref = app.as_ref().lock().unwrap();

        //let mut usersToRemove = vec![];

        /*   let rooms = app_ref.rooms.values();

        for ((name, mut v), room) in app_ref.ws_clients.iter_mut().zip(rooms) {
            if v.is_alive {
                v.is_alive = false;
            } else {
                let _msg = common::ChatMessage::new("Server",name, room);

                println!("{} left the chat",name);
                // TODO: send_to_all("{name} left the chat");
                app_ref.ws_clients.remove(name);
            }
        }*/
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
