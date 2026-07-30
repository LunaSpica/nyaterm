use std::collections::{BTreeMap, HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, UNIX_EPOCH};

use super::{
    CLOUD_SYNC_HISTORY_DOMAIN, CLOUD_SYNC_HISTORY_EVENT, CLOUD_SYNC_HISTORY_LIMIT, CloudSyncError,
    CloudSyncHistoryEntry, CloudSyncRemote, CloudSyncSettings, CloudSyncState,
    GiteeSnippetHttpBackend, GiteeSnippetSyncSettings, GithubGistHttpBackend,
    GithubGistSyncSettings, LocalCloudSyncOptions, MASKED_SECRET_VALUE, S3HttpMethod,
    S3SyncSettings, SnippetBlobBackend, SnippetHttpClient, SnippetHttpMethod, SnippetHttpRequest,
    SnippetHttpResponse, SnippetRemote, append_cloud_sync_history, build_s3_signed_request,
    decode_snippet_blob, drive_remote_segments, encode_snippet_blob, gitee_snippet_patch_body,
    github_gist_patch_body, google_drive_query_literal, merge_masked_cloud_sync_settings,
    pull_local_snapshot, pull_snapshot_with_remote, push_local_snapshot, push_snapshot_with_remote,
    read_cloud_sync_history, remote_path, s3_payload_sha256, snippet_remote_filename,
    snippet_remote_path,
};
use crate::{AiExecutionProfile, ConnectionStore, ConnectionType, SavedConnection, SessionsConfig};

#[derive(Default)]
struct MemoryRemote {
    files: Mutex<HashMap<String, Vec<u8>>>,
}

#[derive(Default)]
struct MemorySnippetBackend {
    blobs: Mutex<std::collections::BTreeMap<String, String>>,
}

#[derive(Clone)]
struct RecordingSnippetHttpClient {
    inner: Arc<RecordingSnippetHttpClientInner>,
}

struct RecordingSnippetHttpClientInner {
    requests: Mutex<Vec<SnippetHttpRequest>>,
    responses: Mutex<VecDeque<Result<SnippetHttpResponse, CloudSyncError>>>,
}

impl RecordingSnippetHttpClient {
    fn new(responses: Vec<SnippetHttpResponse>) -> Self {
        Self {
            inner: Arc::new(RecordingSnippetHttpClientInner {
                requests: Mutex::new(Vec::new()),
                responses: Mutex::new(responses.into_iter().map(Ok).collect()),
            }),
        }
    }

    fn requests(&self) -> Vec<SnippetHttpRequest> {
        self.inner
            .requests
            .lock()
            .expect("http requests lock")
            .clone()
    }
}

impl SnippetHttpClient for RecordingSnippetHttpClient {
    fn send(&self, request: SnippetHttpRequest) -> Result<SnippetHttpResponse, CloudSyncError> {
        self.inner
            .requests
            .lock()
            .expect("http requests lock")
            .push(request);
        self.inner
            .responses
            .lock()
            .expect("http responses lock")
            .pop_front()
            .expect("queued response")
    }
}

impl SnippetBlobBackend for MemorySnippetBackend {
    fn fetch_blob(&self, filename: &str) -> Result<Option<String>, CloudSyncError> {
        Ok(self
            .blobs
            .lock()
            .expect("snippet lock")
            .get(filename)
            .cloned())
    }

    fn patch_blobs(
        &self,
        files: std::collections::BTreeMap<String, Option<String>>,
    ) -> Result<(), CloudSyncError> {
        let mut blobs = self.blobs.lock().expect("snippet lock");
        for (filename, content) in files {
            match content {
                Some(content) => {
                    blobs.insert(filename, content);
                }
                None => {
                    blobs.remove(&filename);
                }
            }
        }
        Ok(())
    }
}

