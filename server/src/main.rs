use bytes::Bytes;
use hyper::{
    body::to_bytes,
    service::{make_service_fn, service_fn},
    Body, Request, Server,
};
use route_recognizer::Params;
use router::Router;
use std::sync::{Arc,Mutex};
use tokio::sync::mpsc;
use std::{thread, time,convert::Infallible};
use warp::{Filter, Rejection};
use std::collections::HashMap;


mod handler;
mod router;
mod common;
mod ws;

type Response = hyper::Response<hyper::Body>;
type Error = Box<dyn std::error::Error + Send + Sync + 'static>;
type ResultWS<T> = std::result::Result<T, Rejection>;


#[derive(Debug, Clone)]
pub struct WSClient {
    pub client_id: String,
    pub sender: Option<mpsc::UnboundedSender<std::result::Result<warp::ws::Message, warp::Error>>>,
}


type WSClientsMap = HashMap<String, WSClient>;

#[derive(Clone, Debug)]
pub struct AppState {
    pub name: String,
    pub counter: u64,
    pub ws_clients: WSClientsMap,
    // O tutaj mapa userow, Appstate wchodzi zawsze do endpointa z ktorego mozna bedzie striggerowac wysłanie
}

#[tokio::main]
async fn main() {

	let app = Arc::new(Mutex::new(AppState {
	    name: "Pre websocket server".to_string(),
	    counter: 0,
	    ws_clients: HashMap::new(),
	}));

	let http = tokio::spawn(run_http(app.clone()));
	let ws   = tokio::spawn(run_ws(app.clone()));

	let _ = ws.await;
	let _ = http.await;
}

async fn run_ws(app: Arc<Mutex<AppState>>){
	println!("Preparing WS...");

        let ws_route = warp::path("ws")
            .and(warp::ws())
            .and_then(handler::ws_handler);

        let routes = ws_route.with(warp::cors().allow_any_origin());
        println!("WS open on 127.0.0.1:8000/ws ...");
        warp::serve(routes).run(([127, 0, 0, 1], 8000)).await;
}

fn with_clients(clients: AppState) -> impl Filter<Extract = (AppState,), Error = Infallible> + Clone {
    warp::any().map(move || clients.clone())
}


async fn run_http(app: Arc<Mutex<AppState>>){
    
    println!("Preparing HTTP...");
    let new_service = make_service_fn(move |_| {

	let app_capture = app.clone();

        async {
            Ok::<_, Error>(service_fn(move |req| {
                route(make_routing_map(), req, app_capture.clone())
            }))
        }
    });

    let addr = "127.0.0.1:8080".parse().expect("address creation works");
    let server = Server::bind(&addr).serve(new_service);
    println!("HTTP open on {}", addr);
    let _ = server.await;
}

fn make_routing_map() -> Arc<Router>
{
    // Register endpoints
    let mut router: Router = Router::new();
    router.get("/test",  Box::new(handler::test_handler));
    router.post("/post", Box::new(handler::send_handler));


    return Arc::new(router);
}



async fn route(router: Arc<Router>, req: Request<hyper::Body>,app_state: Arc<Mutex<AppState>>) -> Result<Response, Error> {

    let found_handler = router.route(req.uri().path(), req.method());
    let resp = found_handler
        .handler
        .invoke(Context::new(app_state, req, found_handler.params))
        .await;
    Ok(resp)
}

#[derive(Debug)]
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
        Ok(serde_json::from_slice(&body_bytes)?)
    }
}
