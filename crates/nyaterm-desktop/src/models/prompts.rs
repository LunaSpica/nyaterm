use nyaterm_core::{ConfigBackupInfo, DiagnosticsExportInfo};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CloudSyncInputField {
    RemoteRoot,
    WebdavEndpoint,
    WebdavRoot,
    WebdavUsername,
    WebdavPassword,
    S3Endpoint,
    S3Bucket,
    S3Region,
    S3Root,
    S3AccessKeyId,
    S3SecretAccessKey,
    S3SessionToken,
    GoogleDriveRoot,
    GoogleDriveAccessToken,
    GoogleDriveRefreshToken,
    GoogleDriveClientId,
    GoogleDriveClientSecret,
    OneDriveRoot,
    OneDriveAccessToken,
    OneDriveRefreshToken,
    OneDriveClientId,
    OneDriveClientSecret,
    AliyunDriveRoot,
    AliyunDriveType,
    AliyunDriveAccessToken,
    AliyunDriveRefreshToken,
    AliyunDriveClientId,
    AliyunDriveClientSecret,
    GiteeEndpoint,
    GiteeGistId,
    GiteeToken,
    GithubGistId,
    GithubToken,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AiInputField {
    Model,
    BaseUrl,
    ApiKey,
    RequestUserAgent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AiActionListKind {
    Terminal,
    File,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AiActionEditorField {
    Name,
    Prompt,
}

impl AiActionEditorField {
    pub(crate) fn next(self) -> Self {
        match self {
            Self::Name => Self::Prompt,
            Self::Prompt => Self::Name,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AiCredentialEditorField {
    Name,
    BaseUrl,
    ApiKey,
}

impl AiCredentialEditorField {
    pub(crate) fn next(self, builtin: bool) -> Self {
        if builtin {
            Self::ApiKey
        } else {
            match self {
                Self::Name => Self::BaseUrl,
                Self::BaseUrl => Self::ApiKey,
                Self::ApiKey => Self::Name,
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TranslateInputField {
    TargetLanguage,
    Text,
    SettingsTargetLanguage,
    DeeplApiKey,
    BaiduAppId,
    BaiduAppKey,
    AliAppId,
    AliAppKey,
    YoudaoAppId,
    YoudaoAppKey,
}

impl TranslateInputField {
    pub(crate) fn is_settings_field(self) -> bool {
        matches!(
            self,
            Self::SettingsTargetLanguage
                | Self::DeeplApiKey
                | Self::BaiduAppId
                | Self::BaiduAppKey
                | Self::AliAppId
                | Self::AliAppKey
                | Self::YoudaoAppId
                | Self::YoudaoAppKey
        )
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct CloudSyncSecretDraft {
    pub(crate) webdav_password: String,
    pub(crate) s3_access_key_id: String,
    pub(crate) s3_secret_access_key: String,
    pub(crate) s3_session_token: String,
    pub(crate) google_drive_access_token: String,
    pub(crate) google_drive_refresh_token: String,
    pub(crate) google_drive_client_secret: String,
    pub(crate) onedrive_access_token: String,
    pub(crate) onedrive_refresh_token: String,
    pub(crate) onedrive_client_secret: String,
    pub(crate) aliyun_drive_access_token: String,
    pub(crate) aliyun_drive_refresh_token: String,
    pub(crate) aliyun_drive_client_secret: String,
    pub(crate) gitee_token: String,
    pub(crate) github_token: String,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct TranslationSecretDraft {
    pub(crate) deepl_api_key: String,
    pub(crate) baidu_app_key: String,
    pub(crate) ali_app_key: String,
    pub(crate) youdao_app_key: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TransferPathPromptKind {
    UploadFile,
    UploadDirectory,
    DownloadDirectory,
}

#[derive(Debug)]
pub(crate) enum TransferPathPromptResult {
    Selected(Vec<PathBuf>),
    Cancelled,
    Failed(String),
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RecordingPathPromptKind {
    Start,
    SaveTranscript,
}

#[derive(Debug)]
pub(crate) enum RecordingPathPromptResult {
    Selected(PathBuf),
    Cancelled,
    Failed(String),
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConfigPathPromptKind {
    Export,
    Import,
    PortableExport,
    PortableImport,
    EncryptedPortableExport,
    EncryptedPortableImport,
}

#[derive(Debug)]
pub(crate) enum ConfigPathPromptResult {
    Exported(ConfigBackupInfo),
    Imported(ConfigBackupInfo),
    Cancelled,
    Failed(String),
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SnapshotPasswordPromptKind {
    Export,
    Import,
    CloudPush,
    CloudPull,
    CloudForcePush,
    CloudForcePull,
    CloudProviderPush,
    CloudProviderPull,
    CloudProviderForcePush,
    CloudProviderForcePull,
}

#[derive(Debug, Clone)]
pub(crate) struct SnapshotPasswordPromptState {
    pub(crate) kind: SnapshotPasswordPromptKind,
    pub(crate) value: String,
}

#[derive(Debug, Clone)]
pub(crate) struct CloudSyncConflictState {
    pub(crate) provider: String,
    pub(crate) message: String,
    pub(crate) provider_action: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DiagnosticsPathPromptKind {
    Export,
}

#[derive(Debug)]
pub(crate) enum DiagnosticsPathPromptResult {
    Exported(DiagnosticsExportInfo),
    Cancelled,
    Failed(String),
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeywordHighlightPathPromptKind {
    Import,
}

#[derive(Debug)]
pub(crate) enum KeywordHighlightPathPromptResult {
    Imported {
        imported_rules: usize,
        updated_rules: usize,
        total_rules: usize,
    },
    Cancelled,
    Failed(String),
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QuickCommandImportPathPromptKind {
    NyatermJson,
    WindTermQuickbar,
    XshellXts,
}

#[derive(Debug)]
pub(crate) enum QuickCommandImportPathPromptResult {
    Imported {
        imported_commands: usize,
        imported_categories: usize,
        updated_commands: usize,
        total_commands: usize,
        total_categories: usize,
    },
    Cancelled,
    Failed(String),
    Closed,
}
