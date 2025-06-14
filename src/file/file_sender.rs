use std::path::Path;
use tokio::{fs::OpenOptions, io::{AsyncReadExt, AsyncWriteExt}};

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
    pub async fn write_bytes(self, bytes: &[u8]) -> tokio::io::Result<()> {
        let path = Path::new(&self.target_path);
        let mut file = OpenOptions::new().create(true).read(true).write(true).append(!self.overwrite).truncate(self.overwrite).open(path).await?;

        if !self.overwrite {
            let metadata = file.metadata().await?;
            if metadata.len() > 0 {
                file.write_all(NEW_LINE).await?;
            }
        }

        file.write_all(bytes).await?;
        Ok(())
    }

    /// Writes the string to the target file.
    pub async fn write_string(self, string: &str) -> tokio::io::Result<()> {
        self.write_bytes(string.as_bytes()).await
    }

    /// Copy the contents of the source file to the target file.
    pub async fn copy_from(mut self, source_path: &str) -> tokio::io::Result<()> {
        self.overwrite = true;
        self.copy(source_path, false).await
    }

    /// Copy the contents of the source file to the target file and delete the source file if successful.
    pub async fn move_from(mut self, source_path: &str) -> tokio::io::Result<()> {
        self.overwrite = true;
        self.copy(source_path, true).await
    }

    async fn copy(self, source_path: &str, delete_file_on_success: bool) -> tokio::io::Result<()> {
        let source_path = Path::new(source_path);
        let target_path = Path::new(&self.target_path);
        let mut source = OpenOptions::new().read(true).open(source_path).await?;
        let mut target = OpenOptions::new().create(true).read(true).write(true).append(!self.overwrite).truncate(self.overwrite).open(target_path).await?;
        let mut buffer = vec![0u8; 1024 * 1024];

        loop {
            let bytes = source.read(&mut buffer).await?;
            if bytes == 0 {
                break;
            }
            target.write_all(&buffer[..bytes]).await?;
        }
        target.flush().await?;

        if delete_file_on_success {
            drop(source);
            tokio::fs::remove_file(source_path).await?;
        }

        Ok(())
    }
}