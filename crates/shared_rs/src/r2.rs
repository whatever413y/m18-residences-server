//! [`FileStore`] on the Worker's R2 bucket binding.
use worker::{Bucket, HttpMetadata, send::SendFuture, send::SendWrapper};

use crate::files::{FileError, FileInfo, FileStore, StoredFile};

pub struct R2FileStore(SendWrapper<Bucket>);

impl R2FileStore {
    pub fn new(bucket: Bucket) -> Self {
        Self(SendWrapper::new(bucket))
    }
}

fn r2_err(action: &str, key: &str, err: worker::Error) -> FileError {
    FileError(format!("R2 {action} {key}: {err}"))
}

#[async_trait::async_trait]
impl FileStore for R2FileStore {
    async fn put(&self, key: &str, bytes: Vec<u8>, content_type: &str) -> Result<(), FileError> {
        let metadata = HttpMetadata {
            content_type: Some(content_type.into()),
            ..Default::default()
        };
        SendFuture::new(self.0.0.put(key, bytes).http_metadata(metadata).execute())
            .await
            .map_err(|e| r2_err("put", key, e))?;
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Option<StoredFile>, FileError> {
        SendFuture::new(async {
            let Some(object) = self
                .0
                .0
                .get(key)
                .execute()
                .await
                .map_err(|e| r2_err("get", key, e))?
            else {
                return Ok(None);
            };
            let content_type = object.http_metadata().content_type;
            let bytes = match object.body() {
                Some(body) => body.bytes().await.map_err(|e| r2_err("read", key, e))?,
                None => Vec::new(),
            };
            Ok(Some(StoredFile {
                bytes,
                content_type,
            }))
        })
        .await
    }

    async fn head(&self, key: &str) -> Result<Option<FileInfo>, FileError> {
        let object = SendFuture::new(self.0.0.head(key))
            .await
            .map_err(|e| r2_err("head", key, e))?;
        Ok(object.map(|o| FileInfo {
            content_type: o.http_metadata().content_type,
        }))
    }

    async fn delete(&self, key: &str) -> Result<(), FileError> {
        SendFuture::new(self.0.0.delete(key))
            .await
            .map_err(|e| r2_err("delete", key, e))
    }
}
