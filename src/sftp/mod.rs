#[cfg(feature = "sftp")]
mod sftp;
#[cfg(feature = "sftp")]
mod sftp_authentication;
#[cfg(feature = "sftp")]
mod sftp_auth_basic;
#[cfg(feature = "sftp")]
mod sftp_auth_key;
#[cfg(feature = "sftp")]
pub mod ssh_client;
#[cfg(feature = "sftp")]
pub mod sftp_receiver;
#[cfg(feature = "sftp")]
pub mod sftp_sender;

#[cfg(feature = "sftp")]
#[cfg(test)]
mod test {
    use crate::sftp::{sftp_receiver::SftpReceiver, sftp_sender::SftpSender};

    #[tokio::test]
    async fn sftp_receiver() {
        let result = SftpReceiver::new("127.0.0.1:2222").auth_basic("user", "password").remote_dir("upload").regex(r".*\.txt$").receive_to_path("/home/andreas/output").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn sftp_sender() {
        let result = SftpSender::new("127.0.0.1:2222").auth_basic("user", "password").remote_dir("upload").send_bytes("HELLO WORLD".as_bytes(), "file.txt").await;
        assert!(result.is_ok());
    }
}