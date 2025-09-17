# HttpReceiver

Run a HTTP receiver listening on `127.0.0.1:8080` that handles requests on the root path.
``` rust
HttpReceiver::new("127.0.0.1:8080")
.route("/", async move |_uuid, _request| {
    HttpResponse::ok()
})
.receive()
.await;
```

Run a HTTP receiver with TLS listening on `127.0.0.1:8080` that handles requests on the root path.
``` rust
HttpReceiver::new("127.0.0.1:8080")
.tls("/home/user/cert.pem", "/home/user/key.pem")
.route("/", async move |_uuid, _request| {
    HttpResponse::ok()
})
.receive()
.await;
```

Run a HTTP receiver listening on `127.0.0.1:8080` that handles requests on root `/`.
``` rust
HttpReceiver::new("127.0.0.1:8080")
.route("/", async move |_uuid, _request| {
    HttpResponse::ok()
})
.receive()
.await;
```

Run a HTTP receiver listening on `127.0.0.1:8080` that handles requests with a dynamic route `/user/{name}` where `{name}` is a path parameter.
``` rust
HttpReceiver::new("127.0.0.1:8080")
.route("/user/{name}", async move |_uuid, _request| {
    let name = request.params.get("name").unwrap();
    let text = format!("Hello {}", name);
    HttpResponse::ok().body(text.as_bytes())
})
.receive()
.await;
```

# HttpSender
HttpSender will automatically use a secure TLS connection if the scheme is `https` and ALPN is used to determine whether to use HTTP/2 or HTTP/1.1 for the request.

Send a GET request to `http://127.0.0.1:8080`.
``` rust
let response = HttpSender::new("http://127.0.0.1:8080")
.send(HttpRequest::get())
.await
.unwrap();
```

Send a GET request using TLS to `https://127.0.0.1:8080`.
``` rust
let response = HttpSender::new("https://127.0.0.1:8080")
.send(HttpRequest::get())
.await
.unwrap();
```

Send a GET request using TLS and add a custom Root CA to `https://127.0.0.1:8080`.
``` rust
let root_ca_path = home_dir().unwrap().join(".local/share/mkcert/rootCA.pem");
let response = HttpSender::new("https://127.0.0.1:8080")
.add_root_ca(root_ca_path)
.send(HttpRequest::get())
.await
.unwrap();
```