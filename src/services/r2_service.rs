use aws_sdk_s3::Client;
use aws_sdk_s3::config::Credentials;
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::primitives::ByteStream;
use aws_types::region::Region;
use std::env;
use std::time::{Duration, SystemTime};
use aws_sdk_s3::error::SdkError;
use aws_sdk_s3::operation::get_object::GetObjectError;

/// Shared R2 client data
#[derive(Clone)]
pub struct R2Config {
    pub client: Client,
    pub bucket: String,
    pub base_url: String,
}

/// Initialize the R2 client once
pub async fn init_r2() -> R2Config {
    let endpoint = env::var("R2_ENDPOINT").expect("R2_ENDPOINT missing");
    let bucket = env::var("R2_BUCKET_NAME").expect("R2_BUCKET_NAME missing");
    let base_url = env::var("R2_URL").unwrap_or_default();

    let access_key = env::var("R2_ACCESS_KEY_ID").expect("R2_ACCESS_KEY_ID missing");
    let secret_key = env::var("R2_SECRET_ACCESS_KEY").expect("R2_SECRET_ACCESS_KEY missing");

    let creds = Credentials::new(&access_key, &secret_key, None, None, "r2");
    let config = aws_sdk_s3::config::Builder::new()
        .region(Region::new("auto"))
        .credentials_provider(creds)
        .endpoint_url(endpoint)
        .build();

    let client = Client::from_conf(config);

    R2Config { client, bucket, base_url }
}

/// Upload a file
pub async fn upload_file(
    r2: &R2Config,
    key: &str,
    file_bytes: Vec<u8>,
    content_type: &str,
) -> Result<String, SdkError<aws_sdk_s3::operation::put_object::PutObjectError>> {
    r2.client.put_object()
        .bucket(&r2.bucket)
        .key(key)
        .body(ByteStream::from(file_bytes))
        .content_type(content_type)
        .send()
        .await?;

    Ok(format!("{}/{}", r2.base_url, key))
}

/// Generate a signed URL
pub async fn get_signed_url(
    r2: &R2Config,
    key: &str,
    expires_secs: u64,
) -> Result<String, SdkError<GetObjectError>> {
    let presign_config = PresigningConfig::builder()
        .expires_in(Duration::from_secs(expires_secs))
        .start_time(SystemTime::now())
        .build()
        .unwrap();

    let get_obj = r2.client.get_object()
        .bucket(&r2.bucket)
        .key(key)
        .presigned(presign_config)
        .await?;

    Ok(get_obj.uri().to_string())
}