impl CloudSyncRemote for MemoryRemote {
    fn provider(&self) -> &'static str {
        "memory"
    }

    fn create_dir(&self, _path: &str) -> Result<(), CloudSyncError> {
        Ok(())
    }

    fn read(&self, path: &str) -> Result<Vec<u8>, CloudSyncError> {
        self.files
            .lock()
            .expect("memory lock")
            .get(path)
            .cloned()
            .ok_or_else(|| CloudSyncError::ReadFile {
                path: PathBuf::from(path),
                source: std::io::Error::new(std::io::ErrorKind::NotFound, "missing"),
            })
    }

    fn read_if_exists(&self, path: &str) -> Result<Option<Vec<u8>>, CloudSyncError> {
        Ok(self.files.lock().expect("memory lock").get(path).cloned())
    }

    fn write(&self, path: &str, bytes: &[u8]) -> Result<(), CloudSyncError> {
        self.files
            .lock()
            .expect("memory lock")
            .insert(path.to_string(), bytes.to_vec());
        Ok(())
    }
}

#[test]
fn remote_path_joins_without_duplicate_slashes() {
    assert_eq!(
        remote_path("nyaterm", "sync/latest.redb"),
        "nyaterm/sync/latest.redb"
    );
    assert_eq!(
        remote_path("/nyaterm/", "/sync/latest.redb"),
        "nyaterm/sync/latest.redb"
    );
    assert_eq!(remote_path("", "sync/latest.redb"), "sync/latest.redb");
}

#[test]
fn drive_remote_segments_trim_root_and_child_paths() {
    assert_eq!(
        drive_remote_segments("/root/", "/sync/latest.redb"),
        vec!["root", "sync", "latest.redb"]
    );
    assert_eq!(
        drive_remote_segments("", "nyaterm//sync/latest.redb"),
        vec!["nyaterm", "sync", "latest.redb"]
    );
}

#[test]
fn google_drive_query_literal_escapes_quotes_and_backslashes() {
    assert_eq!(google_drive_query_literal("a'b\\c"), "'a\\'b\\\\c'");
}

#[test]
fn s3_signed_request_uses_path_style_url_and_headers() {
    let settings = S3SyncSettings {
        endpoint: "https://s3.example.com/".to_string(),
        bucket: "nyaterm-sync".to_string(),
        region: "ap-east-1".to_string(),
        root: "/profiles/default/".to_string(),
        access_key_id: Some("AKIDEXAMPLE".to_string()),
        secret_access_key: Some("SECRET".to_string()),
        session_token: Some("SESSION".to_string()),
        virtual_host_style: false,
    };
    let request = build_s3_signed_request(
        &settings,
        S3HttpMethod::Put,
        "/nyaterm/sync/latest redb",
        &s3_payload_sha256(b"payload"),
        UNIX_EPOCH + Duration::from_secs(1_704_067_200),
    )
    .expect("signed request");

    assert_eq!(
        request.url,
        "https://s3.example.com/nyaterm-sync/profiles/default/nyaterm/sync/latest%20redb"
    );
    assert_eq!(
        request.headers.get("x-amz-date").map(String::as_str),
        Some("20240101T000000Z")
    );
    assert_eq!(
        request
            .headers
            .get("x-amz-security-token")
            .map(String::as_str),
        Some("SESSION")
    );
    let authorization = request.headers.get("authorization").expect("authorization");
    assert!(authorization.contains("Credential=AKIDEXAMPLE/20240101/ap-east-1/s3/aws4_request"));
    assert!(
        authorization
            .contains("SignedHeaders=host;x-amz-content-sha256;x-amz-date;x-amz-security-token")
    );
}

