use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use nyaterm_core::{
    AliyunDriveSyncSettings, CloudSyncError, CloudSyncRemote, OAuthDriveSyncSettings, S3HttpMethod,
    S3SyncSettings, SnippetHttpClient, SnippetHttpMethod, SnippetHttpRequest, SnippetHttpResponse,
    WebdavSyncSettings, build_s3_signed_request, drive_remote_segments, google_drive_query_literal,
    s3_payload_sha256,
};
use serde_json::json;
use sha2::{Digest as ShaDigest, Sha256};
use zed_reqwest::blocking::RequestBuilder;
use zed_reqwest::header::{AUTHORIZATION, CONTENT_TYPE, WWW_AUTHENTICATE};
use zed_reqwest::{Method, StatusCode};

const GOOGLE_DRIVE_FILES_URL: &str = "https://www.googleapis.com/drive/v3/files";
const GOOGLE_DRIVE_UPLOAD_FILES_URL: &str = "https://www.googleapis.com/upload/drive/v3/files";
const GOOGLE_OAUTH_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_DRIVE_FOLDER_MIME: &str = "application/vnd.google-apps.folder";
const MICROSOFT_GRAPH_BASE_URL: &str = "https://graph.microsoft.com/v1.0";
const MICROSOFT_OAUTH_TOKEN_URL: &str =
    "https://login.microsoftonline.com/common/oauth2/v2.0/token";
const ALIYUN_DRIVE_BASE_URL: &str = "https://openapi.alipan.com";

#[derive(Clone)]
pub struct NativeSnippetHttpClient {
    client: zed_reqwest::blocking::Client,
}

impl NativeSnippetHttpClient {
    pub fn new() -> Result<Self, CloudSyncError> {
        let client = zed_reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("NyaTerm")
            .build()
            .map_err(map_http_error)?;
        Ok(Self { client })
    }
}

impl SnippetHttpClient for NativeSnippetHttpClient {
    fn send(&self, request: SnippetHttpRequest) -> Result<SnippetHttpResponse, CloudSyncError> {
        let mut builder = match request.method {
            SnippetHttpMethod::Get => self.client.get(&request.url),
            SnippetHttpMethod::Patch => self.client.patch(&request.url),
        };

        if !request.query.is_empty() {
            builder = builder.query(&request.query);
        }
        for (name, value) in &request.headers {
            builder = builder.header(name.as_str(), value.as_str());
        }
        if let Some(body) = request.json_body {
            builder = builder.json(&body);
        }

        let response = builder.send().map_err(map_http_error)?;
        let status = response.status().as_u16();
        let body = response.text().map_err(map_http_error)?;
        Ok(SnippetHttpResponse { status, body })
    }
}

fn map_http_error(error: zed_reqwest::Error) -> CloudSyncError {
    if error.is_timeout() {
        CloudSyncError::Io(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            format!("snippet HTTP operation timed out: {error}"),
        ))
    } else {
        CloudSyncError::Remote(format!("snippet HTTP request failed: {error}"))
    }
}

#[derive(Clone)]
pub struct NativeWebdavRemote {
    client: zed_reqwest::blocking::Client,
    endpoint: String,
    root: String,
    username: String,
    password: Option<String>,
}

impl NativeWebdavRemote {
    pub fn new(settings: &WebdavSyncSettings) -> Result<Self, CloudSyncError> {
        let endpoint = normalize_endpoint(&settings.endpoint);
        if endpoint.is_empty() {
            return Err(CloudSyncError::Remote(
                "WebDAV endpoint is required".to_string(),
            ));
        }
        let client = zed_reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("NyaTerm")
            .build()
            .map_err(map_webdav_http_error)?;
        Ok(Self {
            client,
            endpoint,
            root: trim_remote_path(&settings.root),
            username: settings.username.trim().to_string(),
            password: settings.password.clone(),
        })
    }

    fn url_for(&self, path: &str) -> String {
        let mut parts = Vec::new();
        if !self.root.is_empty() {
            parts.push(self.root.as_str());
        }
        let path = trim_remote_path(path);
        if !path.is_empty() {
            parts.push(path.as_str());
        }
        if parts.is_empty() {
            self.endpoint.clone()
        } else {
            format!("{}/{}", self.endpoint, parts.join("/"))
        }
    }

    fn send(
        &self,
        method: Method,
        url: &str,
        body: Option<Vec<u8>>,
    ) -> Result<zed_reqwest::blocking::Response, CloudSyncError> {
        let response = self.send_once(method.clone(), url, body.clone(), None)?;
        if response.status() != StatusCode::UNAUTHORIZED {
            return Ok(response);
        }
        let Some(challenge) = digest_challenge(&response) else {
            return Ok(response);
        };
        if self.username.is_empty() || self.password.as_deref().unwrap_or_default().is_empty() {
            return Ok(response);
        }
        let auth = build_digest_authorization(
            &challenge,
            &self.username,
            self.password.as_deref().unwrap_or_default(),
            method.as_str(),
            path_and_query(url),
            &webdav_cnonce(),
            "00000001",
        )?;
        self.send_once(method, url, body, Some(auth))
    }

    fn send_once(
        &self,
        method: Method,
        url: &str,
        body: Option<Vec<u8>>,
        authorization: Option<String>,
    ) -> Result<zed_reqwest::blocking::Response, CloudSyncError> {
        let mut request = self.client.request(method, url);
        if let Some(authorization) = authorization {
            request = request.header(AUTHORIZATION, authorization);
        } else if !self.username.is_empty() {
            request = request.basic_auth(&self.username, self.password.as_deref());
        }
        if let Some(body) = body {
            request = request.body(body);
        }
        request.send().map_err(map_webdav_http_error)
    }
}

impl CloudSyncRemote for NativeWebdavRemote {
    fn provider(&self) -> &'static str {
        "webdav"
    }

    fn create_dir(&self, path: &str) -> Result<(), CloudSyncError> {
        let mut current = String::new();
        for segment in trim_remote_path(path)
            .split('/')
            .filter(|value| !value.is_empty())
        {
            if !current.is_empty() {
                current.push('/');
            }
            current.push_str(segment);
            let url = self.url_for(&current);
            let method = Method::from_bytes(b"MKCOL").map_err(|error| {
                CloudSyncError::Remote(format!("failed to build WebDAV MKCOL method: {error}"))
            })?;
            let response = self.send(method, &url, None)?;
            match response.status() {
                StatusCode::CREATED | StatusCode::OK | StatusCode::METHOD_NOT_ALLOWED => {}
                status if status.as_u16() == 409 => {}
                status if status.is_success() => {}
                status => {
                    let body = response.text().unwrap_or_default();
                    return Err(CloudSyncError::Remote(format!(
                        "WebDAV MKCOL failed ({status}): {}",
                        body.trim()
                    )));
                }
            }
        }
        Ok(())
    }

    fn read(&self, path: &str) -> Result<Vec<u8>, CloudSyncError> {
        let url = self.url_for(path);
        let response = self.send(Method::GET, &url, None)?;
        let status = response.status();
        if status.is_success() {
            return response
                .bytes()
                .map(|bytes| bytes.to_vec())
                .map_err(map_webdav_http_error);
        }
        let body = response.text().unwrap_or_default();
        Err(CloudSyncError::Remote(format!(
            "WebDAV GET failed ({status}): {}",
            body.trim()
        )))
    }

    fn read_if_exists(&self, path: &str) -> Result<Option<Vec<u8>>, CloudSyncError> {
        let url = self.url_for(path);
        let response = self.send(Method::GET, &url, None)?;
        let status = response.status();
        if status == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if status.is_success() {
            return response
                .bytes()
                .map(|bytes| Some(bytes.to_vec()))
                .map_err(map_webdav_http_error);
        }
        let body = response.text().unwrap_or_default();
        Err(CloudSyncError::Remote(format!(
            "WebDAV GET failed ({status}): {}",
            body.trim()
        )))
    }

    fn write(&self, path: &str, bytes: &[u8]) -> Result<(), CloudSyncError> {
        let parent = path
            .rsplit_once('/')
            .map(|(parent, _)| parent)
            .unwrap_or("");
        if !parent.is_empty() {
            self.create_dir(parent)?;
        }
        let url = self.url_for(path);
        let response = self.send(Method::PUT, &url, Some(bytes.to_vec()))?;
        let status = response.status();
        if status.is_success() {
            return Ok(());
        }
        let body = response.text().unwrap_or_default();
        Err(CloudSyncError::Remote(format!(
            "WebDAV PUT failed ({status}): {}",
            body.trim()
        )))
    }
}

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

