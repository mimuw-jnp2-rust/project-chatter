use hyper::{
    body::to_bytes,
    service::{make_service_fn, service_fn},
    Body, Request, Server,
};
use route_recognizer::Params;
use router::Router;
use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use warp::{Filter, Rejection};
use std::{thread, time};
mod handler;
mod router;
mod ws;

type Response = hyper::Response<hyper::Body>;
type Error = Box<dyn std::error::Error + Send + Sync + 'static>;
type ResultWS<T> = std::result::Result<T, Rejection>;

#[derive(Debug, Clone)]
pub struct WSClient {
    pub isAlive: bool,
    pub sender: mpsc::UnboundedSender<std::result::Result<warp::ws::Message, warp::Error>>,
}

type WSClientsMap = HashMap<String, WSClient>;

pub struct AppState {
    pub name: String,
    pub ws_clients: WSClientsMap,
    pub routing_map: Arc<Router>,
}

impl AppState {
    fn new() -> Arc<Mutex<AppState>> {
        Arc::new(Mutex::new(AppState {
            name: "Pre-websocket server".to_string(),
            ws_clients: WSClientsMap::new(),
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

async fn run_heartbeat_service(app: Arc<Mutex<AppState>>)
{
    loop{
	thread::sleep(time::Duration::from_millis(5000));
	
	
	println!("All Clients are alive.");
    }
}

async fn run_ws(app: Arc<Mutex<AppState>>) {
    println!("Preparing WS...");

    let ws_route = warp::path("ws")
        .and(warp::ws())
        .and(with_clients(app.clone()))
        .and_then(handler::ws_handler);

    let routes = ws_route.with(warp::cors().allow_any_origin());
    println!("WS open on {} ...", "127.0.0.1:8000/ws");
    warp::serve(routes).run(([127, 0, 0, 1], 8000)).await;
}

fn with_clients(
    clients: Arc<Mutex<AppState>>,
) -> impl Filter<Extract = (Arc<Mutex<AppState>>,), Error = Infallible> + Clone {
    warp::any().map(move || clients.clone())
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
