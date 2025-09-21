#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpStatus {
    // 1xx Informational
    Continue = 100,
    SwitchingProtocols = 101,
    EarlyHints = 103,

    // 2xx Success
    Ok = 200,
    Created = 201,
    Accepted = 202,
    NoContent = 204,
    PartialContent = 206,

    // 3xx Redirection
    MovedPermanently = 301,
    Found = 302,
    SeeOther = 303,
    NotModified = 304,
    TemporaryRedirect = 307,
    PermanentRedirect = 308,

    // 4xx Client Error
    BadRequest = 400,
    Unauthorized = 401,
    Forbidden = 403,
    NotFound = 404,
    MethodNotAllowed = 405,

    // 5xx Server Error
    InternalServerError = 500,
    NotImplemented = 501,
    BadGateway = 502,
    ServiceUnavailable = 503,
    GatewayTimeout = 504,
}

impl HttpStatus {
    pub fn code(self) -> u16 {
        self as u16
    }

    pub fn text(self) -> &'static str {
        match self {
            // 1xx
            HttpStatus::Continue => "Continue",
            HttpStatus::SwitchingProtocols => "Switching Protocols",
            HttpStatus::EarlyHints => "Early Hints",

            // 2xx
            HttpStatus::Ok => "OK",
            HttpStatus::Created => "Created",
            HttpStatus::Accepted => "Accepted",
            HttpStatus::NoContent => "No Content",
            HttpStatus::PartialContent => "Partial Content",

            // 3xx
            HttpStatus::MovedPermanently => "Moved Permanently",
            HttpStatus::Found => "Found",
            HttpStatus::SeeOther => "See Other",
            HttpStatus::NotModified => "Not Modified",
            HttpStatus::TemporaryRedirect => "Temporary Redirect",
            HttpStatus::PermanentRedirect => "Permanent Redirect",

            // 4xx
            HttpStatus::BadRequest => "Bad Request",
            HttpStatus::Unauthorized => "Unauthorized",
            HttpStatus::Forbidden => "Forbidden",
            HttpStatus::NotFound => "Not Found",
            HttpStatus::MethodNotAllowed => "Method Not Allowed",

            // 5xx
            HttpStatus::InternalServerError => "Internal Server Error",
            HttpStatus::NotImplemented => "Not Implemented",
            HttpStatus::BadGateway => "Bad Gateway",
            HttpStatus::ServiceUnavailable => "Service Unavailable",
            HttpStatus::GatewayTimeout => "Gateway Timeout",
        }
    }

    pub fn from_code(code: u16) -> anyhow::Result<Self> {
        match code {
            100 => Ok(HttpStatus::Continue),
            101 => Ok(HttpStatus::SwitchingProtocols),
            103 => Ok(HttpStatus::EarlyHints),

            200 => Ok(HttpStatus::Ok),
            201 => Ok(HttpStatus::Created),
            202 => Ok(HttpStatus::Accepted),
            204 => Ok(HttpStatus::NoContent),
            206 => Ok(HttpStatus::PartialContent),

            301 => Ok(HttpStatus::MovedPermanently),
            302 => Ok(HttpStatus::Found),
            303 => Ok(HttpStatus::SeeOther),
            304 => Ok(HttpStatus::NotModified),
            307 => Ok(HttpStatus::TemporaryRedirect),
            308 => Ok(HttpStatus::PermanentRedirect),

            400 => Ok(HttpStatus::BadRequest),
            401 => Ok(HttpStatus::Unauthorized),
            403 => Ok(HttpStatus::Forbidden),
            404 => Ok(HttpStatus::NotFound),
            405 => Ok(HttpStatus::MethodNotAllowed),

            500 => Ok(HttpStatus::InternalServerError),
            501 => Ok(HttpStatus::NotImplemented),
            502 => Ok(HttpStatus::BadGateway),
            503 => Ok(HttpStatus::ServiceUnavailable),
            504 => Ok(HttpStatus::GatewayTimeout),

            _ => Err(anyhow::anyhow!("Invalid HTTP status code")),
        }
    }
}