#[test]
fn s3_signed_request_supports_virtual_host_style() {
    let settings = S3SyncSettings {
        endpoint: "https://objects.example.com/base".to_string(),
        bucket: "nyaterm".to_string(),
        region: String::new(),
        root: String::new(),
        access_key_id: Some("key".to_string()),
        secret_access_key: Some("secret".to_string()),
        session_token: None,
        virtual_host_style: true,
    };
    let request = build_s3_signed_request(
        &settings,
        S3HttpMethod::Get,
        "sync/current.redb.enc",
        &s3_payload_sha256(&[]),
        UNIX_EPOCH,
    )
    .expect("signed request");

    assert_eq!(
        request.url,
        "https://nyaterm.objects.example.com/base/sync/current.redb.enc"
    );
    assert_eq!(
        request.headers.get("host").map(String::as_str),
        Some("nyaterm.objects.example.com")
    );
    assert!(request.headers["authorization"].contains("/19700101/us-east-1/s3/aws4_request"));
}

#[test]
fn s3_signed_request_requires_bucket_and_credentials() {
    let settings = S3SyncSettings {
        endpoint: "https://s3.example.com".to_string(),
        access_key_id: Some("key".to_string()),
        secret_access_key: Some("secret".to_string()),
        ..S3SyncSettings::default()
    };
    let error = build_s3_signed_request(
        &settings,
        S3HttpMethod::Head,
        "sync/latest.redb",
        &s3_payload_sha256(&[]),
        UNIX_EPOCH,
    )
    .expect_err("missing bucket");
    assert!(error.to_string().contains("S3 bucket is required"));

    let settings = S3SyncSettings {
        endpoint: "https://s3.example.com".to_string(),
        bucket: "bucket".to_string(),
        ..S3SyncSettings::default()
    };
    let error = build_s3_signed_request(
        &settings,
        S3HttpMethod::Head,
        "sync/latest.redb",
        &s3_payload_sha256(&[]),
        UNIX_EPOCH,
    )
    .expect_err("missing access key");
    assert!(error.to_string().contains("S3 access key ID is required"));
}

