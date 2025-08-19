use aws_sdk_s3::error::SdkError;
use aws_sdk_s3::operation::get_object::GetObjectError;
use aws_sdk_s3::operation::put_object::PutObjectError;
use aws_sdk_s3::{
    Client, config::Credentials, presigning::PresigningConfig, primitives::ByteStream,
};
use aws_types::region::Region;
use axum::body::Bytes;
use std::env;
use std::time::{Duration, SystemTime};

/// Shared R2 client data
#[derive(Clone)]
pub struct R2Config {
    pub client: Client,
    pub bucket: String,
}

/// Initialize the R2 client once
pub async fn init_r2() -> R2Config {
    let endpoint = env::var("R2_ENDPOINT").expect("R2_ENDPOINT missing");
    let bucket = env::var("R2_BUCKET_NAME").expect("R2_BUCKET_NAME missing");
    let access_key = env::var("R2_ACCESS_KEY_ID").expect("R2_ACCESS_KEY_ID missing");
    let secret_key = env::var("R2_SECRET_ACCESS_KEY").expect("R2_SECRET_ACCESS_KEY missing");

    println!(
        "🔑 Initializing R2 client for bucket '{}' at endpoint '{}'",
        bucket, endpoint
    );

    let creds = Credentials::new(&access_key, &secret_key, None, None, "r2");
    let config = aws_sdk_s3::config::Builder::new()
        .region(Region::new("auto"))
        .credentials_provider(creds)
        .endpoint_url(endpoint)
        .build();

    let client = Client::from_conf(config);

    println!("✅ R2 client initialized successfully");

    R2Config {
        client,
        bucket,
    }
}

pub async fn upload_file(
    r2: &R2Config,
    bytes: Bytes,
    key: &str,
    mime_type: &str,
) -> Result<String, SdkError<PutObjectError>> {
    let byte_stream = ByteStream::from(bytes);

    // Upload to R2
    r2.client
        .put_object()
        .bucket(&r2.bucket)
        .key(key)
        .body(byte_stream)
        .content_type(mime_type)
        .send()
        .await?;

    let last_segment = key
        .rsplit('/')
        .next()
        .unwrap_or(key)
        .to_string();

    println!("✅ File uploaded to R2: {}", last_segment);

    Ok(last_segment)
}

/// Generate a signed URL
pub async fn get_signed_url(
    r2: &R2Config,
    key: &str,
    expires_secs: u64,
) -> Result<String, SdkError<GetObjectError>> {
    println!(
        "🔑 Generating signed URL for '{}', expires in {} seconds",
        key, expires_secs
    );

    let presign_config = PresigningConfig::builder()
        .expires_in(Duration::from_secs(expires_secs))
        .start_time(SystemTime::now())
        .build()
        .unwrap();

    let get_obj = r2
        .client
        .get_object()
        .bucket(&r2.bucket)
        .key(key)
        .presigned(presign_config)
        .await?;

    let url = get_obj.uri().to_string();
    println!("✅ Signed URL generated: {}", url);

    Ok(url)
}
