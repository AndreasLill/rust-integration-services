use std::path::Path;
use tokio::{fs::OpenOptions, io::{self, AsyncWriteExt}};

#[cfg(windows)]
const NEW_LINE: &[u8] = b"\r\n";
#[cfg(not(windows))]
const NEW_LINE: &[u8] = b"\n";

pub struct FileSender {
    file_path: String,
    append: bool,
    new_line: bool,
}

impl FileSender {
    pub fn new(file_path: &str) -> Self {
        FileSender {
            file_path: file_path.to_string(),
            append: false,
            new_line: false,
        }
    }

    pub fn append(mut self, append: bool) -> Self {
        self.append = append;
        self
    }

    pub fn new_line(mut self, new_line: bool) -> Self {
        self.new_line = new_line;
        self
    }

    pub async fn send(self, bytes: &[u8]) -> io::Result<()> {
        let path = Path::new(&self.file_path);
        let mut file = OpenOptions::new().create(true).write(true).append(self.append).open(path).await?;
        file.write_all(bytes).await?;

        if self.new_line {
            file.write_all(NEW_LINE).await?;
        }

        Ok(())
    }
}