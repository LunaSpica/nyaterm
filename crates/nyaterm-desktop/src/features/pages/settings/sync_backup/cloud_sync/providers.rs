use super::*;

impl NyaTermApp {
    pub(super) fn cloud_sync_webdav_provider_fields(
        &mut self,
        password: String,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .grid()
            .grid_cols(2)
            .gap_2()
            .child(self.cloud_sync_input(
                "cloud-webdav-endpoint",
                self.tr("settings.webdavEndpoint"),
                self.cloud_sync_settings.webdav.endpoint.clone(),
                CloudSyncInputField::WebdavEndpoint,
                cx,
            ))
            .child(self.cloud_sync_input(
                "cloud-webdav-root",
                self.tr("settings.providerRoot"),
                self.cloud_sync_settings.webdav.root.clone(),
                CloudSyncInputField::WebdavRoot,
                cx,
            ))
            .child(self.cloud_sync_input(
                "cloud-webdav-username",
                self.tr("dialog.username"),
                self.cloud_sync_settings.webdav.username.clone(),
                CloudSyncInputField::WebdavUsername,
                cx,
            ))
            .child(self.cloud_sync_input(
                "cloud-webdav-password",
                self.tr("dialog.password"),
                password,
                CloudSyncInputField::WebdavPassword,
                cx,
            ))
            .into_any_element()
    }

