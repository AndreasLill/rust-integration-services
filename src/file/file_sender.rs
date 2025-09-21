use std::path::{Path, PathBuf};
use tokio::{fs::OpenOptions, io::AsyncWriteExt};

#[cfg(windows)]
const NEW_LINE: &[u8] = b"\r\n";
#[cfg(not(windows))]
const NEW_LINE: &[u8] = b"\n";

pub struct FileSender {
    overwrite: bool,
    target_path: PathBuf,
}

impl FileSender {
    /// Builds a new FileSender with the full path to the target file.
    /// 
    /// If the file does not exist, it will be created.
    pub fn new<T: AsRef<Path>>(target_path: T) -> Self {
        FileSender  { 
            overwrite: false,
            target_path: target_path.as_ref().to_path_buf(),
        }
    }

    /// This will overwrite the contents of the target file.
    pub fn overwrite(mut self, overwrite: bool) -> Self {
        self.overwrite = overwrite;
        self
    }

    /// Writes the bytes to the target file.
    pub async fn send_bytes(self, bytes: &[u8]) -> anyhow::Result<()> {
        let mut file = OpenOptions::new().create(true).write(true).append(!self.overwrite).truncate(self.overwrite).open(&self.target_path).await?;

        if !self.overwrite && file.metadata().await?.len() > 0 {
            file.write_all(NEW_LINE).await?;
        }

        file.write_all(bytes).await?;
        Ok(())
    }

    /// Writes the string to the target file.
    pub async fn send_string<T: AsRef<str>>(self, string: T) -> anyhow::Result<()> {
        self.send_bytes(string.as_ref().as_bytes()).await
    }

    /// Copy the contents of the source file to the target file.
    pub async fn send_copy<T: AsRef<Path>>(self, source_path: T) -> anyhow::Result<()> {
        self.copy(source_path, false).await
    }

    /// Copy the contents of the source file to the target file and delete the source file if successful.
    pub async fn send_move<T: AsRef<Path>>(self, source_path: T) -> anyhow::Result<()> {
        self.copy(source_path, true).await
    }

    async fn copy<T: AsRef<Path>>(self, source_path: T, delete_file_on_success: bool) -> anyhow::Result<()> {
        tokio::fs::copy(&source_path, &self.target_path).await?;

        if delete_file_on_success {
            tokio::fs::remove_file(&source_path).await?;
        }

        Ok(())
    }
}