pub struct NativeGoogleDriveRemote {
    client: zed_reqwest::blocking::Client,
    root: String,
    access_token: Mutex<String>,
    refresh_token: Option<String>,
    client_id: Option<String>,
    client_secret: Option<String>,
}

impl NativeGoogleDriveRemote {
    pub fn new(settings: &OAuthDriveSyncSettings) -> Result<Self, CloudSyncError> {
        let client = zed_reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("NyaTerm")
            .build()
            .map_err(map_google_drive_http_error)?;
        let remote = Self {
            client,
            root: trim_remote_path(&settings.root),
            access_token: Mutex::new(trim_optional_secret(settings.access_token.as_deref())),
            refresh_token: trim_optional(settings.refresh_token.as_deref()),
            client_id: trim_optional(settings.client_id.as_deref()),
            client_secret: trim_optional(settings.client_secret.as_deref()),
        };

        if remote.bearer_token().is_empty() {
            if remote.can_refresh_access_token() {
                remote.refresh_access_token()?;
            } else {
                return Err(CloudSyncError::Remote(
                    "Google Drive access token is required".to_string(),
                ));
            }
        }

        Ok(remote)
    }

    fn bearer_token(&self) -> String {
        self.access_token
            .lock()
            .expect("google drive access token lock")
            .clone()
    }

    fn can_refresh_access_token(&self) -> bool {
        self.refresh_token
            .as_deref()
            .is_some_and(|value| !value.is_empty())
            && self
                .client_id
                .as_deref()
                .is_some_and(|value| !value.is_empty())
            && self
                .client_secret
                .as_deref()
                .is_some_and(|value| !value.is_empty())
    }

    fn refresh_access_token(&self) -> Result<(), CloudSyncError> {
        let refresh_token = self
            .refresh_token
            .as_deref()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                CloudSyncError::Remote("Google Drive refresh token is required".to_string())
            })?;
        let client_id = self
            .client_id
            .as_deref()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                CloudSyncError::Remote("Google Drive client ID is required".to_string())
            })?;
        let client_secret = self
            .client_secret
            .as_deref()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                CloudSyncError::Remote("Google Drive client secret is required".to_string())
            })?;
        let body = form_urlencoded(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ]);
        let response = self
            .client
            .post(GOOGLE_OAUTH_TOKEN_URL)
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .map_err(map_google_drive_http_error)?;
        let status = response.status();
        let body = response.text().map_err(map_google_drive_http_error)?;
        if !status.is_success() {
            return Err(CloudSyncError::Remote(format!(
                "Google Drive token refresh failed ({status}): {}",
                body.trim()
            )));
        }
        let value: serde_json::Value = serde_json::from_str(&body)?;
        let access_token = value
            .get("access_token")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                CloudSyncError::Remote(
                    "Google Drive token refresh response did not include access_token".to_string(),
                )
            })?;
        *self
            .access_token
            .lock()
            .expect("google drive access token lock") = access_token.to_string();
        Ok(())
    }

    fn send_authorized(
        &self,
        build: impl Fn(&zed_reqwest::blocking::Client, &str) -> RequestBuilder,
    ) -> Result<zed_reqwest::blocking::Response, CloudSyncError> {
        if self.bearer_token().is_empty() && self.can_refresh_access_token() {
            self.refresh_access_token()?;
        }
        let response = build(&self.client, &self.bearer_token())
            .send()
            .map_err(map_google_drive_http_error)?;
        if response.status() != StatusCode::UNAUTHORIZED || !self.can_refresh_access_token() {
            return Ok(response);
        }
        self.refresh_access_token()?;
        build(&self.client, &self.bearer_token())
            .send()
            .map_err(map_google_drive_http_error)
    }

    fn create_folder(&self, parent_id: &str, name: &str) -> Result<String, CloudSyncError> {
        let metadata = json!({
            "name": name,
            "mimeType": GOOGLE_DRIVE_FOLDER_MIME,
            "parents": [parent_id],
        });
        let response = self.send_authorized(|client, token| {
            client
                .post(GOOGLE_DRIVE_FILES_URL)
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .json(&metadata)
                .query(&[("fields", "id")])
        })?;
        let status = response.status();
        let body = response.text().map_err(map_google_drive_http_error)?;
        if !status.is_success() {
            return Err(CloudSyncError::Remote(format!(
                "Google Drive create folder failed ({status}): {}",
                body.trim()
            )));
        }
        google_drive_json_id(&body, "created folder")
    }

    fn ensure_folder_segments(&self, segments: &[String]) -> Result<String, CloudSyncError> {
        let mut parent_id = "root".to_string();
        for segment in segments {
            if segment.is_empty() {
                continue;
            }
            parent_id = if let Some(folder) = self.find_child(&parent_id, segment, true)? {
                folder.id
            } else {
                self.create_folder(&parent_id, segment)?
            };
        }
        Ok(parent_id)
    }

    fn locate_file(&self, path: &str) -> Result<Option<GoogleDriveFile>, CloudSyncError> {
        let segments = drive_remote_segments(&self.root, path);
        let Some((file_name, parent_segments)) = segments.split_last() else {
            return Ok(None);
        };
        let parent_id = match self.locate_folder_segments(parent_segments)? {
            Some(parent_id) => parent_id,
            None => return Ok(None),
        };
        self.find_child(&parent_id, file_name, false)
    }

    fn locate_folder_segments(
        &self,
        segments: &[String],
    ) -> Result<Option<String>, CloudSyncError> {
        let mut parent_id = "root".to_string();
        for segment in segments {
            let Some(folder) = self.find_child(&parent_id, segment, true)? else {
                return Ok(None);
            };
            parent_id = folder.id;
        }
        Ok(Some(parent_id))
    }

    fn find_child(
        &self,
        parent_id: &str,
        name: &str,
        folder_only: bool,
    ) -> Result<Option<GoogleDriveFile>, CloudSyncError> {
        let mut query = format!(
            "name = {} and {} in parents and trashed = false",
            google_drive_query_literal(name),
            google_drive_query_literal(parent_id)
        );
        if folder_only {
            query.push_str(&format!(
                " and mimeType = {}",
                google_drive_query_literal(GOOGLE_DRIVE_FOLDER_MIME)
            ));
        }
        let response = self.send_authorized(|client, token| {
            client
                .get(GOOGLE_DRIVE_FILES_URL)
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .query(&[
                    ("q", query.as_str()),
                    ("pageSize", "10"),
                    ("fields", "files(id,name,mimeType)"),
                ])
        })?;
        let status = response.status();
        let body = response.text().map_err(map_google_drive_http_error)?;
        if !status.is_success() {
            return Err(CloudSyncError::Remote(format!(
                "Google Drive file lookup failed ({status}): {}",
                body.trim()
            )));
        }
        let value: serde_json::Value = serde_json::from_str(&body)?;
        let files = value
            .get("files")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| {
                CloudSyncError::Remote(
                    "Google Drive file lookup response is missing files".to_string(),
                )
            })?;
        for file in files {
            let id = file
                .get("id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .trim();
            let mime_type = file
                .get("mimeType")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .trim();
            if id.is_empty() {
                continue;
            }
            if folder_only || mime_type != GOOGLE_DRIVE_FOLDER_MIME {
                return Ok(Some(GoogleDriveFile { id: id.to_string() }));
            }
        }
        Ok(None)
    }

    fn read_file_content(&self, file_id: &str) -> Result<Option<Vec<u8>>, CloudSyncError> {
        let url = format!("{GOOGLE_DRIVE_FILES_URL}/{file_id}");
        let response = self.send_authorized(|client, token| {
            client
                .get(&url)
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .query(&[("alt", "media")])
        })?;
        let status = response.status();
        if status == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if status.is_success() {
            return response
                .bytes()
                .map(|bytes| Some(bytes.to_vec()))
                .map_err(map_google_drive_http_error);
        }
        let body = response.text().unwrap_or_default();
        Err(CloudSyncError::Remote(format!(
            "Google Drive media download failed ({status}): {}",
            body.trim()
        )))
    }

    fn create_file(&self, parent_id: &str, name: &str, bytes: &[u8]) -> Result<(), CloudSyncError> {
        let (boundary, body) = google_drive_multipart_body(parent_id, name, bytes)?;
        let content_type = format!("multipart/related; boundary={boundary}");
        let response = self.send_authorized(|client, token| {
            client
                .post(GOOGLE_DRIVE_UPLOAD_FILES_URL)
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .header(CONTENT_TYPE, content_type.clone())
                .query(&[("uploadType", "multipart"), ("fields", "id")])
                .body(body.clone())
        })?;
        google_drive_expect_success(response, "Google Drive file create")
    }

    fn update_file_content(&self, file_id: &str, bytes: &[u8]) -> Result<(), CloudSyncError> {
        let url = format!("{GOOGLE_DRIVE_UPLOAD_FILES_URL}/{file_id}");
        let response = self.send_authorized(|client, token| {
            client
                .patch(&url)
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .header(CONTENT_TYPE, "application/octet-stream")
                .query(&[("uploadType", "media")])
                .body(bytes.to_vec())
        })?;
        google_drive_expect_success(response, "Google Drive file update")
    }
}