#[test]
fn cloud_sync_history_append_and_read_matches_legacy_log_shape() {
    let dir = unique_temp_dir("cloud-history-append");
    let entry = CloudSyncHistoryEntry {
        id: "history-1".to_string(),
        timestamp_ms: 300,
        kind: "sync".to_string(),
        status: "success".to_string(),
        trigger: "manual_push".to_string(),
        provider: Some("local_directory".to_string()),
        revision: Some("rev-1".to_string()),
        duration_ms: Some(42),
        message: "uploaded".to_string(),
    };

    append_cloud_sync_history(&dir, &entry).expect("append history");
    let entries =
        read_cloud_sync_history(&dir, 7, CLOUD_SYNC_HISTORY_LIMIT).expect("read appended history");

    assert_eq!(entries, vec![entry]);
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn cloud_sync_history_reads_only_recent_cloud_entries_with_limit() {
    let dir = unique_temp_dir("cloud-history-limit");
    let path = dir.join(format!(
        "{}-legacy.{}",
        crate::diagnostics::LOG_FILE_PREFIX,
        crate::diagnostics::LOG_FILE_SUFFIX
    ));
    let lines = [
        serde_json::json!({
            "domain": "session.lifecycle",
            "event": "entry",
            "message": "ignored",
            "data": {
                "id": "ignored",
                "timestamp_ms": 999,
                "kind": "sync",
                "status": "success",
                "trigger": "manual_push"
            }
        })
        .to_string(),
        history_line("old", 100),
        history_line("new", 200),
    ];
    std::fs::write(&path, lines.join("\n")).expect("write legacy history log");

    let entries = read_cloud_sync_history(&dir, 7, 1).expect("read history");

    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>(),
        vec!["new"]
    );
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn local_cloud_sync_push_and_forced_pull_round_trip() {
    let source_dir = unique_temp_dir("cloud-source");
    let target_dir = unique_temp_dir("cloud-target");
    let remote_dir = unique_temp_dir("cloud-remote");
    let source_options = options(&source_dir, &remote_dir, "source-device");
    let target_options = options(&target_dir, &remote_dir, "target-device");
    let source_store = ConnectionStore::open(&source_dir).expect("source store");
    source_store
        .replace_sessions(&SessionsConfig {
            groups: Vec::new(),
            connections: vec![local_connection("conn-1", "Synced Shell", "bash")],
        })
        .expect("seed source");
    drop(source_store);

    let push = push_local_snapshot(&source_options, &CloudSyncState::default(), false)
        .expect("push snapshot");
    assert_eq!(push.status.message, "Cloud sync snapshot uploaded");
    assert!(remote_dir.join("nyaterm/sync/current.redb.enc").exists());
    assert!(remote_dir.join("nyaterm/sync/latest.redb").exists());
    let saved_source_state = ConnectionStore::open(&source_dir)
        .expect("source reopen")
        .load_cloud_sync_state()
        .expect("source cloud state");
    assert_eq!(
        saved_source_state.last_synced_payload_hash,
        push.state.last_synced_payload_hash
    );

    let pull = pull_local_snapshot(&target_options, &CloudSyncState::default(), true)
        .expect("pull snapshot");
    assert_eq!(pull.status.message, "Cloud sync snapshot downloaded");
    assert!(pull.backup.is_some());
    let saved_target_state = ConnectionStore::open(&target_dir)
        .expect("target reopen")
        .load_cloud_sync_state()
        .expect("target cloud state");
    assert_eq!(
        saved_target_state.last_applied_remote_revision,
        pull.state.last_applied_remote_revision
    );

    let loaded = ConnectionStore::open(&target_dir)
        .expect("target store")
        .load_sessions()
        .expect("load target");
    assert_eq!(loaded.connections[0].name, "Synced Shell");
    assert_eq!(
        pull.state.last_synced_payload_hash,
        push.state.last_synced_payload_hash
    );

    std::fs::remove_dir_all(source_dir).ok();
    std::fs::remove_dir_all(target_dir).ok();
    std::fs::remove_dir_all(remote_dir).ok();
}

#[test]
fn cloud_sync_algorithm_uses_remote_backend_abstraction() {
    let source_dir = unique_temp_dir("cloud-remote-source");
    let target_dir = unique_temp_dir("cloud-remote-target");
    let remote_dir = unique_temp_dir("cloud-remote-unused");
    let source_options = options(&source_dir, &remote_dir, "source-device");
    let target_options = options(&target_dir, &remote_dir, "target-device");
    let remote = MemoryRemote::default();

    ConnectionStore::open(&source_dir)
        .expect("source store")
        .replace_sessions(&SessionsConfig {
            groups: Vec::new(),
            connections: vec![local_connection("conn-1", "Remote Trait Shell", "bash")],
        })
        .expect("seed source");

    let push =
        push_snapshot_with_remote(&source_options, &remote, &CloudSyncState::default(), false)
            .expect("push through memory remote");
    assert_eq!(push.status.provider, "memory");
    assert!(
        remote
            .read_if_exists("nyaterm/sync/latest.redb")
            .expect("read pointer")
            .is_some()
    );

    let pull =
        pull_snapshot_with_remote(&target_options, &remote, &CloudSyncState::default(), true)
            .expect("pull through memory remote");
    assert_eq!(pull.status.provider, "memory");

    let loaded = ConnectionStore::open(&target_dir)
        .expect("target store")
        .load_sessions()
        .expect("load target");
    assert_eq!(loaded.connections[0].name, "Remote Trait Shell");

    std::fs::remove_dir_all(source_dir).ok();
    std::fs::remove_dir_all(target_dir).ok();
    std::fs::remove_dir_all(remote_dir).ok();
}

#[test]
fn snippet_remote_codec_matches_legacy_blob_layout_and_syncs() {
    let source_dir = unique_temp_dir("cloud-snippet-source");
    let target_dir = unique_temp_dir("cloud-snippet-target");
    let remote_dir = unique_temp_dir("cloud-snippet-unused");
    let source_options = options(&source_dir, &remote_dir, "source-device");
    let target_options = options(&target_dir, &remote_dir, "target-device");
    let backend = MemorySnippetBackend::default();
    let remote = SnippetRemote::new("gitee_snippet", backend);

    assert_eq!(
        snippet_remote_path(&snippet_remote_filename("nyaterm/sync/latest.redb")).as_deref(),
        Some("nyaterm/sync/latest.redb")
    );
    assert_eq!(
        decode_snippet_blob(&encode_snippet_blob(b"hello")).expect("decode"),
        b"hello"
    );

    ConnectionStore::open(&source_dir)
        .expect("source store")
        .replace_sessions(&SessionsConfig {
            groups: Vec::new(),
            connections: vec![local_connection("conn-1", "Snippet Shell", "bash")],
        })
        .expect("seed source");

    let push =
        push_snapshot_with_remote(&source_options, &remote, &CloudSyncState::default(), false)
            .expect("push snippet");
    assert_eq!(push.status.provider, "gitee_snippet");
    assert!(
        remote
            .read_if_exists("nyaterm/sync/latest.redb")
            .expect("snippet pointer")
            .is_some()
    );

    let pull =
        pull_snapshot_with_remote(&target_options, &remote, &CloudSyncState::default(), true)
            .expect("pull snippet");
    assert_eq!(pull.status.provider, "gitee_snippet");

    let loaded = ConnectionStore::open(&target_dir)
        .expect("target store")
        .load_sessions()
        .expect("load target");
    assert_eq!(loaded.connections[0].name, "Snippet Shell");

    std::fs::remove_dir_all(source_dir).ok();
    std::fs::remove_dir_all(target_dir).ok();
    std::fs::remove_dir_all(remote_dir).ok();
}

#[test]
fn gitee_http_backend_fetches_raw_filename_with_access_token() {
    let settings = GiteeSnippetSyncSettings {
        api_endpoint: "https://gitee.example/api/v5/".to_string(),
        gist_id: "gist-1".to_string(),
        access_token: Some("token-1".to_string()),
    };
    let client = RecordingSnippetHttpClient::new(vec![SnippetHttpResponse {
        status: 200,
        body: encode_snippet_blob(b"hello"),
    }]);
    let backend = GiteeSnippetHttpBackend::new(&settings, client.clone()).expect("backend");

    let content = backend
        .fetch_blob(&snippet_remote_filename("nyaterm/sync/latest.redb"))
        .expect("fetch blob")
        .expect("blob");

    assert_eq!(decode_snippet_blob(&content).expect("decode"), b"hello");
    let requests = client.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, SnippetHttpMethod::Get);
    assert_eq!(
        requests[0].query.get("access_token").map(String::as_str),
        Some("token-1")
    );
    assert!(requests[0].url.contains("/gists/gist-1/raw/nyaterm-"));
}

