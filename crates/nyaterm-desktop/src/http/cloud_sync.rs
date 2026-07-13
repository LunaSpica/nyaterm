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

#[path = "cloud_sync/helpers.rs"]
mod helpers;
use helpers::*;

#[path = "cloud_sync/snippet.rs"]
mod snippet;
pub use snippet::*;

#[path = "cloud_sync/webdav.rs"]
mod webdav;
pub use webdav::*;

#[path = "cloud_sync/s3.rs"]
mod s3;
pub use s3::*;

#[path = "cloud_sync/google_drive.rs"]
mod google_drive;
#[cfg(test)]
use google_drive::google_drive_multipart_body;
pub use google_drive::*;

#[path = "cloud_sync/onedrive.rs"]
mod onedrive;
#[cfg(test)]
use onedrive::onedrive_item_path;
pub use onedrive::*;

#[path = "cloud_sync/aliyun.rs"]
mod aliyun;
pub use aliyun::*;
#[cfg(test)]
use aliyun::{AliyunDriveType, aliyun_drive_error_code, aliyun_drive_remote_error};

#[cfg(test)]
#[path = "cloud_sync/tests.rs"]
mod tests;