impl CloudSyncRemote for NativeGoogleDriveRemote {
    fn provider(&self) -> &'static str {
        "google_drive"
    }

    fn create_dir(&self, path: &str) -> Result<(), CloudSyncError> {
        let segments = drive_remote_segments(&self.root, path);
        self.ensure_folder_segments(&segments)?;
        Ok(())
    }

    fn read(&self, path: &str) -> Result<Vec<u8>, CloudSyncError> {
        self.read_if_exists(path)?
            .ok_or_else(|| CloudSyncError::Remote(format!("Google Drive file '{path}' not found")))
    }

    fn read_if_exists(&self, path: &str) -> Result<Option<Vec<u8>>, CloudSyncError> {
        let Some(file) = self.locate_file(path)? else {
            return Ok(None);
        };
        self.read_file_content(&file.id)
    }

    fn write(&self, path: &str, bytes: &[u8]) -> Result<(), CloudSyncError> {
        let segments = drive_remote_segments(&self.root, path);
        let Some((file_name, parent_segments)) = segments.split_last() else {
            return Err(CloudSyncError::InvalidRemotePath {
                path: path.to_string(),
            });
        };
        let parent_id = self.ensure_folder_segments(parent_segments)?;
        if let Some(existing) = self.find_child(&parent_id, file_name, false)? {
            self.update_file_content(&existing.id, bytes)
        } else {
            self.create_file(&parent_id, file_name, bytes)
        }
    }
}

pub struct NativeOneDriveRemote {
    client: zed_reqwest::blocking::Client,
    root: String,
    access_token: Mutex<String>,
    refresh_token: Option<String>,
    client_id: Option<String>,
    client_secret: Option<String>,
}

impl NativeOneDriveRemote {
    pub fn new(settings: &OAuthDriveSyncSettings) -> Result<Self, CloudSyncError> {
        let client = zed_reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("NyaTerm")
            .build()
            .map_err(map_onedrive_http_error)?;
        let remote = Self {
            client,
            root: trim_remote_path(&settings.root),
            access_token: Mutex::new(trim_optional_secret(settings.access_token.as_deref())),
            refresh_token: trim_optional(settings.refresh_token.as_deref()),
            client_id: trim_optional(settings.client_id.as_deref()),
            client_secret: trim_optional(settings.client_secret.as_deref()),
        };

        if remote.bearer_token().is_empty() {
            if remote.can_refresh_access_token() {
                remote.refresh_access_token()?;
            } else {
                return Err(CloudSyncError::Remote(
                    "OneDrive access token is required".to_string(),
                ));
            }
        }

        Ok(remote)
    }

    fn bearer_token(&self) -> String {
        self.access_token
            .lock()
            .expect("onedrive access token lock")
            .clone()
    }

    fn can_refresh_access_token(&self) -> bool {
        self.refresh_token
            .as_deref()
            .is_some_and(|value| !value.is_empty())
            && self
                .client_id
                .as_deref()
                .is_some_and(|value| !value.is_empty())
    }

