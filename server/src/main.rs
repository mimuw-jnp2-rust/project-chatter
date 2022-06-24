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

use crate::utils::{log_msg, setup_app_dir};

mod handler;
mod router;
mod ws;
mod utils;

const WS_ADDR: &str = "127.0.0.1:8000/ws";
const SERVER_SIGNATURE: &str = "SERVER"; //TODO: zarezerwowanie tego nicka

type Response = hyper::Response<hyper::Body>;
type Error = Box<dyn std::error::Error + Send + Sync + 'static>;
type ResultWS<T> = std::result::Result<T, Rejection>;

type ClientMap = HashMap<Uuid, Client>;
type RoomMap = HashMap<Uuid, Room>;

pub struct AppState {
    pub name: String,
    pub routing_map: Arc<Router>,
    pub clients: ClientMap, //TODO: ujednolicić nazewnictwo user/client
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
                // Register endpoints under the router
                //TODO: uporządkować to, w commonie też, w handler.rs też
                router.get("/test", Box::new(handler::test_handler));
                router.post("/send_msg", Box::new(handler::send_msg_handler));
                router.post("/leave_room", Box::new(handler::leave_room_handler));
                router.post("/exit_app", Box::new(handler::exit_app_handler));
                router.post("/login", Box::new(handler::login_handler));
                router.post("/get_room", Box::new(handler::get_room_handler));
                router.post("/create_room", Box::new(handler::create_room_handler));
                router.post("/join_room", Box::new(handler::join_room_handler));
                router.post("/heartbeat", Box::new(handler::heartbeat_handler));
                Arc::new(router)
            },
        }))
    }

    fn from_persistent_storage() -> Arc<Mutex<Self>> {
        todo!()
    }

    fn send_to_room(&self, msg: &ChatMessage, room_uuid: Uuid) {
        let msg_json = serde_json::to_string(&msg).unwrap();
        let room = self.rooms.get(&room_uuid).unwrap();

        for user_uuid in &room.members {
            let msg_for_user = Ok(warp::ws::Message::text(msg_json.clone()));

            if let Some(user_conn) = self.clients.get(user_uuid){
                user_conn
                    .sender
                    .send(msg_for_user)
                    .expect("Sending message failed!");
            }
        }
    }

    fn get_dead_users(&self) -> Vec<Uuid> {
        self.clients
            .iter()
            .filter(|(_, v)| !v.is_alive)
            .map(|(k, _)| *k)
            .collect::<Vec<_>>()
    }

    fn get_user_rooms(&self, user_uuid: Uuid) -> Vec<Uuid> {
        self.rooms
            .iter()
            .filter(|(_, v)| v.members.contains(&user_uuid))
            .map(|(k, _)| *k)
            .collect::<Vec<_>>()
    }

    fn remove_user(&mut self, uuid: Uuid) {
        self.clients.remove(&uuid);
    }

    fn disconnect_user_from_one(&mut self, user_uuid: Uuid, room_uuid: Uuid) {
        let goodbye_msg_content = format!(
            "{} has left the chat",
            &self.clients.get(&user_uuid).unwrap().username
        );
        let goodbye_msg = common::ChatMessage::new(SERVER_SIGNATURE, &*goodbye_msg_content);
        println!("{}", goodbye_msg);
        self.send_to_room(&goodbye_msg, room_uuid);
        self.rooms.get_mut(&room_uuid).unwrap().remove_user(user_uuid);
        log_msg(&goodbye_msg, room_uuid).expect(&*format!("Error logging message for room {}", room_uuid));
    }

    fn disconnect_user_from_all(&mut self, user_uuid: Uuid) {
        let goodbye_msg_content = format!(
            "{} has left the chat",
            &self.clients.get(&user_uuid).unwrap().username
        );
        let goodbye_msg = common::ChatMessage::new(SERVER_SIGNATURE, &*goodbye_msg_content);
        println!("{}", goodbye_msg);

        let user_rooms = self.get_user_rooms(user_uuid);
        for room in user_rooms {
            self.send_to_room(&goodbye_msg, room);
            self.rooms.get_mut(&room).unwrap().remove_user(user_uuid);
            log_msg(&goodbye_msg, room).expect(&*format!("Error logging message for room {}", room));
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
                self.body_bytes.as_ref().expect("body_bytes was set above")
            }
        };
        Ok(serde_json::from_slice(body_bytes)?)
    }
}

#[tokio::main]
async fn main() {
    let app = AppState::new();

    setup_app_dir().expect("App's directory setup failed");
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
        let dead_users = app.lock().unwrap().get_dead_users();

        app.lock().unwrap().clients.values_mut()
            .for_each(|user| user.is_alive = false); // flip users' status to dead

        if dead_users.is_empty() {
            println!("Everybody is alive");
        }
        else {
            println!("Some clients died");
            for dead_user_id in dead_users {
                app.lock().unwrap().disconnect_user_from_all(dead_user_id);
                // TODO: jak się to usunie, a user potem znowu włączy apkę, to skutkuje to
                // wysłaniem przez serwer goodbye-message'a ponownie. Ja bym np stworzył jakiegoś
                // enuma np UserState i tam trzymał 3 stany: Aktywny, Niekatywny i Martwy. No bo
                // ustawienie z powrotem `is_alive = true` tutaj byłoby jakieś mega głupie
                app.lock().unwrap().remove_user(dead_user_id);
            }
        }
    }
}

async fn run_ws(app: Arc<Mutex<AppState>>) {
    println!("Preparing WS...");

    let ws_route = warp::path("ws")
        .and(warp::ws())
        .and(warp::any().map(move || app.clone()))
        .and_then(handler::registration_handler);

    let routes = ws_route.with(warp::cors().allow_any_origin());
    println!("WS open on {}", WS_ADDR);
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
