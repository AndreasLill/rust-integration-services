# Rust Integration Services

A modern, fast, and lightweight integration library written in Rust, designed for memory safety and stability. It simplifies the development of scalable integrations for receiving and sending data, with built-in support for common protocols.

## Installation

``` toml
[dependencies]
rust-integration-services = { version = "0", features = ["http", "file", "schedule"] }
```

## Features
### File
#### FileReceiver

>Example: Poll the directory ./io/in/ every 500ms, and receive a callback with the path of a matching file using regular expression.

``` rust
FileReceiver::new("./io/in/")
.filter(".*", async move |_uuid, path| {
    println!("Callback: {}", path.to_string_lossy());
})
.run_polling(500)
.await;
```
---
#### FileSender

>Example: Move a file from one directory to another.
``` rust
FileSender::new("./io/out/file.txt")
.move_from("./io/in/file.txt")
.await
.unwrap();
```
---
>Example: Copy the contents from a file to another.
``` rust
FileSender::new("./io/out/file.txt")
.copy_from("./io/in/file.txt")
.await
.unwrap();
```
---
>Example: Write a string to a file.
``` rust
FileSender::new("./io/out/file.txt")
.write_string("text")
.await
.unwrap();
```
---
### Schedule
#### ScheduleReceiver

>Example: Run a task once every hour.
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

>Example: Run a HTTP receiver listening on 127.0.0.1:8080 that handles GET and POST requests on the root path.
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
---
#### HttpSender

>Example: Send a HTTP GET request to 127.0.0.1:8080.
``` rust
HttpSender::new("127.0.0.1:8080")
.send(HttpRequest::get())
.await
.unwrap();
```
---
### SFTP (Coming soon)
### SMTP (Coming soon)
### SSH (Coming soon)