# FileReceiver

Poll the directory `./io/in/` and receive a callback with the path of a matching file using regular expression.

``` rust
FileReceiver::new("./io/in/")
.route(".*", async move |_uuid, path| {
    println!("Callback: {:?}", path);
})
.receive()
.await;
```

# FileSender

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