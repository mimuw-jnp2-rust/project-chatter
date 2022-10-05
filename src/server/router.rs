use std::collections::HashMap;

use async_trait::async_trait;
use futures::future::Future;
use hyper::Method;
use route_recognizer::{Match, Params, Router as InternalRouter};

use crate::{Context, Response};
use crate::handler::not_found_resp;

#[async_trait]
pub trait Handler: Send + Sync + 'static {
    async fn invoke(&self, context: Context) -> Response;
}

#[async_trait]
impl<F, Fut> Handler for F
where
    F: Send + Sync + 'static + Fn(Context) -> Fut,
    Fut: Future + Send + 'static,
    Fut::Output: IntoResponse,
{
    async fn invoke(&self, context: Context) -> Response {
        (self)(context).await.into_response()
    }
}

pub struct RouterMatch<'a> {
    pub handler: &'a dyn Handler,
    pub params: Params,
}

pub struct Router {
    method_map: HashMap<Method, InternalRouter<Box<dyn Handler>>>,
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

impl Router {
    pub fn new() -> Router {
        Router {
            method_map: HashMap::new(),
        }
    }

    pub fn get(&mut self, path: &str, handler: Box<dyn Handler>) {
        self.method_map
            .entry(Method::GET)
            .or_insert_with(InternalRouter::new)
            .add(path, handler)
    }

    pub fn post(&mut self, path: &str, handler: Box<dyn Handler>) {
        self.method_map
            .entry(Method::POST)
            .or_insert_with(InternalRouter::new)
            .add(path, handler)
    }

    pub fn route(&self, path: &str, method: &Method) -> RouterMatch<'_> {
        if let Some(Match { handler, params }) = self
            .method_map
            .get(method)
            .and_then(|r| r.recognize(path).ok())
        {
            RouterMatch {
                handler: &**handler,
                params,
            }
        } else {
            RouterMatch {
                handler: &|_| async move { not_found_resp() },
                params: Params::new(),
            }
        }
    }
}

pub trait IntoResponse: Send + Sized {
    fn into_response(self) -> Response;
}

impl IntoResponse for Response {
    fn into_response(self) -> Response {
        self
    }
}
