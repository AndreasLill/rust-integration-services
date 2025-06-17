#[cfg(feature = "sftp")]
pub mod sftp_sender;

#[cfg(feature = "sftp")]
#[cfg(test)]
mod test {
    use crate::sftp::sftp_sender::SftpSender;

    #[tokio::test(start_paused = true)]
    async fn sftp_sender_file() {
        let result = SftpSender::new("127.0.0.1:2222", "user").auth_password("pass").remote_dir("upload").send_file("./test/file/in/TextFile1.txt").await;
        assert!(result.is_ok());
    }

    #[tokio::test(start_paused = true)]
    async fn sftp_sender_bytes() {
        let result = SftpSender::new("127.0.0.1:2222", "user").auth_password("pass").remote_dir("upload").file_name("file.txt").send_string("test").await;
        assert!(result.is_ok());
    }
}