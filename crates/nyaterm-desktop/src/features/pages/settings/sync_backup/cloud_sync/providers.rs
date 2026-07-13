use super::*;

impl NyaTermApp {
    pub(super) fn cloud_sync_webdav_provider_section(
        &mut self,
        webdav_password_value: String,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        settings_form_section(
            palette,
            Some("WebDAV"),
            Some("Endpoint and credentials for the selected WebDAV target."),
            div().flex().flex_col().gap_3().child(
                div()
                    .grid()
                    .grid_cols(2)
                    .gap_2()
                    .child(self.cloud_sync_input(
                        "cloud-webdav-endpoint",
                        "Endpoint",
                        self.cloud_sync_settings.webdav.endpoint.clone(),
                        CloudSyncInputField::WebdavEndpoint,
                        cx,
                    ))
                    .child(self.cloud_sync_input(
                        "cloud-webdav-root",
                        "Root",
                        self.cloud_sync_settings.webdav.root.clone(),
                        CloudSyncInputField::WebdavRoot,
                        cx,
                    ))
                    .child(self.cloud_sync_input(
                        "cloud-webdav-username",
                        "Username",
                        self.cloud_sync_settings.webdav.username.clone(),
                        CloudSyncInputField::WebdavUsername,
                        cx,
                    ))
                    .child(self.cloud_sync_input(
                        "cloud-webdav-password",
                        "Password",
                        webdav_password_value,
                        CloudSyncInputField::WebdavPassword,
                        cx,
                    )),
            ),
        )
    }

