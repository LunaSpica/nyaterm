use super::*;

#[derive(Clone)]
pub struct NativeS3Remote {
    client: zed_reqwest::blocking::Client,
    settings: S3SyncSettings,
}

impl NativeS3Remote {
    pub fn new(settings: &S3SyncSettings) -> Result<Self, CloudSyncError> {
        build_s3_signed_request(
            settings,
            S3HttpMethod::Head,
            "nyaterm/sync/latest.redb",
            &s3_payload_sha256(&[]),
            std::time::SystemTime::now(),
        )?;
        let client = zed_reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("NyaTerm")
            .build()
            .map_err(map_s3_http_error)?;
        Ok(Self {
            client,
            settings: settings.clone(),
        })
    }

    fn send(
        &self,
        method: S3HttpMethod,
        path: &str,
        body: Option<Vec<u8>>,
    ) -> Result<zed_reqwest::blocking::Response, CloudSyncError> {
        let payload_hash = body
            .as_deref()
            .map(s3_payload_sha256)
            .unwrap_or_else(|| s3_payload_sha256(&[]));
        let request = build_s3_signed_request(
            &self.settings,
            method,
            path,
            &payload_hash,
            std::time::SystemTime::now(),
        )?;
        let http_method = match method {
            S3HttpMethod::Get => Method::GET,
            S3HttpMethod::Head => Method::HEAD,
            S3HttpMethod::Put => Method::PUT,
        };
        let mut builder = self.client.request(http_method, &request.url);
        for (name, value) in &request.headers {
            builder = builder.header(name.as_str(), value.as_str());
        }
        if let Some(body) = body {
            builder = builder.body(body);
        }
        builder.send().map_err(map_s3_http_error)
    }
}

impl CloudSyncRemote for NativeS3Remote {
    fn provider(&self) -> &'static str {
        "s3"
    }

    fn create_dir(&self, _path: &str) -> Result<(), CloudSyncError> {
        Ok(())
    }

    fn read(&self, path: &str) -> Result<Vec<u8>, CloudSyncError> {
        self.read_if_exists(path)?
            .ok_or_else(|| CloudSyncError::Remote(format!("S3 object '{path}' not found")))
    }

    fn read_if_exists(&self, path: &str) -> Result<Option<Vec<u8>>, CloudSyncError> {
        let response = self.send(S3HttpMethod::Get, path, None)?;
        let status = response.status();
        if status == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if status.is_success() {
            return response
                .bytes()
                .map(|bytes| Some(bytes.to_vec()))
                .map_err(map_s3_http_error);
        }
        let body = response.text().unwrap_or_default();
        Err(CloudSyncError::Remote(format!(
            "S3 GET failed ({status}): {}",
            body.trim()
        )))
    }

    fn write(&self, path: &str, bytes: &[u8]) -> Result<(), CloudSyncError> {
        let response = self.send(S3HttpMethod::Put, path, Some(bytes.to_vec()))?;
        let status = response.status();
        if status.is_success() {
            return Ok(());
        }
        let body = response.text().unwrap_or_default();
        Err(CloudSyncError::Remote(format!(
            "S3 PUT failed ({status}): {}",
            body.trim()
        )))
    }
}