    pub(super) fn cloud_sync_s3_provider_fields(
        &mut self,
        access_key: String,
        secret_key: String,
        session_token: String,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let palette = self.theme_palette();
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .grid()
                    .grid_cols(2)
                    .gap_2()
                    .child(self.cloud_sync_input(
                        "cloud-s3-endpoint",
                        self.tr("settings.s3Endpoint"),
                        self.cloud_sync_settings.s3.endpoint.clone(),
                        CloudSyncInputField::S3Endpoint,
                        cx,
                    ))
                    .child(self.cloud_sync_input(
                        "cloud-s3-bucket",
                        self.tr("settings.s3Bucket"),
                        self.cloud_sync_settings.s3.bucket.clone(),
                        CloudSyncInputField::S3Bucket,
                        cx,
                    ))
                    .child(self.cloud_sync_input(
                        "cloud-s3-region",
                        self.tr("settings.s3Region"),
                        self.cloud_sync_settings.s3.region.clone(),
                        CloudSyncInputField::S3Region,
                        cx,
                    ))
                    .child(self.cloud_sync_input(
                        "cloud-s3-root",
                        self.tr("settings.providerRoot"),
                        self.cloud_sync_settings.s3.root.clone(),
                        CloudSyncInputField::S3Root,
                        cx,
                    ))
                    .child(self.cloud_sync_input(
                        "cloud-s3-access-key",
                        self.tr("settings.s3AccessKeyId"),
                        access_key,
                        CloudSyncInputField::S3AccessKeyId,
                        cx,
                    ))
                    .child(self.cloud_sync_input(
                        "cloud-s3-secret-key",
                        self.tr("settings.s3SecretAccessKey"),
                        secret_key,
                        CloudSyncInputField::S3SecretAccessKey,
                        cx,
                    ))
                    .child(self.cloud_sync_input(
                        "cloud-s3-session-token",
                        self.tr("settings.s3SessionToken"),
                        session_token,
                        CloudSyncInputField::S3SessionToken,
                        cx,
                    )),
            )
            .child(settings_form_row(
                palette,
                self.tr("settings.s3VirtualHostStyle"),
                Some(SharedString::from(
                    self.tr("settings.s3VirtualHostStyleDesc"),
                )),
                settings_switch_with_enabled(
                    palette,
                    "cloud-s3-url-style",
                    self.cloud_sync_settings.s3.virtual_host_style,
                    self.cloud_sync_form_enabled(),
                    cx.listener(|this, _, _, cx| {
                        this.toggle_s3_virtual_host_style(cx);
                    }),
                ),
            ))
            .into_any_element()
    }

    pub(super) fn cloud_sync_oauth_provider_fields(
        &mut self,
        provider: &'static str,
        access_token: String,
        refresh_token: String,
        client_secret: String,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let (root, client_id, root_field, access_field, refresh_field, id_field, secret_field) =
            match provider {
                "google_drive" => (
                    self.cloud_sync_settings.google_drive.root.clone(),
                    self.cloud_sync_settings
                        .google_drive
                        .client_id
                        .clone()
                        .unwrap_or_default(),
                    CloudSyncInputField::GoogleDriveRoot,
                    CloudSyncInputField::GoogleDriveAccessToken,
                    CloudSyncInputField::GoogleDriveRefreshToken,
                    CloudSyncInputField::GoogleDriveClientId,
                    CloudSyncInputField::GoogleDriveClientSecret,
                ),
                _ => (
                    self.cloud_sync_settings.onedrive.root.clone(),
                    self.cloud_sync_settings
                        .onedrive
                        .client_id
                        .clone()
                        .unwrap_or_default(),
                    CloudSyncInputField::OneDriveRoot,
                    CloudSyncInputField::OneDriveAccessToken,
                    CloudSyncInputField::OneDriveRefreshToken,
                    CloudSyncInputField::OneDriveClientId,
                    CloudSyncInputField::OneDriveClientSecret,
                ),
            };
        let prefix = if provider == "google_drive" {
            "cloud-google-drive"
        } else {
            "cloud-onedrive"
        };

        div()
            .grid()
            .grid_cols(2)
            .gap_2()
            .child(self.cloud_sync_input(
                if provider == "google_drive" {
                    "cloud-google-drive-root"
                } else {
                    "cloud-onedrive-root"
                },
                self.tr("settings.providerRoot"),
                root,
                root_field,
                cx,
            ))
            .child(self.cloud_sync_input(
                if provider == "google_drive" {
                    "cloud-google-drive-access-token"
                } else {
                    "cloud-onedrive-access-token"
                },
                self.tr("settings.driveAccessToken"),
                access_token,
                access_field,
                cx,
            ))
            .child(self.cloud_sync_input(
                if provider == "google_drive" {
                    "cloud-google-drive-refresh-token"
                } else {
                    "cloud-onedrive-refresh-token"
                },
                self.tr("settings.driveRefreshToken"),
                refresh_token,
                refresh_field,
                cx,
            ))
            .child(self.cloud_sync_input(
                if provider == "google_drive" {
                    "cloud-google-drive-client-id"
                } else {
                    "cloud-onedrive-client-id"
                },
                self.tr("settings.driveClientId"),
                client_id,
                id_field,
                cx,
            ))
            .child(self.cloud_sync_input(
                if provider == "google_drive" {
                    "cloud-google-drive-client-secret"
                } else {
                    "cloud-onedrive-client-secret"
                },
                self.tr("settings.driveClientSecret"),
                client_secret,
                secret_field,
                cx,
            ))
            .id(SharedString::from(format!("{prefix}-fields")))
            .into_any_element()
    }

    pub(super) fn cloud_sync_aliyun_provider_fields(
        &mut self,
        access_token: String,
        refresh_token: String,
        client_secret: String,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .grid()
            .grid_cols(2)
            .gap_2()
            .child(self.cloud_sync_input(
                "cloud-aliyun-drive-root",
                self.tr("settings.providerRoot"),
                self.cloud_sync_settings.aliyun_drive.root.clone(),
                CloudSyncInputField::AliyunDriveRoot,
                cx,
            ))
            .child(self.cloud_sync_input(
                "cloud-aliyun-drive-type",
                self.tr("settings.aliyunDriveType"),
                self.cloud_sync_settings.aliyun_drive.drive_type.clone(),
                CloudSyncInputField::AliyunDriveType,
                cx,
            ))
            .child(self.cloud_sync_input(
                "cloud-aliyun-drive-access-token",
                self.tr("settings.driveAccessToken"),
                access_token,
                CloudSyncInputField::AliyunDriveAccessToken,
                cx,
            ))
            .child(self.cloud_sync_input(
                "cloud-aliyun-drive-refresh-token",
                self.tr("settings.driveRefreshToken"),
                refresh_token,
                CloudSyncInputField::AliyunDriveRefreshToken,
                cx,
            ))
            .child(
                self.cloud_sync_input(
                    "cloud-aliyun-drive-client-id",
                    self.tr("settings.driveClientId"),
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
                self.tr("settings.driveClientSecret"),
                client_secret,
                CloudSyncInputField::AliyunDriveClientSecret,
                cx,
            ))
            .into_any_element()
    }

    pub(super) fn cloud_sync_gitee_provider_fields(
        &mut self,
        token: String,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .grid()
            .grid_cols(2)
            .gap_2()
            .child(self.cloud_sync_input(
                "cloud-gitee-endpoint",
                self.tr("settings.giteeSnippetApiEndpoint"),
                self.cloud_sync_settings.gitee_snippet.api_endpoint.clone(),
                CloudSyncInputField::GiteeEndpoint,
                cx,
            ))
            .child(self.cloud_sync_input(
                "cloud-gitee-gist",
                self.tr("settings.giteeSnippetId"),
                self.cloud_sync_settings.gitee_snippet.gist_id.clone(),
                CloudSyncInputField::GiteeGistId,
                cx,
            ))
            .child(self.cloud_sync_input(
                "cloud-gitee-token",
                self.tr("settings.giteeSnippetAccessToken"),
                token,
                CloudSyncInputField::GiteeToken,
                cx,
            ))
            .into_any_element()
    }

    pub(super) fn cloud_sync_github_provider_fields(
        &mut self,
        token: String,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .grid()
            .grid_cols(2)
            .gap_2()
            .child(self.cloud_sync_input(
                "cloud-github-gist",
                self.tr("settings.githubGistId"),
                self.cloud_sync_settings.github_gist.gist_id.clone(),
                CloudSyncInputField::GithubGistId,
                cx,
            ))
            .child(self.cloud_sync_input(
                "cloud-github-token",
                self.tr("settings.githubGistAccessToken"),
                token,
                CloudSyncInputField::GithubToken,
                cx,
            ))
            .into_any_element()
    }
}