#[test]
fn github_gist_http_backend_fetches_raw_url_for_truncated_file() {
    let filename = snippet_remote_filename("nyaterm/sync/current.redb.enc");
    let settings = GithubGistSyncSettings {
        gist_id: "gist-2".to_string(),
        access_token: Some("gh-token".to_string()),
    };
    let document = serde_json::json!({
        "files": {
            filename.clone(): {
                "content": "partial",
                "raw_url": "https://gist.example/raw/file",
                "truncated": true
            }
        }
    });
    let client = RecordingSnippetHttpClient::new(vec![
        SnippetHttpResponse {
            status: 200,
            body: document.to_string(),
        },
        SnippetHttpResponse {
            status: 200,
            body: encode_snippet_blob(b"full"),
        },
    ]);
    let backend = GithubGistHttpBackend::new(&settings, client.clone()).expect("backend");

    let content = backend
        .fetch_blob(&filename)
        .expect("fetch blob")
        .expect("blob");

    assert_eq!(decode_snippet_blob(&content).expect("decode"), b"full");
    let requests = client.requests();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].url, "https://api.github.com/gists/gist-2");
    assert_eq!(requests[1].url, "https://gist.example/raw/file");
    assert_eq!(
        requests[0].headers.get("Authorization").map(String::as_str),
        Some("Bearer gh-token")
    );
}