    fn refresh_access_token(&self) -> Result<(), CloudSyncError> {
        let refresh_token = self
            .refresh_token
            .as_deref()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                CloudSyncError::Remote("OneDrive refresh token is required".to_string())
            })?;
        let client_id = self
            .client_id
            .as_deref()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| CloudSyncError::Remote("OneDrive client ID is required".to_string()))?;
        let mut fields = vec![
            ("client_id", client_id),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ];
        if let Some(client_secret) = self
            .client_secret
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            fields.push(("client_secret", client_secret));
        }
        let response = self
            .client
            .post(MICROSOFT_OAUTH_TOKEN_URL)
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(form_urlencoded(&fields))
            .send()
            .map_err(map_onedrive_http_error)?;
        let status = response.status();
        let body = response.text().map_err(map_onedrive_http_error)?;
        if !status.is_success() {
            return Err(CloudSyncError::Remote(format!(
                "OneDrive token refresh failed ({status}): {}",
                body.trim()
            )));
        }
        let value: serde_json::Value = serde_json::from_str(&body)?;
        let access_token = value
            .get("access_token")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                CloudSyncError::Remote(
                    "OneDrive token refresh response did not include access_token".to_string(),
                )
            })?;
        *self
            .access_token
            .lock()
            .expect("onedrive access token lock") = access_token.to_string();
        Ok(())
    }

    fn send_authorized(
        &self,
        build: impl Fn(&zed_reqwest::blocking::Client, &str) -> RequestBuilder,
    ) -> Result<zed_reqwest::blocking::Response, CloudSyncError> {
        if self.bearer_token().is_empty() && self.can_refresh_access_token() {
            self.refresh_access_token()?;
        }
        let response = build(&self.client, &self.bearer_token())
            .send()
            .map_err(map_onedrive_http_error)?;
        if response.status() != StatusCode::UNAUTHORIZED || !self.can_refresh_access_token() {
            return Ok(response);
        }
        self.refresh_access_token()?;
        build(&self.client, &self.bearer_token())
            .send()
            .map_err(map_onedrive_http_error)
    }

    fn metadata_url(&self, path: &str) -> String {
        let path = onedrive_item_path(&self.root, path);
        if path.is_empty() {
            format!("{MICROSOFT_GRAPH_BASE_URL}/me/drive/root")
        } else {
            format!(
                "{MICROSOFT_GRAPH_BASE_URL}/me/drive/root:/{}",
                percent_encode_path(&path)
            )
        }
    }

    fn children_url(&self, parent_path: &str) -> String {
        let parent_path = trim_remote_path(parent_path);
        if parent_path.is_empty() {
            format!("{MICROSOFT_GRAPH_BASE_URL}/me/drive/root/children")
        } else {
            format!(
                "{MICROSOFT_GRAPH_BASE_URL}/me/drive/root:/{}:/children",
                percent_encode_path(&parent_path)
            )
        }
    }

    fn content_url(&self, path: &str) -> Result<String, CloudSyncError> {
        let path = onedrive_item_path(&self.root, path);
        if path.is_empty() {
            return Err(CloudSyncError::InvalidRemotePath {
                path: path.to_string(),
            });
        }
        Ok(format!(
            "{MICROSOFT_GRAPH_BASE_URL}/me/drive/root:/{}:/content",
            percent_encode_path(&path)
        ))
    }

    fn folder_exists(&self, path: &str) -> Result<bool, CloudSyncError> {
        let url = self.metadata_url(path);
        let response = self.send_authorized(|client, token| {
            client
                .get(&url)
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .query(&[("select", "id,name,folder,file")])
        })?;
        let status = response.status();
        if status == StatusCode::NOT_FOUND {
            return Ok(false);
        }
        let body = response.text().map_err(map_onedrive_http_error)?;
        if !status.is_success() {
            return Err(CloudSyncError::Remote(format!(
                "OneDrive item lookup failed ({status}): {}",
                body.trim()
            )));
        }
        let value: serde_json::Value = serde_json::from_str(&body)?;
        if value.get("folder").is_some() {
            return Ok(true);
        }
        Err(CloudSyncError::Remote(format!(
            "OneDrive path '{}' exists but is not a folder",
            path
        )))
    }

    fn create_folder(&self, parent_path: &str, name: &str) -> Result<(), CloudSyncError> {
        let url = self.children_url(parent_path);
        let body = json!({
            "name": name,
            "folder": {},
            "@microsoft.graph.conflictBehavior": "fail",
        });
        let response = self.send_authorized(|client, token| {
            client
                .post(&url)
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .json(&body)
        })?;
        let status = response.status();
        if status == StatusCode::CONFLICT {
            return Ok(());
        }
        if status.is_success() {
            return Ok(());
        }
        let body = response.text().unwrap_or_default();
        Err(CloudSyncError::Remote(format!(
            "OneDrive create folder failed ({status}): {}",
            body.trim()
        )))
    }

    fn ensure_folder_segments(&self, child_path: &str) -> Result<(), CloudSyncError> {
        let segments = drive_remote_segments(&self.root, child_path);
        let mut current = String::new();
        for segment in segments {
            if !current.is_empty() {
                current.push('/');
            }
            let parent = current.clone();
            current.push_str(&segment);
            if self.folder_exists(&current)? {
                continue;
            }
            self.create_folder(&parent, &segment)?;
        }
        Ok(())
    }
}

impl CloudSyncRemote for NativeOneDriveRemote {
    fn provider(&self) -> &'static str {
        "onedrive"
    }

    fn create_dir(&self, path: &str) -> Result<(), CloudSyncError> {
        self.ensure_folder_segments(path)
    }

    fn read(&self, path: &str) -> Result<Vec<u8>, CloudSyncError> {
        self.read_if_exists(path)?
            .ok_or_else(|| CloudSyncError::Remote(format!("OneDrive file '{path}' not found")))
    }

    fn read_if_exists(&self, path: &str) -> Result<Option<Vec<u8>>, CloudSyncError> {
        let url = self.content_url(path)?;
        let response = self.send_authorized(|client, token| {
            client
                .get(&url)
                .header(AUTHORIZATION, format!("Bearer {token}"))
        })?;
        let status = response.status();
        if status == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if status.is_success() {
            return response
                .bytes()
                .map(|bytes| Some(bytes.to_vec()))
                .map_err(map_onedrive_http_error);
        }
        let body = response.text().unwrap_or_default();
        Err(CloudSyncError::Remote(format!(
            "OneDrive media download failed ({status}): {}",
            body.trim()
        )))
    }

    fn write(&self, path: &str, bytes: &[u8]) -> Result<(), CloudSyncError> {
        if let Some(parent) = trim_remote_path(path)
            .rsplit_once('/')
            .map(|(parent, _)| parent)
            .filter(|parent| !parent.is_empty())
        {
            self.ensure_folder_segments(parent)?;
        } else if !self.root.is_empty() {
            self.ensure_folder_segments("")?;
        }
        let url = self.content_url(path)?;
        let response = self.send_authorized(|client, token| {
            client
                .put(&url)
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .header(CONTENT_TYPE, "application/octet-stream")
                .body(bytes.to_vec())
        })?;
        let status = response.status();
        if status.is_success() {
            return Ok(());
        }
        let body = response.text().unwrap_or_default();
        Err(CloudSyncError::Remote(format!(
            "OneDrive media upload failed ({status}): {}",
            body.trim()
        )))
    }
}

pub struct NativeAliyunDriveRemote {
    client: zed_reqwest::blocking::Client,
    root: String,
    drive_type: AliyunDriveType,
    access_token: Mutex<String>,
    refresh_token: Mutex<String>,
    client_id: Option<String>,
    client_secret: Option<String>,
    drive_id: Mutex<Option<String>>,
}

impl NativeAliyunDriveRemote {
    pub fn new(settings: &AliyunDriveSyncSettings) -> Result<Self, CloudSyncError> {
        let client = zed_reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("NyaTerm")
            .build()
            .map_err(map_aliyun_drive_http_error)?;
        let remote = Self {
            client,
            root: trim_remote_path(&settings.root),
            drive_type: AliyunDriveType::parse(&settings.drive_type)?,
            access_token: Mutex::new(trim_optional_secret(settings.access_token.as_deref())),
            refresh_token: Mutex::new(trim_optional_secret(settings.refresh_token.as_deref())),
            client_id: trim_optional(settings.client_id.as_deref()),
            client_secret: trim_optional(settings.client_secret.as_deref()),
            drive_id: Mutex::new(None),
        };

        if remote.bearer_token().is_empty() {
            if remote.can_refresh_access_token() {
                remote.refresh_access_token()?;
            } else {
                return Err(CloudSyncError::Remote(
                    "Aliyun Drive access token is required".to_string(),
                ));
            }
        }

        Ok(remote)
    }

    fn bearer_token(&self) -> String {
        self.access_token
            .lock()
            .expect("aliyun drive access token lock")
            .clone()
    }

