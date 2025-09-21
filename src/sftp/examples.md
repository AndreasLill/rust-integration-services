# SftpReceiver

Download all files matching regex `.*\.txt$` from `127.0.0.1:22` user remote directory `upload` to local path `/home/user/output` and delete remote files after successful download.
``` rust
let result = SftpReceiver::new("127.0.0.1:22")
.auth_basic("user", "password")
.remote_dir("upload")
.regex(r".*\.txt$")
.delete_after_download(true)
.receive_to_path("/home/user/output")
.await;
```

# SftpSender

Send a file to `127.0.0.1:22` user remote path `upload` keeping the same file name.
``` rust
let result = SftpSender::new("127.0.0.1:22")
.auth_basic("user", "password")
.remote_dir("upload")
.send_file("/home/user/input/file.txt", None)
.await;
```

Send a string as a file to `127.0.0.1:22` user remote path `upload` with file name `file.txt`.
``` rust
let result = SftpSender::new("127.0.0.1:22")
.auth_basic("user", "password")
.remote_dir("upload")
.send_bytes("HELLO WORLD".as_bytes(), "file.txt")
.await;
```