use std::{marker::PhantomData, sync::Arc};

use matchit::Router;

use crate::http::{http_request::HttpRequest, http_response::HttpResponse, server::{http_server::{HttpServer, HttpServerError, RequestMiddlewareCallback, ResponseMiddlewareCallback, RouteCallback}, http_server_config::HttpServerConfig}};

pub struct Before;
pub struct Route;
pub struct After;

pub struct HttpServerBuilder<State> {
    config: HttpServerConfig,
    router: Router<RouteCallback>,
    request_middleware: Vec<RequestMiddlewareCallback>,
    response_middleware: Vec<ResponseMiddlewareCallback>,
    _state: PhantomData<State>
}

impl HttpServerBuilder<Before> {
    pub fn new(config: HttpServerConfig) -> Self {
        Self {
            config: config,
            router: Router::new(),
            request_middleware: Vec::new(),
            response_middleware: Vec::new(),
            _state: PhantomData
        }
    }

    /// Add a middleware to the request pipeline.
    ///
    /// - Return `Ok(HttpRequest)` to continue to the next middleware or route handler
    /// - Return `Err(HttpResponse)` to short-circuit the pipeline and respond immediately
    ///
    /// Response will still be processed by the response `after` pipeline, if any.
    pub fn request_middleware<T, Fut>(mut self, callback: T) -> Self
    where 
        T: Fn(HttpRequest) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<HttpRequest, HttpResponse>> + Send + 'static
    {
        let callback = Arc::new(callback);
        self.request_middleware.push(Arc::new(move |request| {
            let callback = Arc::clone(&callback);
            Box::pin(async move {
                callback(request).await
            })
        }));
        self
    }

    /// Registers a route with a path, associating it with a handler callback.
    pub fn route<T, Fut>(mut self, path: impl Into<String>, callback: T) -> HttpServerBuilder<Route>
    where
        T: Fn(HttpRequest) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<HttpResponse, HttpServerError>> + Send + 'static,
    {
        self.router.insert(path.into(), Arc::new(move |request| Box::pin(callback(request)))).unwrap();
        HttpServerBuilder {
            config: self.config,
            router: self.router,
            request_middleware: self.request_middleware,
            response_middleware: self.response_middleware,
            _state: PhantomData
        }
    }
}

impl HttpServerBuilder<Route> {
    /// Registers a route with a path, associating it with a handler callback.
    pub fn route<T, Fut>(mut self, path: impl Into<String>, callback: T) -> Self
    where
        T: Fn(HttpRequest) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<HttpResponse, HttpServerError>> + Send + 'static,
    {
        self.router.insert(path.into(), Arc::new(move |request| Box::pin(callback(request)))).unwrap();
        self
    }

    /// Add a middleware to the response pipeline.
    ///
    /// This middleware runs after a response is produced and can modify the response before it is sent to the client.
    /// 
    /// It does not affect request execution flow.
    pub fn response_middleware<T, Fut>(mut self, callback: T) -> HttpServerBuilder<After>
    where 
        T: Fn(HttpResponse) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<HttpResponse, HttpServerError>> + Send + 'static,
    {
        self.response_middleware.push(Arc::new(move |response| Box::pin(callback(response))));
        HttpServerBuilder {
            config: self.config,
            router: self.router,
            request_middleware: self.request_middleware,
            response_middleware: self.response_middleware,
            _state: PhantomData
        }
    }

    pub fn build(self) -> HttpServer {
        HttpServer::new(self.config, self.router, self.request_middleware, self.response_middleware)
    }
}

impl HttpServerBuilder<After> {
    pub fn build(self) -> HttpServer {
        HttpServer::new(self.config, self.router, self.request_middleware, self.response_middleware)
    }
}