    fn can_refresh_access_token(&self) -> bool {
        !self
            .refresh_token
            .lock()
            .expect("aliyun drive refresh token lock")
            .is_empty()
            && self
                .client_id
                .as_deref()
                .is_some_and(|value| !value.is_empty())
            && self
                .client_secret
                .as_deref()
                .is_some_and(|value| !value.is_empty())
    }

    fn refresh_access_token(&self) -> Result<(), CloudSyncError> {
        let refresh_token = self
            .refresh_token
            .lock()
            .expect("aliyun drive refresh token lock")
            .clone();
        if refresh_token.is_empty() {
            return Err(CloudSyncError::Remote(
                "Aliyun Drive refresh token is required".to_string(),
            ));
        }
        let client_id = self
            .client_id
            .as_deref()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                CloudSyncError::Remote("Aliyun Drive client ID is required".to_string())
            })?;
        let client_secret = self
            .client_secret
            .as_deref()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                CloudSyncError::Remote("Aliyun Drive client secret is required".to_string())
            })?;
        let body = json!({
            "refresh_token": refresh_token,
            "grant_type": "refresh_token",
            "client_id": client_id,
            "client_secret": client_secret,
        });
        let response = self
            .client
            .post(format!("{ALIYUN_DRIVE_BASE_URL}/oauth/access_token"))
            .header(CONTENT_TYPE, "application/json;charset=UTF-8")
            .json(&body)
            .send()
            .map_err(map_aliyun_drive_http_error)?;
        let status = response.status();
        let body = response.text().map_err(map_aliyun_drive_http_error)?;
        if !status.is_success() {
            return Err(aliyun_drive_remote_error(
                status,
                &body,
                "Aliyun Drive token refresh",
            ));
        }
        let value: serde_json::Value = serde_json::from_str(&body)?;
        let access_token = json_string_field(&value, "access_token", "Aliyun Drive token refresh")?;
        *self
            .access_token
            .lock()
            .expect("aliyun drive access token lock") = access_token;
        if let Some(refresh_token) = value
            .get("refresh_token")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            *self
                .refresh_token
                .lock()
                .expect("aliyun drive refresh token lock") = refresh_token.to_string();
        }
        *self.drive_id.lock().expect("aliyun drive id lock") = None;
        Ok(())
    }

    fn send_json(
        &self,
        endpoint: &str,
        body: &serde_json::Value,
        auth: bool,
    ) -> Result<zed_reqwest::blocking::Response, CloudSyncError> {
        let url = format!("{ALIYUN_DRIVE_BASE_URL}{endpoint}");
        let mut request = self
            .client
            .post(&url)
            .header(CONTENT_TYPE, "application/json;charset=UTF-8")
            .json(body);
        if auth {
            request = request.header(AUTHORIZATION, format!("Bearer {}", self.bearer_token()));
        }
        let response = request.send().map_err(map_aliyun_drive_http_error)?;
        if response.status() != StatusCode::UNAUTHORIZED
            || !auth
            || !self.can_refresh_access_token()
        {
            return Ok(response);
        }
        self.refresh_access_token()?;
        self.client
            .post(&url)
            .header(CONTENT_TYPE, "application/json;charset=UTF-8")
            .header(AUTHORIZATION, format!("Bearer {}", self.bearer_token()))
            .json(body)
            .send()
            .map_err(map_aliyun_drive_http_error)
    }

    fn send_json_expect_success(
        &self,
        endpoint: &str,
        body: &serde_json::Value,
        operation: &str,
    ) -> Result<serde_json::Value, CloudSyncError> {
        let response = self.send_json(endpoint, body, true)?;
        let status = response.status();
        let body = response.text().map_err(map_aliyun_drive_http_error)?;
        if !status.is_success() {
            return Err(aliyun_drive_remote_error(status, &body, operation));
        }
        Ok(serde_json::from_str(&body)?)
    }

    fn drive_id(&self) -> Result<String, CloudSyncError> {
        if let Some(drive_id) = self.drive_id.lock().expect("aliyun drive id lock").clone() {
            return Ok(drive_id);
        }
        let value = self.send_json_expect_success(
            "/adrive/v1.0/user/getDriveInfo",
            &json!({}),
            "Aliyun Drive drive info",
        )?;
        let default_drive_id =
            json_string_field(&value, "default_drive_id", "Aliyun Drive drive info")?;
        let drive_id = match self.drive_type {
            AliyunDriveType::Default => default_drive_id,
            AliyunDriveType::Resource => value
                .get("resource_drive_id")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .unwrap_or(default_drive_id),
            AliyunDriveType::Backup => value
                .get("backup_drive_id")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .unwrap_or(default_drive_id),
        };
        *self.drive_id.lock().expect("aliyun drive id lock") = Some(drive_id.clone());
        Ok(drive_id)
    }

    fn item_path(&self, path: &str) -> String {
        let path = drive_remote_segments(&self.root, path).join("/");
        if path.is_empty() {
            "/".to_string()
        } else {
            format!("/{path}")
        }
    }

    fn get_by_path(&self, path: &str) -> Result<Option<AliyunDriveItem>, CloudSyncError> {
        let drive_id = self.drive_id()?;
        let response = self.send_json(
            "/adrive/v1.0/openFile/get_by_path",
            &json!({
                "drive_id": drive_id,
                "file_path": self.item_path(path),
            }),
            true,
        )?;
        let status = response.status();
        let body = response.text().map_err(map_aliyun_drive_http_error)?;
        if status.is_success() {
            return aliyun_drive_item_from_body(&body).map(Some);
        }
        if status == StatusCode::BAD_REQUEST
            && aliyun_drive_error_code(&body).as_deref() == Some("NotFound.File")
        {
            return Ok(None);
        }
        Err(aliyun_drive_remote_error(
            status,
            &body,
            "Aliyun Drive path lookup",
        ))
    }

    fn create_item(
        &self,
        parent_file_id: &str,
        name: &str,
        item_type: &str,
        size: Option<u64>,
    ) -> Result<serde_json::Value, CloudSyncError> {
        let drive_id = self.drive_id()?;
        self.send_json_expect_success(
            "/adrive/v1.0/openFile/create",
            &json!({
                "drive_id": drive_id,
                "parent_file_id": parent_file_id,
                "name": name,
                "type": item_type,
                "check_name_mode": "refuse",
                "size": size,
            }),
            "Aliyun Drive create item",
        )
    }

    fn create_folder(&self, parent_file_id: &str, name: &str) -> Result<String, CloudSyncError> {
        let value = self.create_item(parent_file_id, name, "folder", None)?;
        json_string_field(&value, "file_id", "Aliyun Drive create folder")
    }

    fn ensure_folder_segments(&self, child_path: &str) -> Result<String, CloudSyncError> {
        let segments = drive_remote_segments(&self.root, child_path);
        if segments.is_empty() {
            return Ok("root".to_string());
        }
        let mut parent_file_id = "root".to_string();
        let mut current_path = String::new();
        for segment in segments {
            if !current_path.is_empty() {
                current_path.push('/');
            }
            current_path.push_str(&segment);
            if let Some(item) = self.get_by_path(&current_path)? {
                if item.is_folder() {
                    parent_file_id = item.file_id;
                    continue;
                }
                return Err(CloudSyncError::Remote(format!(
                    "Aliyun Drive path '{current_path}' exists but is not a folder"
                )));
            }
            parent_file_id = self.create_folder(&parent_file_id, &segment)?;
        }
        Ok(parent_file_id)
    }

    fn delete_file(&self, file_id: &str) -> Result<(), CloudSyncError> {
        let drive_id = self.drive_id()?;
        self.send_json_expect_success(
            "/adrive/v1.0/openFile/delete",
            &json!({
                "drive_id": drive_id,
                "file_id": file_id,
            }),
            "Aliyun Drive delete file",
        )?;
        Ok(())
    }

    fn get_upload_url(&self, file_id: &str, upload_id: &str) -> Result<String, CloudSyncError> {
        let drive_id = self.drive_id()?;
        let value = self.send_json_expect_success(
            "/adrive/v1.0/openFile/getUploadUrl",
            &json!({
                "drive_id": drive_id,
                "file_id": file_id,
                "upload_id": upload_id,
                "part_info_list": [{"part_number": 1}],
            }),
            "Aliyun Drive upload URL",
        )?;
        value
            .get("part_info_list")
            .and_then(serde_json::Value::as_array)
            .and_then(|items| items.first())
            .and_then(|item| item.get("upload_url"))
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .ok_or_else(|| {
                CloudSyncError::Remote(
                    "Aliyun Drive upload URL response is missing upload_url".to_string(),
                )
            })
    }

    fn upload_part(&self, upload_url: &str, bytes: &[u8]) -> Result<(), CloudSyncError> {
        let response = self
            .client
            .put(upload_url)
            .body(bytes.to_vec())
            .send()
            .map_err(map_aliyun_drive_http_error)?;
        let status = response.status();
        if status.is_success() {
            return Ok(());
        }
        let body = response.text().unwrap_or_default();
        Err(CloudSyncError::Remote(format!(
            "Aliyun Drive upload failed ({status}): {}",
            body.trim()
        )))
    }

    fn complete_upload(&self, file_id: &str, upload_id: &str) -> Result<(), CloudSyncError> {
        let drive_id = self.drive_id()?;
        self.send_json_expect_success(
            "/adrive/v1.0/openFile/complete",
            &json!({
                "drive_id": drive_id,
                "file_id": file_id,
                "upload_id": upload_id,
            }),
            "Aliyun Drive complete upload",
        )?;
        Ok(())
    }

    fn download_url(&self, file_id: &str) -> Result<String, CloudSyncError> {
        let drive_id = self.drive_id()?;
        let value = self.send_json_expect_success(
            "/adrive/v1.0/openFile/getDownloadUrl",
            &json!({
                "drive_id": drive_id,
                "file_id": file_id,
            }),
            "Aliyun Drive download URL",
        )?;
        json_string_field(&value, "url", "Aliyun Drive download URL")
    }
}

