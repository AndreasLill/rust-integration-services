# Rust Integration Services

A modern, fast, and lightweight integration library written in Rust, designed for memory safety and stability. It simplifies the development of scalable integrations for receiving and sending data, with built-in support for common protocols.

## Installation

``` toml
[dependencies]
rust-integration-services = { version = "0", features = ["http", "file", "schedule", "sftp"] }
```

## Features
### File
#### FileReceiver

Poll the directory `./io/in/` every 500 milliseconds, and receive a callback with the path of a matching file using regular expression.

``` rust
FileReceiver::new("./io/in/")
.filter(".*", async move |_uuid, path| {
    println!("Callback: {}", path.to_string_lossy());
})
.run_polling(500)
.await;
```

#### FileSender

Move a file from one directory to another.
``` rust
let result = FileSender::new("./io/out/file.txt")
.move_from("./io/in/file.txt")
.await;
```

Copy the contents from a file to another.
``` rust
let result = FileSender::new("./io/out/file.txt")
.copy_from("./io/in/file.txt")
.await;
```

Write a string to a file.
``` rust
let result = FileSender::new("./io/out/file.txt")
.write_string("text")
.await;
```
---
### Schedule
#### ScheduleReceiver

Run a task once every hour and receive an event when it triggers.
``` rust
ScheduleReceiver::new()
.interval(ScheduleInterval::Hour(1))
.on_event(async move |event| {
    match event {
        ScheduleReceiverEventSignal::OnTrigger(uuid) => println!("{}", uuid),
    }
})
.run()
.await;
```
---
### HTTP
#### HttpReceiver

Run a HTTP receiver listening on `127.0.0.1:8080` that handles `GET` and `POST` requests on the root path.
``` rust
HttpReceiver::new("127.0.0.1", 8080)
.route("GET", "/", async move |_uuid, _request| {
    HttpResponse::ok()
})
.route("POST", "/", async move |_uuid, _request| {
    HttpResponse::ok()
})
.run()
.await;
```

#### HttpSender

Send a HTTP GET request to `127.0.0.1:8080`.
``` rust
let response = HttpSender::new("127.0.0.1:8080")
.send(HttpRequest::get())
.await
.unwrap();
```
---

### SFTP

Using SFTP requires `openssl` to be installed on the system.  
Make sure you also have the development packages of openssl installed.
For example, `libssl-dev` on Ubuntu or `openssl-devel` on Fedora.

#### SftpSender

Send a file to `127.0.0.1:22` remote directory `upload` keeping the same file name.  
A private key with a passphrase is used as authentication in this example.

``` rust
let result = SftpSender::new("127.0.0.1:22", "user")
.auth_private_key("/home/user/.ssh/id_rsa", "secret")
.remote_dir("upload")
.send_file("/home/user/files/data.txt")
.await;
```

Send a string as a file to `127.0.0.1:22` remote directory `upload` with a new file name.  
A basic password is used as authentication in this example.

``` rust
let result = SftpSender::new("127.0.0.1:2222", "user")
.auth_password("secret")
.remote_dir("upload")
.file_name("data.txt")
.send_string("a very beautiful important string")
.await;
```