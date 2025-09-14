# SftpReceiver

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
        SftpReceiverEventSignal::OnError(_uuid, err) => println!("Download failed: {}", err),
    }
})
.receive_once("/home/user/files")
.await;
```

By setting a custom regex to the receiver, the sftp receiver will only download files matching this regex.  
For examplem this will only download files starting with "ABC_".

``` rust
.regex("^ABC_.+\.[^./\\]+$")
```


# SftpSender

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