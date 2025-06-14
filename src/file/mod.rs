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
        FileReceiver::new("./test/file/in")
        .filter(r"^[^\.]+?\.[^\.]+$", async move |_, path| {
            let target_path = &format!("./test/file/out/{}", path.file_name().unwrap().to_str().unwrap());
            let source_path = path.to_str().unwrap();
            FileSender::new(target_path).overwrite(true).copy_from(source_path).await.unwrap();
        })
        .run_once().await;
    }
}