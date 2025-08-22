# Rust Integration Services

A modern, fast, and lightweight integration library written in Rust, designed for memory safety and stability. It simplifies the development of scalable integrations for receiving and sending data, with built-in support for common protocols.

[![Crates.io](https://img.shields.io/crates/v/rust-integration-services.svg)](https://crates.io/crates/rust-integration-services)
![Rust Version](https://img.shields.io/badge/rustc-1.70+-blue.svg)
[![Docs.rs](https://docs.rs/rust-integration-services/badge.svg)](https://docs.rs/rust-integration-services)  
[![License](https://img.shields.io/crates/l/rust-integration-services.svg)](https://github.com/AndreasLill/rust-integration-services#license)

## Installation

Add rust-integration-services to your project Cargo.toml with all or select features.

**All features**
``` toml
[dependencies]
tokio = { version = "1.47.1", features = ["full"] }
rust-integration-services = "0.2.4"
```

**With select features**
``` toml
[dependencies]
tokio = { version = "1.47.1", features = ["full"] }
rust-integration-services = { version = "0.2.4", default-features = false, features = ["file", "schedule", "sftp", "http"] }
```

## Features
### File
#### FileReceiver

Poll the directory `./io/in/` and receive a callback with the path of a matching file using regular expression.

``` rust
let result = FileReceiver::new("./io/in/")
.filter(".*", async move |_uuid, path| {
    println!("Callback: {:?}", path);
})
.receive()
.await;
```

#### FileSender

Move a file from one directory to another.
``` rust
let result = FileSender::new("./io/out/file.txt")
.send_move("./io/in/file.txt")
.await;
```

Copy the contents from a file to another.
``` rust
let result = FileSender::new("./io/out/file.txt")
.send_copy("./io/in/file.txt")
.await;
```

Write a string to a file, appending the text.
``` rust
let result = FileSender::new("./io/out/file.txt")
.send_string("text")
.await;
```
---
### Schedule
#### ScheduleReceiver

Run a task once every hour and receive an event when it triggers.
``` rust
let result = ScheduleReceiver::new()
.interval(ScheduleInterval::Hours(1))
.on_event(async move |event| {
    match event {
        ScheduleReceiverEventSignal::OnTrigger(uuid) => println!("{}", uuid),
    }
})
.receive()
.await;
```

Run a task once every day at 03:00 UTC and receive an event when it triggers.
``` rust
let result = ScheduleReceiver::new()
.start_time(03, 00, 00)
.interval(ScheduleInterval::Days(1))
.on_event(async move |event| {
    match event {
        ScheduleReceiverEventSignal::OnTrigger(uuid) => println!("{}", uuid),
    }
})
.receive()
.await;
```
---
### HTTP
The http module is built on top of the fast and reliable [`hyper`](https://crates.io/crates/hyper) HTTP library, and routing is handled using [`matchit`](https://crates.io/crates/matchit) url router library.

It supports both **HTTP/1.1** and **HTTP/2** protocols, enabling modern, high-performance HTTP communication with automatic protocol negotiation via ALPN (Application-Layer Protocol Negotiation) with dynamic routing for REST.

#### HttpReceiver

Run a HTTP receiver listening on `127.0.0.1:8080` that handles requests on the root path.
``` rust
HttpReceiver::new("127.0.0.1:8080")
.route("/", async move |_uuid, _request| {
    HttpResponse::ok()
})
.receive()
.await;
```

Run a HTTP receiver with TLS listening on `127.0.0.1:8080` that handles requests on the root path and log events.
``` rust
HttpReceiver::new("127.0.0.1:8080")
.tls("/home/user/cert.pem", "/home/user/key.pem")
.route("/", async move |_uuid, _request| {
    HttpResponse::ok()
})
.on_event(async move |event| {
    match event {
        HttpReceiverEventSignal::OnConnectionOpened(uuid, ip) => println!("Connection[{}] opened: {}", uuid, ip),
        HttpReceiverEventSignal::OnRequest(uuid, request) => println!("Request[{}]: {:?}", uuid, request),
        HttpReceiverEventSignal::OnResponse(uuid, response) => println!("Response[{}]: {:?}", uuid, response),
        HttpReceiverEventSignal::OnConnectionFailed(uuid, err) => println!("Failed[{}]: {}", uuid, err),
    }
})
.receive()
.await;
```

Run a HTTP receiver listening on `127.0.0.1:8080` that handles requests with a dynamic route `/user/{id}` where `{id}` is a path parameter.
``` rust
HttpReceiver::new("127.0.0.1:8080")
.route("/user/{id}", async move |_uuid, _request| {
    HttpResponse::ok()
})
.receive()
.await;
```

#### HttpSender
HttpSender will automatically use a secure TLS connection if the scheme is `https` and ALPN is used to determine whether to use HTTP/2 or HTTP/1.1 for the request.

Send a GET request to `http://127.0.0.1:8080`.
``` rust
let response = HttpSender::new()
.send("http://127.0.0.1:8080", HttpRequest::get())
.await
.unwrap();
```

Send a GET request using TLS to `https://127.0.0.1:8080`.
``` rust
let response = HttpSender::new()
.send("https://127.0.0.1:8080", HttpRequest::get())
.await
.unwrap();
```

Send a GET request using TLS and add a custom Root CA to `https://127.0.0.1:8080`.
``` rust
let root_ca_path = home_dir().unwrap().join(".local/share/mkcert/rootCA.pem");
let response = HttpSender::new()
.add_root_ca(root_ca_path)
.send("https://127.0.0.1:8080", HttpRequest::get())
.await
.unwrap();
```
---

### SFTP

Using SFTP requires `openssl` to be installed on the system.  
Make sure you also have the development packages of openssl installed.
For example, `libssl-dev` on Ubuntu or `openssl-devel` on Fedora.

#### SftpReceiver

Download all files from `127.0.0.1:22` user remote directory `upload` and delete files after successful download.  
Using `on_event` callback to print on download start and success.  
That way files can be processed asyncronously without blocking other downloads.

``` rust
let result = SftpReceiver::new("127.0.0.1:22", "user")
.auth_password("pass")
.remote_dir("upload")
.delete_after(true)
.on_event(async move |event| {
    match event {
        SftpReceiverEventSignal::OnDownloadStart(_uuid, path) => println!("Download started: {:?}", path),
        SftpReceiverEventSignal::OnDownloadSuccess(_uuid, path) => println!("Download complete: {:?}", path),
        SftpReceiverEventSignal::OnDownloadFailed(_uuid, path) => println!("Download failed: {:?}", path),
    }
})
.receive_files_to_path("/home/user/files")
.await;
```

By setting a custom regex to the receiver, the sftp receiver will only download files matching this regex.  
For examplem this will only download files starting with "ABC_".

``` rust
.regex("^ABC_.+\.[^./\\]+$")
```


#### SftpSender

Send a file to `127.0.0.1:22` user remote path `upload` keeping the same file name.  
A private key with a passphrase is used as authentication in this example.

``` rust
let result = SftpSender::new("127.0.0.1:22", "user")
.private_key("/home/user/.ssh/id_rsa", Some("secret"))
.remote_path("upload")
.send_file("/home/user/files/data.txt")
.await;
```

Send a string as a file to `127.0.0.1:22` user remote path `upload` with a new file name.  
A basic password is used as authentication in this example.

``` rust
let result = SftpSender::new("127.0.0.1:22", "user")
.password("secret")
.remote_path("upload")
.file_name("data.txt")
.send_string("a very beautiful important string")
.await;
```