impl CloudSyncRemote for NativeAliyunDriveRemote {
    fn provider(&self) -> &'static str {
        "aliyun_drive"
    }

    fn create_dir(&self, path: &str) -> Result<(), CloudSyncError> {
        self.ensure_folder_segments(path)?;
        Ok(())
    }

    fn read(&self, path: &str) -> Result<Vec<u8>, CloudSyncError> {
        self.read_if_exists(path)?
            .ok_or_else(|| CloudSyncError::Remote(format!("Aliyun Drive file '{path}' not found")))
    }

    fn read_if_exists(&self, path: &str) -> Result<Option<Vec<u8>>, CloudSyncError> {
        let Some(item) = self.get_by_path(path)? else {
            return Ok(None);
        };
        if item.is_folder() {
            return Err(CloudSyncError::Remote(format!(
                "Aliyun Drive path '{path}' is a folder"
            )));
        }
        let download_url = self.download_url(&item.file_id)?;
        let response = self
            .client
            .get(download_url)
            .send()
            .map_err(map_aliyun_drive_http_error)?;
        let status = response.status();
        if status.is_success() {
            return response
                .bytes()
                .map(|bytes| Some(bytes.to_vec()))
                .map_err(map_aliyun_drive_http_error);
        }
        let body = response.text().unwrap_or_default();
        Err(CloudSyncError::Remote(format!(
            "Aliyun Drive download failed ({status}): {}",
            body.trim()
        )))
    }

    fn write(&self, path: &str, bytes: &[u8]) -> Result<(), CloudSyncError> {
        let segments = drive_remote_segments("", path);
        let Some((file_name, parent_segments)) = segments.split_last() else {
            return Err(CloudSyncError::InvalidRemotePath {
                path: path.to_string(),
            });
        };
        let parent_path = parent_segments.join("/");
        let parent_file_id = self.ensure_folder_segments(&parent_path)?;
        if let Some(existing) = self.get_by_path(path)? {
            if existing.is_folder() {
                return Err(CloudSyncError::Remote(format!(
                    "Aliyun Drive path '{path}' is a folder"
                )));
            }
            self.delete_file(&existing.file_id)?;
        }
        let created =
            self.create_item(&parent_file_id, file_name, "file", Some(bytes.len() as u64))?;
        let file_id = json_string_field(&created, "file_id", "Aliyun Drive create file")?;
        let upload_id = json_string_field(&created, "upload_id", "Aliyun Drive create file")?;
        let upload_url = self.get_upload_url(&file_id, &upload_id)?;
        self.upload_part(&upload_url, bytes)?;
        self.complete_upload(&file_id, &upload_id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AliyunDriveType {
    Default,
    Resource,
    Backup,
}

impl AliyunDriveType {
    fn parse(value: &str) -> Result<Self, CloudSyncError> {
        match value.trim() {
            "" | "default" => Ok(Self::Default),
            "resource" => Ok(Self::Resource),
            "backup" => Ok(Self::Backup),
            other => Err(CloudSyncError::Remote(format!(
                "Aliyun Drive type '{other}' is not supported"
            ))),
        }
    }
}

#[derive(Debug, Clone)]
struct AliyunDriveItem {
    file_id: String,
    item_type: String,
}

impl AliyunDriveItem {
    fn is_folder(&self) -> bool {
        self.item_type == "folder"
    }
}

#[derive(Debug, Clone)]
struct GoogleDriveFile {
    id: String,
}

fn google_drive_json_id(body: &str, label: &str) -> Result<String, CloudSyncError> {
    let value: serde_json::Value = serde_json::from_str(body)?;
    value
        .get("id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            CloudSyncError::Remote(format!("Google Drive {label} response is missing id"))
        })
}

fn json_string_field(
    value: &serde_json::Value,
    field: &str,
    operation: &str,
) -> Result<String, CloudSyncError> {
    value
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| CloudSyncError::Remote(format!("{operation} response is missing {field}")))
}

fn aliyun_drive_item_from_body(body: &str) -> Result<AliyunDriveItem, CloudSyncError> {
    let value: serde_json::Value = serde_json::from_str(body)?;
    Ok(AliyunDriveItem {
        file_id: json_string_field(&value, "file_id", "Aliyun Drive item")?,
        item_type: json_string_field(&value, "type", "Aliyun Drive item")?,
    })
}

fn aliyun_drive_error_code(body: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| {
            value
                .get("code")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        })
}

fn aliyun_drive_remote_error(status: StatusCode, body: &str, operation: &str) -> CloudSyncError {
    let message = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| {
            let code = value.get("code").and_then(serde_json::Value::as_str);
            let message = value.get("message").and_then(serde_json::Value::as_str);
            match (code, message) {
                (Some(code), Some(message)) => Some(format!("{code}: {message}")),
                (Some(code), None) => Some(code.to_string()),
                (None, Some(message)) => Some(message.to_string()),
                (None, None) => None,
            }
        })
        .unwrap_or_else(|| body.trim().to_string());
    CloudSyncError::Remote(format!("{operation} failed ({status}): {message}"))
}

