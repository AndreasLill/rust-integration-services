#[cfg(feature = "file")]
pub mod file_sender;

#[cfg(test)]
mod test {
    use crate::file::file_sender::FileSender;

    #[tokio::test(start_paused = true)]
    async fn file_sender() {
        let result = FileSender::new("./test.txt").send(b"test").await;
        assert!(result.is_ok());
        let result = tokio::fs::remove_file("./test.txt").await;
        assert!(result.is_ok());
    }
}