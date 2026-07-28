use std::sync::Mutex;

use nyaterm_core::WebdavSyncSettings;
use zed_reqwest::StatusCode;

use super::aliyun::{AliyunDriveType, aliyun_drive_error_code, aliyun_drive_remote_error};
use super::google_drive::google_drive_multipart_body;
use super::helpers::{
    build_digest_authorization, form_urlencoded, parse_digest_challenge, percent_encode_path,
};
use super::onedrive::onedrive_item_path;
use super::{NativeAliyunDriveRemote, NativeOneDriveRemote, NativeWebdavRemote};

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
