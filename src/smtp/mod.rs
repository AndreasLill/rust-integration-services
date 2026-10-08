#[cfg(feature = "smtp")]
pub mod smtp_security;
#[cfg(feature = "smtp")]
pub mod smtp_message;
#[cfg(feature = "smtp")]
pub mod smtp_client_config;
#[cfg(feature = "smtp")]
pub mod smtp_client;

#[cfg(feature = "smtp")]
#[cfg(test)]
mod test;