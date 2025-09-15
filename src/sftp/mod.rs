#[cfg(feature = "sftp")]
mod sftp_auth;
#[cfg(feature = "sftp")]
pub mod sftp_receiver_event;
#[cfg(feature = "sftp")]
pub mod sftp_receiver;
#[cfg(feature = "sftp")]
pub mod sftp_sender;

#[cfg(feature = "sftp")]
#[cfg(test)]
mod test {
    use crate::sftp::{sftp_receiver::SftpReceiver, sftp_sender::SftpSender};
    
    #[tokio::test(start_paused = true)]
    async fn sftp_receiver() {
        let result = SftpReceiver::new("127.0.0.1:2222", "user").password("pass").remote_path("upload").receive_once("./test/file/out").await;
        assert!(result.is_ok());
    }

    #[tokio::test(start_paused = true)]
    async fn sftp_sender_file() {
        let result = SftpSender::new("127.0.0.1:2222", "user").password("pass").remote_path("upload").send_file("./test/file/in/TextFile1.txt").await;
        assert!(result.is_ok());
    }

    #[tokio::test(start_paused = true)]
    async fn sftp_sender_bytes() {
        let result = SftpSender::new("127.0.0.1:2222", "user").password("pass").remote_path("upload").file_name("file.txt").send_string("test").await;
        assert!(result.is_ok());
    }

}