#[test]
fn github_gist_http_backend_retries_retryable_update_conflict() {
    let settings = GithubGistSyncSettings {
        gist_id: "gist-3".to_string(),
        access_token: Some("gh-token".to_string()),
    };
    let client = RecordingSnippetHttpClient::new(vec![
        SnippetHttpResponse {
            status: 409,
            body: r#"{"message":"Gist cannot be updated."}"#.to_string(),
        },
        SnippetHttpResponse {
            status: 200,
            body: "{}".to_string(),
        },
    ]);
    let backend = GithubGistHttpBackend::new(&settings, client.clone()).expect("backend");
    let mut files = BTreeMap::new();
    files.insert("nyaterm-rev.blob".to_string(), Some("payload".to_string()));

    backend.patch_blobs(files).expect("patch retry");

    let requests = client.requests();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0], requests[1]);
    assert_eq!(requests[0].method, SnippetHttpMethod::Patch);
}

#[test]
fn snippet_patch_bodies_match_gitee_and_github_shapes() {
    let mut files = BTreeMap::new();
    files.insert("nyaterm-a.blob".to_string(), Some("payload".to_string()));
    files.insert("nyaterm-b.blob".to_string(), None);

    let gitee = gitee_snippet_patch_body("token", files.clone());
    assert_eq!(gitee["access_token"], "token");
    assert_eq!(gitee["files"]["nyaterm-a.blob"]["content"], "payload");
    assert!(gitee["files"]["nyaterm-b.blob"].is_null());

    let github = github_gist_patch_body(files);
    assert!(github.get("access_token").is_none());
    assert_eq!(github["files"]["nyaterm-a.blob"]["content"], "payload");
    assert!(github["files"]["nyaterm-b.blob"].is_null());
}

#[test]
fn local_cloud_sync_detects_push_conflict() {
    let source_dir = unique_temp_dir("cloud-conflict-source");
    let other_dir = unique_temp_dir("cloud-conflict-other");
    let remote_dir = unique_temp_dir("cloud-conflict-remote");
    let source_options = options(&source_dir, &remote_dir, "source-device");
    let other_options = options(&other_dir, &remote_dir, "other-device");

    ConnectionStore::open(&source_dir)
        .expect("source")
        .replace_sessions(&SessionsConfig {
            groups: Vec::new(),
            connections: vec![local_connection("conn-1", "Local A", "bash")],
        })
        .expect("seed source");
    let source_state = push_local_snapshot(&source_options, &CloudSyncState::default(), false)
        .expect("initial push")
        .state;

    ConnectionStore::open(&other_dir)
        .expect("other")
        .replace_sessions(&SessionsConfig {
            groups: Vec::new(),
            connections: vec![local_connection("conn-2", "Remote B", "zsh")],
        })
        .expect("seed other");
    push_local_snapshot(&other_options, &CloudSyncState::default(), true)
        .expect("remote force push");

    ConnectionStore::open(&source_dir)
        .expect("source reopen")
        .replace_sessions(&SessionsConfig {
            groups: Vec::new(),
            connections: vec![local_connection("conn-1", "Local Changed", "fish")],
        })
        .expect("change source");
    let error =
        push_local_snapshot(&source_options, &source_state, false).expect_err("conflict expected");
    assert!(matches!(error, CloudSyncError::Conflict(_)));

    std::fs::remove_dir_all(source_dir).ok();
    std::fs::remove_dir_all(other_dir).ok();
    std::fs::remove_dir_all(remote_dir).ok();
}

