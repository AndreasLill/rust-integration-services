use std::path::Path;
use tokio::{fs::OpenOptions, io::{AsyncReadExt, AsyncWriteExt}};

#[cfg(windows)]
const NEW_LINE: &[u8] = b"\r\n";
#[cfg(not(windows))]
const NEW_LINE: &[u8] = b"\n";

pub struct FileSender {
    append: bool,
    add_new_line: bool,
    delete_source_file: bool,
}

impl FileSender {
    pub fn new() -> Self {
        FileSender  { 
            append: false,
            add_new_line: false,
            delete_source_file: false,
        }
    }

    pub fn append(mut self, append: bool) -> Self {
        self.append = append;
        self
    }

    pub fn new_line(mut self, new_line: bool) -> Self {
        self.add_new_line = new_line;
        self
    }

    pub fn delete_source(mut self, delete_source: bool) -> Self {
        self.delete_source_file = delete_source;
        self
    }

    pub async fn write_bytes(self, bytes: &[u8], target_path: &str) -> tokio::io::Result<()> {
        let path = Path::new(target_path);
        let mut file = OpenOptions::new().create(true).write(true).append(self.append).truncate(!self.append).open(path).await?;
        file.write_all(bytes).await?;

        if self.add_new_line {
            file.write_all(NEW_LINE).await?;
        }

        Ok(())
    }

    pub async fn write_string(self, string: &str, target_path: &str) -> tokio::io::Result<()> {
        self.write_bytes(string.as_bytes(), target_path).await
    }

    pub async fn move_file(self, source_path: &str, target_path: &str) -> tokio::io::Result<()> {
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

        if self.add_new_line {
            target.write_all(NEW_LINE).await?;
        }

        if self.delete_source_file {
            drop(source);
            tokio::fs::remove_file(source_path).await?;
        }

        Ok(())
    }
}