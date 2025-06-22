pub struct Error;

impl Error {
    pub fn tokio_io<T: AsRef<str>>(message: T) -> tokio::io::Error {
        tokio::io::Error::new(tokio::io::ErrorKind::Other, message.as_ref())
    }

    pub fn std_io<T: AsRef<str>>(message: T) -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::Other, message.as_ref())
    }
}