#[test]
fn local_cloud_sync_detects_pull_conflict_until_forced() {
    let source_dir = unique_temp_dir("cloud-pull-conflict-source");
    let target_dir = unique_temp_dir("cloud-pull-conflict-target");
    let other_dir = unique_temp_dir("cloud-pull-conflict-other");
    let remote_dir = unique_temp_dir("cloud-pull-conflict-remote");
    let source_options = options(&source_dir, &remote_dir, "source-device");
    let target_options = options(&target_dir, &remote_dir, "target-device");
    let other_options = options(&other_dir, &remote_dir, "other-device");

    ConnectionStore::open(&source_dir)
        .expect("source")
        .replace_sessions(&SessionsConfig {
            groups: Vec::new(),
            connections: vec![local_connection("conn-1", "Initial", "bash")],
        })
        .expect("seed source");
    push_local_snapshot(&source_options, &CloudSyncState::default(), false).expect("initial push");
    let target_state = pull_local_snapshot(&target_options, &CloudSyncState::default(), true)
        .expect("initial pull")
        .state;

    ConnectionStore::open(&other_dir)
        .expect("other")
        .replace_sessions(&SessionsConfig {
            groups: Vec::new(),
            connections: vec![local_connection("conn-2", "Remote Changed", "zsh")],
        })
        .expect("seed other");
    push_local_snapshot(&other_options, &CloudSyncState::default(), true)
        .expect("remote force push");

    ConnectionStore::open(&target_dir)
        .expect("target reopen")
        .replace_sessions(&SessionsConfig {
            groups: Vec::new(),
            connections: vec![local_connection("conn-1", "Local Changed", "fish")],
        })
        .expect("change target");

    let error = pull_local_snapshot(&target_options, &target_state, false)
        .expect_err("pull conflict expected");
    assert!(matches!(error, CloudSyncError::Conflict(_)));

    pull_local_snapshot(&target_options, &target_state, true).expect("forced pull");
    let loaded = ConnectionStore::open(&target_dir)
        .expect("target final")
        .load_sessions()
        .expect("load target");
    assert_eq!(loaded.connections[0].name, "Remote Changed");

    std::fs::remove_dir_all(source_dir).ok();
    std::fs::remove_dir_all(target_dir).ok();
    std::fs::remove_dir_all(other_dir).ok();
    std::fs::remove_dir_all(remote_dir).ok();
}

#[test]
fn local_cloud_sync_wrong_password_does_not_replace_target() {
    let source_dir = unique_temp_dir("cloud-password-source");
    let target_dir = unique_temp_dir("cloud-password-target");
    let remote_dir = unique_temp_dir("cloud-password-remote");
    let source_options = options(&source_dir, &remote_dir, "source-device");
    let mut wrong_options = options(&target_dir, &remote_dir, "target-device");
    wrong_options.master_password = "wrong".to_string();

    ConnectionStore::open(&source_dir)
        .expect("source")
        .replace_sessions(&SessionsConfig {
            groups: Vec::new(),
            connections: vec![local_connection("conn-1", "Remote State", "bash")],
        })
        .expect("seed source");
    push_local_snapshot(&source_options, &CloudSyncState::default(), false).expect("push");

    ConnectionStore::open(&target_dir)
        .expect("target")
        .replace_sessions(&SessionsConfig {
            groups: Vec::new(),
            connections: vec![local_connection("keep", "Keep Local", "zsh")],
        })
        .expect("seed target");

    let error = pull_local_snapshot(&wrong_options, &CloudSyncState::default(), true)
        .expect_err("wrong password");
    assert!(
        error
            .to_string()
            .contains("cloud snapshot decryption failed")
    );
    let loaded = ConnectionStore::open(&target_dir)
        .expect("target reopen")
        .load_sessions()
        .expect("load target");
    assert_eq!(loaded.connections[0].name, "Keep Local");

    std::fs::remove_dir_all(source_dir).ok();
    std::fs::remove_dir_all(target_dir).ok();
    std::fs::remove_dir_all(remote_dir).ok();
}

