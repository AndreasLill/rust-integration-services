use std::path::Path;
use tokio::{fs::OpenOptions, io::AsyncWriteExt};

#[cfg(windows)]
const NEW_LINE: &[u8] = b"\r\n";
#[cfg(not(windows))]
const NEW_LINE: &[u8] = b"\n";

pub struct FileSender {
    overwrite: bool,
    target_path: String,
}

impl FileSender {
    /// Builds a new FileSender with the full path to the target file.
    /// 
    /// If the file does not exist, it will be created.
    pub fn new(target_path: &str) -> Self {
        FileSender  { 
            overwrite: false,
            target_path: target_path.to_string(),
        }
    }

    /// This will overwrite the contents of the target file.
    pub fn overwrite(mut self, overwrite: bool) -> Self {
        self.overwrite = overwrite;
        self
    }

    /// Writes the bytes to the target file.
    pub async fn send_bytes(self, bytes: &[u8]) -> tokio::io::Result<()> {
        let path = Path::new(&self.target_path);
        let mut file = OpenOptions::new().create(true).write(true).append(!self.overwrite).truncate(self.overwrite).open(path).await?;

        if !self.overwrite && file.metadata().await?.len() > 0 {
            file.write_all(NEW_LINE).await?;
        }

        file.write_all(bytes).await?;
        Ok(())
    }

    /// Writes the string to the target file.
    pub async fn send_string(self, string: &str) -> tokio::io::Result<()> {
        self.send_bytes(string.as_bytes()).await
    }

    /// Copy the contents of the source file to the target file.
    pub async fn send_copy(self, source_path: &str) -> tokio::io::Result<()> {
        self.copy(source_path, false).await
    }

    /// Copy the contents of the source file to the target file and delete the source file if successful.
    pub async fn send_move(self, source_path: &str) -> tokio::io::Result<()> {
        self.copy(source_path, true).await
    }

    async fn copy(self, source_path: &str, delete_file_on_success: bool) -> tokio::io::Result<()> {
        let source_path = Path::new(source_path);
        let target_path = Path::new(&self.target_path);
        tokio::fs::copy(source_path, target_path).await?;

        if delete_file_on_success {
            tokio::fs::remove_file(source_path).await?;
        }

        Ok(())
    }
}