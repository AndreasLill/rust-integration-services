#[cfg(feature = "file")]
pub mod file_receiver;
#[cfg(feature = "file")]
pub mod file_sender;

#[cfg(test)]
mod test {
    use crate::file::{file_receiver::FileReceiver, file_sender::FileSender};

    #[tokio::test(start_paused = true)]
    async fn file_sender_write() {
        let result = FileSender::new("./test/file/out/file.txt").overwrite(true).write_string("test").await;
        assert!(result.is_ok());
    }

    #[tokio::test(start_paused = true)]
    async fn file_sender_copy() {
        let result = FileSender::new("./test/file/out/TextFile1.txt").overwrite(true).copy_from("./test/file/in/TextFile1.txt").await;
        assert!(result.is_ok());
    }

    #[tokio::test(start_paused = true)]
    async fn file_receiver() {
        let result = FileReceiver::new("./test/file/in")
        .filter(r"^.+\.[^./\\]+$", async move |_uuid, _path| {
        })
        .read_directory()
        .await;
        assert!(result.is_ok());
    }
}