#[test]
fn masked_cloud_sync_merge_preserves_provider_secrets() {
    let mut current = CloudSyncSettings::default();
    current.webdav.password = Some("webdav-password".to_string());
    current.s3.secret_access_key = Some("s3-secret".to_string());
    current.google_drive.access_token = Some("google-access".to_string());
    current.google_drive.refresh_token = Some("google-refresh".to_string());
    current.google_drive.client_secret = Some("google-secret".to_string());
    current.onedrive.access_token = Some("onedrive-access".to_string());
    current.aliyun_drive.refresh_token = Some("aliyun-refresh".to_string());
    current.github_gist.access_token = Some("github-token".to_string());

    let mut next = CloudSyncSettings::default();
    next.webdav.password = Some(MASKED_SECRET_VALUE.to_string());
    next.s3.secret_access_key = Some(String::new());
    next.google_drive.access_token = Some(MASKED_SECRET_VALUE.to_string());
    next.google_drive.refresh_token = Some(MASKED_SECRET_VALUE.to_string());
    next.google_drive.client_secret = Some(MASKED_SECRET_VALUE.to_string());
    next.onedrive.access_token = Some(MASKED_SECRET_VALUE.to_string());
    next.aliyun_drive.refresh_token = Some(MASKED_SECRET_VALUE.to_string());
    next.github_gist.access_token = Some("replacement".to_string());

    let merged = merge_masked_cloud_sync_settings(&current, next);

    assert_eq!(merged.webdav.password.as_deref(), Some("webdav-password"));
    assert_eq!(merged.s3.secret_access_key, None);
    assert_eq!(
        merged.google_drive.access_token.as_deref(),
        Some("google-access")
    );
    assert_eq!(
        merged.google_drive.refresh_token.as_deref(),
        Some("google-refresh")
    );
    assert_eq!(
        merged.google_drive.client_secret.as_deref(),
        Some("google-secret")
    );
    assert_eq!(
        merged.onedrive.access_token.as_deref(),
        Some("onedrive-access")
    );
    assert_eq!(
        merged.aliyun_drive.refresh_token.as_deref(),
        Some("aliyun-refresh")
    );
    assert_eq!(
        merged.github_gist.access_token.as_deref(),
        Some("replacement")
    );
}

fn options(config_dir: &Path, remote_dir: &Path, device_id: &str) -> LocalCloudSyncOptions {
    LocalCloudSyncOptions {
        config_dir: config_dir.to_path_buf(),
        portable_key_path: None,
        remote_dir: remote_dir.to_path_buf(),
        remote_root: "nyaterm".to_string(),
        device_id: device_id.to_string(),
        app_version: "test".to_string(),
        master_password: "secret".to_string(),
        enabled: true,
    }
}

fn local_connection(id: &str, name: &str, shell: &str) -> SavedConnection {
    SavedConnection {
        id: id.to_string(),
        name: name.to_string(),
        config: ConnectionType::LocalTerminal {
            shell_path: shell.to_string(),
            shell_args: String::new(),
            working_dir: None,
            ai_execution_profile: AiExecutionProfile::Auto,
        },
        group_id: None,
        description: None,
        sort_order: 0,
        icon: None,
        icon_auto_detect: None,
        auth: None,
        network: None,
        post_login: None,
        created_at_ms: None,
        updated_at_ms: None,
        last_used_at_ms: None,
    }
}

fn unique_temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "nyaterm-cloud-sync-{name}-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn history_line(id: &str, timestamp_ms: u64) -> String {
    serde_json::json!({
        "domain": CLOUD_SYNC_HISTORY_DOMAIN,
        "event": CLOUD_SYNC_HISTORY_EVENT,
        "message": format!("history {id}"),
        "data": {
            "id": id,
            "timestamp_ms": timestamp_ms,
            "kind": "sync",
            "status": "success",
            "trigger": "manual_pull",
            "provider": "webdav",
            "revision": null,
            "duration_ms": 1,
        }
    })
    .to_string()
}
