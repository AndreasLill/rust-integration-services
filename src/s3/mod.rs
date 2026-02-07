#[cfg(feature = "s3")]
pub mod s3_client;
#[cfg(feature = "s3")]
pub mod s3_client_bucket;
#[cfg(feature = "s3")]
pub mod s3_client_config;

#[cfg(feature = "s3")]
#[cfg(test)]
mod test;