fn google_drive_expect_success(
    response: zed_reqwest::blocking::Response,
    operation: &str,
) -> Result<(), CloudSyncError> {
    let status = response.status();
    if status.is_success() {
        return Ok(());
    }
    let body = response.text().unwrap_or_default();
    Err(CloudSyncError::Remote(format!(
        "{operation} failed ({status}): {}",
        body.trim()
    )))
}

fn google_drive_multipart_body(
    parent_id: &str,
    name: &str,
    bytes: &[u8],
) -> Result<(String, Vec<u8>), CloudSyncError> {
    let boundary = format!("nyaterm-{}", request_nonce());
    let metadata = serde_json::to_vec(&json!({
        "name": name,
        "parents": [parent_id],
    }))?;
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(b"Content-Type: application/json; charset=UTF-8\r\n\r\n");
    body.extend_from_slice(&metadata);
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
    body.extend_from_slice(bytes);
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    Ok((boundary, body))
}

fn map_s3_http_error(error: zed_reqwest::Error) -> CloudSyncError {
    if error.is_timeout() {
        CloudSyncError::Io(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            format!("S3 operation timed out: {error}"),
        ))
    } else {
        CloudSyncError::Remote(format!("S3 request failed: {error}"))
    }
}

fn map_google_drive_http_error(error: zed_reqwest::Error) -> CloudSyncError {
    if error.is_timeout() {
        CloudSyncError::Io(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            format!("Google Drive operation timed out: {error}"),
        ))
    } else {
        CloudSyncError::Remote(format!("Google Drive request failed: {error}"))
    }
}

fn map_onedrive_http_error(error: zed_reqwest::Error) -> CloudSyncError {
    if error.is_timeout() {
        CloudSyncError::Io(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            format!("OneDrive operation timed out: {error}"),
        ))
    } else {
        CloudSyncError::Remote(format!("OneDrive request failed: {error}"))
    }
}

fn map_aliyun_drive_http_error(error: zed_reqwest::Error) -> CloudSyncError {
    if error.is_timeout() {
        CloudSyncError::Io(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            format!("Aliyun Drive operation timed out: {error}"),
        ))
    } else {
        CloudSyncError::Remote(format!("Aliyun Drive request failed: {error}"))
    }
}

fn map_webdav_http_error(error: zed_reqwest::Error) -> CloudSyncError {
    if error.is_timeout() {
        CloudSyncError::Io(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            format!("WebDAV operation timed out: {error}"),
        ))
    } else {
        CloudSyncError::Remote(format!("WebDAV request failed: {error}"))
    }
}

fn normalize_endpoint(endpoint: &str) -> String {
    endpoint.trim().trim_end_matches('/').to_string()
}

fn trim_remote_path(path: &str) -> String {
    path.trim().trim_matches('/').to_string()
}

fn trim_optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn trim_optional_secret(value: Option<&str>) -> String {
    value.map(str::trim).unwrap_or_default().to_string()
}

fn onedrive_item_path(base_root: &str, child: &str) -> String {
    drive_remote_segments(base_root, child).join("/")
}

fn percent_encode_path(path: &str) -> String {
    path.split('/')
        .map(percent_encode_uri_component)
        .collect::<Vec<_>>()
        .join("/")
}

fn percent_encode_uri_component(value: &str) -> String {
    let mut output = String::new();
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                output.push(*byte as char);
            }
            other => output.push_str(&format!("%{other:02X}")),
        }
    }
    output
}

fn form_urlencoded(fields: &[(&str, &str)]) -> String {
    fields
        .iter()
        .map(|(name, value)| {
            format!(
                "{}={}",
                percent_encode_form(name),
                percent_encode_form(value)
            )
        })
        .collect::<Vec<_>>()
        .join("&")
}

fn percent_encode_form(value: &str) -> String {
    let mut output = String::new();
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                output.push(*byte as char);
            }
            b' ' => output.push('+'),
            other => output.push_str(&format!("%{other:02X}")),
        }
    }
    output
}

fn request_nonce() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:x}-{nanos:x}", std::process::id())
}

fn digest_challenge(response: &zed_reqwest::blocking::Response) -> Option<String> {
    response
        .headers()
        .get_all(WWW_AUTHENTICATE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find_map(|value| {
            value
                .split_once("Digest")
                .map(|(_, challenge)| challenge.trim().to_string())
        })
        .filter(|value| !value.is_empty())
}

fn build_digest_authorization(
    challenge: &str,
    username: &str,
    password: &str,
    method: &str,
    uri: &str,
    cnonce: &str,
    nc: &str,
) -> Result<String, CloudSyncError> {
    let params = parse_digest_challenge(challenge);
    let realm = required_digest_param(&params, "realm")?;
    let nonce = required_digest_param(&params, "nonce")?;
    let qop = choose_digest_qop(params.get("qop").map(String::as_str))?;
    let algorithm = params
        .get("algorithm")
        .map_or("MD5", String::as_str)
        .trim()
        .to_ascii_uppercase();

    let ha1 = digest_hash(&algorithm, &format!("{username}:{realm}:{password}"))?;
    let ha2 = digest_hash(&algorithm, &format!("{method}:{uri}"))?;
    let response = digest_hash(
        &algorithm,
        &format!("{ha1}:{nonce}:{nc}:{cnonce}:{qop}:{ha2}"),
    )?;
    let opaque = params
        .get("opaque")
        .map(|value| format!(", opaque=\"{}\"", escape_digest_value(value)))
        .unwrap_or_default();

    Ok(format!(
        "Digest username=\"{}\", realm=\"{}\", nonce=\"{}\", uri=\"{}\", algorithm={}, response=\"{}\", qop={}, nc={}, cnonce=\"{}\"{}",
        escape_digest_value(username),
        escape_digest_value(realm),
        escape_digest_value(nonce),
        escape_digest_value(uri),
        algorithm,
        response,
        qop,
        nc,
        escape_digest_value(cnonce),
        opaque
    ))
}

fn parse_digest_challenge(challenge: &str) -> std::collections::BTreeMap<String, String> {
    let mut values = std::collections::BTreeMap::new();
    let mut rest = challenge.trim();
    while !rest.is_empty() {
        rest = rest.trim_start_matches(|ch: char| ch == ',' || ch.is_whitespace());
        let Some((key, after_key)) = rest.split_once('=') else {
            break;
        };
        let key = key.trim().to_ascii_lowercase();
        let after_key = after_key.trim_start();
        let (value, next) = if let Some(quoted) = after_key.strip_prefix('"') {
            parse_quoted_digest_value(quoted)
        } else {
            let split_at = after_key.find(',').unwrap_or(after_key.len());
            (
                after_key[..split_at].trim().to_string(),
                after_key[split_at..].trim_start_matches(','),
            )
        };
        if !key.is_empty() {
            values.insert(key, value);
        }
        rest = next;
    }
    values
}

fn parse_quoted_digest_value(input: &str) -> (String, &str) {
    let mut value = String::new();
    let mut escaped = false;
    for (index, ch) in input.char_indices() {
        if escaped {
            value.push(ch);
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '"' => return (value, &input[index + ch.len_utf8()..]),
            _ => value.push(ch),
        }
    }
    (value, "")
}

fn required_digest_param<'a>(
    params: &'a std::collections::BTreeMap<String, String>,
    key: &str,
) -> Result<&'a str, CloudSyncError> {
    params
        .get(key)
        .map(String::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            CloudSyncError::Remote(format!(
                "WebDAV Digest authentication challenge is missing {key}"
            ))
        })
}

