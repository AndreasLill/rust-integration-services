use std::path::Path;
use tokio::{fs::OpenOptions, io::{AsyncReadExt, AsyncWriteExt}};

#[cfg(windows)]
const NEW_LINE: &[u8] = b"\r\n";
#[cfg(not(windows))]
const NEW_LINE: &[u8] = b"\n";

pub struct FileSender {
    append: bool,
    append_new_line: bool,
}

impl FileSender {
    pub fn new() -> Self {
        FileSender  { 
            append: false,
            append_new_line: false,
        }
    }

    pub fn append(mut self, append: bool) -> Self {
        self.append = append;
        self
    }

    pub fn append_new_line(mut self, new_line: bool) -> Self {
        self.append_new_line = new_line;
        self
    }

    pub async fn write_bytes(self, bytes: &[u8], target_path: &str) -> tokio::io::Result<()> {
        let path = Path::new(target_path);
        let mut file = OpenOptions::new().create(true).write(true).append(self.append).truncate(!self.append).open(path).await?;
        file.write_all(bytes).await?;

        if self.append_new_line {
            file.write_all(NEW_LINE).await?;
        }

        Ok(())
    }

    pub async fn write_string(self, string: &str, target_path: &str) -> tokio::io::Result<()> {
        self.write_bytes(string.as_bytes(), target_path).await
    }

    pub async fn copy_file(self, source_path: &str, target_path: &str) -> tokio::io::Result<()> {
        self.copy(source_path, target_path, false).await
    }

    pub async fn move_file(self, source_path: &str, target_path: &str) -> tokio::io::Result<()> {
        self.copy(source_path, target_path, true).await
    }

    async fn copy(self, source_path: &str, target_path: &str, delete_file_on_success: bool) -> tokio::io::Result<()> {
        let source_path = Path::new(source_path);
        let target_path = Path::new(target_path);
        let mut source = OpenOptions::new().read(true).open(source_path).await?;
        let mut target = OpenOptions::new().create(true).write(true).append(self.append).truncate(!self.append).open(target_path).await?;
        let mut buffer = vec![0u8; 1024 * 1024];

        loop {
            let bytes = source.read(&mut buffer).await?;
            if bytes == 0 {
                break;
            }
            target.write_all(&buffer[..bytes]).await?;
        }
        target.flush().await?;

        if self.append_new_line {
            target.write_all(NEW_LINE).await?;
        }

        if delete_file_on_success {
            drop(source);
            tokio::fs::remove_file(source_path).await?;
        }

        Ok(())
    }
}