    pub(super) fn cloud_sync_s3_provider_section(
        &mut self,
        s3_access_key_value: String,
        s3_secret_key_value: String,
        s3_session_token_value: String,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        settings_form_section(
            palette,
            Some("S3 Compatible"),
            Some("Bucket, region, and access keys for S3-compatible storage."),
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    div()
                        .grid()
                        .grid_cols(3)
                        .gap_2()
                        .child(self.cloud_sync_input(
                            "cloud-s3-endpoint",
                            "Endpoint",
                            self.cloud_sync_settings.s3.endpoint.clone(),
                            CloudSyncInputField::S3Endpoint,
                            cx,
                        ))
                        .child(self.cloud_sync_input(
                            "cloud-s3-bucket",
                            "Bucket",
                            self.cloud_sync_settings.s3.bucket.clone(),
                            CloudSyncInputField::S3Bucket,
                            cx,
                        ))
                        .child(self.cloud_sync_input(
                            "cloud-s3-region",
                            "Region",
                            self.cloud_sync_settings.s3.region.clone(),
                            CloudSyncInputField::S3Region,
                            cx,
                        ))
                        .child(self.cloud_sync_input(
                            "cloud-s3-root",
                            "S3 Root",
                            self.cloud_sync_settings.s3.root.clone(),
                            CloudSyncInputField::S3Root,
                            cx,
                        ))
                        .child(self.cloud_sync_input(
                            "cloud-s3-access-key",
                            "Access Key",
                            s3_access_key_value,
                            CloudSyncInputField::S3AccessKeyId,
                            cx,
                        ))
                        .child(self.cloud_sync_input(
                            "cloud-s3-secret-key",
                            "Secret Key",
                            s3_secret_key_value,
                            CloudSyncInputField::S3SecretAccessKey,
                            cx,
                        ))
                        .child(self.cloud_sync_input(
                            "cloud-s3-session-token",
                            "Session Token",
                            s3_session_token_value,
                            CloudSyncInputField::S3SessionToken,
                            cx,
                        )),
                )
                .child(settings_form_row(
                    palette,
                    "Virtual host style",
                    Some(SharedString::from(
                        "Use virtual-hosted-style URLs instead of path style.",
                    )),
                    settings_switch(
                        palette,
                        "cloud-s3-url-style",
                        self.cloud_sync_settings.s3.virtual_host_style,
                        cx.listener(|this, _, _, cx| {
                            this.toggle_s3_virtual_host_style(cx);
                        }),
                    ),
                )),
        )
    }

    pub(super) fn cloud_sync_google_drive_provider_section(
        &mut self,
        google_drive_access_token_value: String,
        google_drive_refresh_token_value: String,
        google_drive_client_secret_value: String,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        settings_form_section(
            palette,
            Some("Google Drive"),
            Some("OAuth client credentials and tokens for Drive sync."),
            div()
                .grid()
                .grid_cols(2)
                .gap_2()
                .child(self.cloud_sync_input(
                    "cloud-google-drive-root",
                    "Drive Root",
                    self.cloud_sync_settings.google_drive.root.clone(),
                    CloudSyncInputField::GoogleDriveRoot,
                    cx,
                ))
                .child(
                    self.cloud_sync_input(
                        "cloud-google-drive-client-id",
                        "Client ID",
                        self.cloud_sync_settings
                            .google_drive
                            .client_id
                            .clone()
                            .unwrap_or_default(),
                        CloudSyncInputField::GoogleDriveClientId,
                        cx,
                    ),
                )
                .child(self.cloud_sync_input(
                    "cloud-google-drive-client-secret",
                    "Client Secret",
                    google_drive_client_secret_value,
                    CloudSyncInputField::GoogleDriveClientSecret,
                    cx,
                ))
                .child(self.cloud_sync_input(
                    "cloud-google-drive-access-token",
                    "Access Token",
                    google_drive_access_token_value,
                    CloudSyncInputField::GoogleDriveAccessToken,
                    cx,
                ))
                .child(self.cloud_sync_input(
                    "cloud-google-drive-refresh-token",
                    "Refresh Token",
                    google_drive_refresh_token_value,
                    CloudSyncInputField::GoogleDriveRefreshToken,
                    cx,
                )),
        )
    }

    pub(super) fn cloud_sync_onedrive_provider_section(
        &mut self,
        onedrive_access_token_value: String,
        onedrive_refresh_token_value: String,
        onedrive_client_secret_value: String,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        settings_form_section(
            palette,
            Some("OneDrive"),
            Some("Microsoft Graph credentials for OneDrive sync."),
            div()
                .grid()
                .grid_cols(2)
                .gap_2()
                .child(self.cloud_sync_input(
                    "cloud-onedrive-root",
                    "OneDrive Root",
                    self.cloud_sync_settings.onedrive.root.clone(),
                    CloudSyncInputField::OneDriveRoot,
                    cx,
                ))
                .child(
                    self.cloud_sync_input(
                        "cloud-onedrive-client-id",
                        "Client ID",
                        self.cloud_sync_settings
                            .onedrive
                            .client_id
                            .clone()
                            .unwrap_or_default(),
                        CloudSyncInputField::OneDriveClientId,
                        cx,
                    ),
                )
                .child(self.cloud_sync_input(
                    "cloud-onedrive-client-secret",
                    "Client Secret",
                    onedrive_client_secret_value,
                    CloudSyncInputField::OneDriveClientSecret,
                    cx,
                ))
                .child(self.cloud_sync_input(
                    "cloud-onedrive-access-token",
                    "Access Token",
                    onedrive_access_token_value,
                    CloudSyncInputField::OneDriveAccessToken,
                    cx,
                ))
                .child(self.cloud_sync_input(
                    "cloud-onedrive-refresh-token",
                    "Refresh Token",
                    onedrive_refresh_token_value,
                    CloudSyncInputField::OneDriveRefreshToken,
                    cx,
                )),
        )
    }

    pub(super) fn cloud_sync_aliyun_drive_provider_section(
        &mut self,
        aliyun_drive_access_token_value: String,
        aliyun_drive_refresh_token_value: String,
        aliyun_drive_client_secret_value: String,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        settings_form_section(
            palette,
            Some("Aliyun Drive"),
            Some("AliyunDrive OAuth credentials and tokens."),
            div()
                .grid()
                .grid_cols(2)
                .gap_2()
                .child(self.cloud_sync_input(
                    "cloud-aliyun-drive-root",
                    "Drive Root",
                    self.cloud_sync_settings.aliyun_drive.root.clone(),
                    CloudSyncInputField::AliyunDriveRoot,
                    cx,
                ))
                .child(
                    self.cloud_sync_input(
                        "cloud-aliyun-drive-client-id",
                        "Client ID",
                        self.cloud_sync_settings
                            .aliyun_drive
                            .client_id
                            .clone()
                            .unwrap_or_default(),
                        CloudSyncInputField::AliyunDriveClientId,
                        cx,
                    ),
                )
                .child(self.cloud_sync_input(
                    "cloud-aliyun-drive-client-secret",
                    "Client Secret",
                    aliyun_drive_client_secret_value,
                    CloudSyncInputField::AliyunDriveClientSecret,
                    cx,
                ))
                .child(self.cloud_sync_input(
                    "cloud-aliyun-drive-access-token",
                    "Access Token",
                    aliyun_drive_access_token_value,
                    CloudSyncInputField::AliyunDriveAccessToken,
                    cx,
                ))
                .child(self.cloud_sync_input(
                    "cloud-aliyun-drive-refresh-token",
                    "Refresh Token",
                    aliyun_drive_refresh_token_value,
                    CloudSyncInputField::AliyunDriveRefreshToken,
                    cx,
                )),
        )
    }

    pub(super) fn cloud_sync_gitee_provider_section(
        &mut self,
        gitee_token_value: String,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        settings_form_section(
            palette,
            Some("Gitee Snippet"),
            Some("API endpoint, snippet id, and personal access token."),
            div()
                .grid()
                .grid_cols(3)
                .gap_2()
                .child(self.cloud_sync_input(
                    "cloud-gitee-endpoint",
                    "API Endpoint",
                    self.cloud_sync_settings.gitee_snippet.api_endpoint.clone(),
                    CloudSyncInputField::GiteeEndpoint,
                    cx,
                ))
                .child(self.cloud_sync_input(
                    "cloud-gitee-gist",
                    "Snippet ID",
                    self.cloud_sync_settings.gitee_snippet.gist_id.clone(),
                    CloudSyncInputField::GiteeGistId,
                    cx,
                ))
                .child(self.cloud_sync_input(
                    "cloud-gitee-token",
                    "Token",
                    gitee_token_value,
                    CloudSyncInputField::GiteeToken,
                    cx,
                )),
        )
    }

    pub(super) fn cloud_sync_github_provider_section(
        &mut self,
        github_token_value: String,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        settings_form_section(
            palette,
            Some("GitHub Gist"),
            Some("Gist id and token for encrypted snapshot sync."),
            div()
                .grid()
                .grid_cols(2)
                .gap_2()
                .child(self.cloud_sync_input(
                    "cloud-github-gist",
                    "Gist ID",
                    self.cloud_sync_settings.github_gist.gist_id.clone(),
                    CloudSyncInputField::GithubGistId,
                    cx,
                ))
                .child(self.cloud_sync_input(
                    "cloud-github-token",
                    "Token",
                    github_token_value,
                    CloudSyncInputField::GithubToken,
                    cx,
                )),
        )
    }
}
