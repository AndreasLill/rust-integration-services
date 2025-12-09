#[cfg(feature = "s3")]
pub mod s3_client;
#[cfg(feature = "s3")]
pub mod s3_client_config;
#[cfg(feature = "s3")]
pub mod s3_client_get_object;
#[cfg(feature = "s3")]
pub mod s3_client_put_object;
#[cfg(feature = "s3")]
pub mod s3_client_delete_object;

#[cfg(feature = "s3")]
#[cfg(test)]
mod test;