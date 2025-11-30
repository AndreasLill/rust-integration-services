pub enum S3Auth {
    None,
    Basic {
        user: String,
        password: String,
    },
}

impl S3Auth {
    pub fn basic<T: AsRef<str>>(user: T, password: T) -> Self {
        S3Auth::Basic {
            user: user.as_ref().to_owned(),
            password: password.as_ref().to_owned()
        }
    }
}