use bytes::Bytes;
use hyper::{
    body::to_bytes,
    service::{make_service_fn, service_fn},
    Body, Request, Server,
};
use route_recognizer::Params;
use router::Router;
use std::sync::{Arc,Mutex};
use std::{thread, time};

mod handler;
mod router;
mod common;

type Response = hyper::Response<hyper::Body>;
type Error = Box<dyn std::error::Error + Send + Sync + 'static>;


// TODO Dodac utilsy do kolejki
#[derive(Clone, Debug)]
pub struct AppState {
    pub name: String,
    pub counter: u64,
    // O tutaj mapa userow, Appstate wchodzi zawsze do endpointa z ktorego mozna bedzie striggerowac wysłanie
}

#[tokio::main]
async fn main() {

	let mut app = Arc::new(Mutex::new(AppState {
	    name: "Pre websocket server".to_string(),
	    counter: 0,
	}));

	let http = tokio::spawn(make_http(app.clone()));
	let ws   = tokio::spawn(make_ws(app.clone()));
	
	let _ = ws.await;
	let _ = http.await;
}

async fn make_ws(app: Arc<Mutex<AppState>>){
	println!("WS...");
	let sleep_time = time::Duration::from_millis(1000);
	loop{
		app.lock().unwrap().counter += 1;
		thread::sleep(sleep_time);
	}
}

async fn make_http(app: Arc<Mutex<AppState>>){

    let shared_router = make_routing_map();


    let new_service = make_service_fn(move |_| {


	let app_capture = app.clone();
        let router_capture = shared_router.clone();

        async {
            Ok::<_, Error>(service_fn(move |req| {
                route(router_capture.clone(), req, app_capture.clone())
            }))
        }
    });

    let addr = "0.0.0.0:8080".parse().expect("address creation works");
    let server = Server::bind(&addr).serve(new_service);
    println!("HTTP open on http://{}", addr);
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
    body_bytes: Option<Bytes>,
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