fn choose_digest_qop(qop: Option<&str>) -> Result<&'static str, CloudSyncError> {
    let Some(qop) = qop else {
        return Err(CloudSyncError::Remote(
            "WebDAV Digest authentication without qop=auth is not supported".to_string(),
        ));
    };
    if qop
        .split(',')
        .map(|value| value.trim().trim_matches('"').to_ascii_lowercase())
        .any(|value| value == "auth")
    {
        Ok("auth")
    } else {
        Err(CloudSyncError::Remote(
            "WebDAV Digest authentication requires qop=auth".to_string(),
        ))
    }
}

fn digest_hash(algorithm: &str, value: &str) -> Result<String, CloudSyncError> {
    match algorithm {
        "MD5" => Ok(format!("{:x}", md5::compute(value.as_bytes()))),
        "SHA-256" | "SHA256" => {
            let mut hasher = Sha256::new();
            hasher.update(value.as_bytes());
            Ok(hex::encode(hasher.finalize()))
        }
        other => Err(CloudSyncError::Remote(format!(
            "WebDAV Digest algorithm {other} is not supported"
        ))),
    }
}

fn escape_digest_value(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn webdav_cnonce() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:x}{:x}", std::process::id(), now)
}

fn path_and_query(url: &str) -> &str {
    let Some(after_scheme) = url.split_once("://").map(|(_, rest)| rest) else {
        return "/";
    };
    after_scheme
        .find('/')
        .map(|index| &after_scheme[index..])
        .unwrap_or("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn webdav_url_joins_endpoint_root_and_sync_path() {
        let remote = NativeWebdavRemote::new(&WebdavSyncSettings {
            endpoint: "https://dav.example.com/remote.php/webdav/".to_string(),
            root: "/apps/nyaterm/".to_string(),
            username: String::new(),
            password: None,
        })
        .expect("remote");

        assert_eq!(
            remote.url_for("/nyaterm/sync/latest.redb"),
            "https://dav.example.com/remote.php/webdav/apps/nyaterm/nyaterm/sync/latest.redb"
        );
    }

    #[test]
    fn webdav_digest_authorization_matches_rfc_example() {
        let header = build_digest_authorization(
            r#"realm="testrealm@host.com", qop="auth", nonce="dcd98b7102dd2f0e8b11d0f600bfb0c093", opaque="5ccc069c403ebaf9f0171e9517f40e41""#,
            "Mufasa",
            "Circle Of Life",
            "GET",
            "/dir/index.html",
            "0a4f113b",
            "00000001",
        )
        .expect("digest header");

        assert!(header.contains("Digest username=\"Mufasa\""));
        assert!(header.contains("qop=auth"));
        assert!(header.contains("response=\"6629fae49393a05397450978507c4ef1\""));
    }

    #[test]
    fn webdav_digest_parser_handles_quoted_commas() {
        let parsed = parse_digest_challenge(
            r#"realm="Nya,Term", nonce="abc", algorithm=MD5, qop="auth,auth-int""#,
        );

        assert_eq!(parsed.get("realm").map(String::as_str), Some("Nya,Term"));
        assert_eq!(parsed.get("nonce").map(String::as_str), Some("abc"));
        assert_eq!(parsed.get("qop").map(String::as_str), Some("auth,auth-int"));
    }

    #[test]
    fn google_drive_multipart_body_contains_metadata_and_media() {
        let (boundary, body) =
            google_drive_multipart_body("root", "latest.redb", b"payload").expect("multipart");
        let text = String::from_utf8(body).expect("utf8 multipart");

        assert!(text.contains(&format!("--{boundary}\r\n")));
        assert!(text.contains(r#""name":"latest.redb""#));
        assert!(text.contains(r#""parents":["root"]"#));
        assert!(text.contains("Content-Type: application/octet-stream\r\n\r\npayload"));
        assert!(text.ends_with(&format!("--{boundary}--\r\n")));
    }

    #[test]
    fn form_urlencoded_uses_oauth_form_rules() {
        assert_eq!(
            form_urlencoded(&[("client id", "abc+123"), ("secret", "a/b?c")]),
            "client+id=abc%2B123&secret=a%2Fb%3Fc"
        );
    }

    #[test]
    fn onedrive_item_path_joins_root_and_child_segments() {
        assert_eq!(
            onedrive_item_path("/Nya Term/", "/sync/latest.redb"),
            "Nya Term/sync/latest.redb"
        );
        assert_eq!(
            onedrive_item_path("", "sync/latest.redb"),
            "sync/latest.redb"
        );
    }

    #[test]
    fn percent_encode_path_preserves_separators_and_encodes_segments() {
        assert_eq!(
            percent_encode_path("Nya Term/sync/latest redb/猫"),
            "Nya%20Term/sync/latest%20redb/%E7%8C%AB"
        );
    }

    #[test]
    fn onedrive_urls_use_graph_path_addressing_templates() {
        let remote = NativeOneDriveRemote {
            client: zed_reqwest::blocking::Client::builder()
                .build()
                .expect("client"),
            root: "Nya Term".to_string(),
            access_token: Mutex::new("token".to_string()),
            refresh_token: None,
            client_id: None,
            client_secret: None,
        };

        assert_eq!(
            remote.children_url("Nya Term/sync"),
            "https://graph.microsoft.com/v1.0/me/drive/root:/Nya%20Term/sync:/children"
        );
        assert_eq!(
            remote.content_url("sync/latest redb").expect("content url"),
            "https://graph.microsoft.com/v1.0/me/drive/root:/Nya%20Term/sync/latest%20redb:/content"
        );
    }

    #[test]
    fn aliyun_drive_type_matches_legacy_values() {
        assert_eq!(
            AliyunDriveType::parse("").expect("default"),
            AliyunDriveType::Default
        );
        assert_eq!(
            AliyunDriveType::parse("resource").expect("resource"),
            AliyunDriveType::Resource
        );
        assert_eq!(
            AliyunDriveType::parse("backup").expect("backup"),
            AliyunDriveType::Backup
        );
        assert!(AliyunDriveType::parse("archive").is_err());
    }

    #[test]
    fn aliyun_drive_item_path_uses_rooted_absolute_path() {
        let remote = NativeAliyunDriveRemote {
            client: zed_reqwest::blocking::Client::builder()
                .build()
                .expect("client"),
            root: "Nya Term".to_string(),
            drive_type: AliyunDriveType::Resource,
            access_token: Mutex::new("token".to_string()),
            refresh_token: Mutex::new(String::new()),
            client_id: None,
            client_secret: None,
            drive_id: Mutex::new(None),
        };

        assert_eq!(
            remote.item_path("sync/latest redb"),
            "/Nya Term/sync/latest redb"
        );
        assert_eq!(remote.item_path(""), "/Nya Term");
    }

    #[test]
    fn aliyun_drive_error_helpers_preserve_code_and_message() {
        let body = r#"{"code":"NotFound.File","message":"file missing"}"#;

        assert_eq!(
            aliyun_drive_error_code(body).as_deref(),
            Some("NotFound.File")
        );
        assert!(
            aliyun_drive_remote_error(StatusCode::BAD_REQUEST, body, "lookup")
                .to_string()
                .contains("NotFound.File: file missing")
        );
    }
}
