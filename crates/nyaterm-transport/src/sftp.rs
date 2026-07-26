//! SFTP browsing and file transfer.
//!
//! Split out of `lib.rs` by domain. The wire protocol, retry and resume
//! behaviour, conflict resolution and progress reporting are unchanged; this
//! only moves the code.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use russh_sftp::client::SftpSession;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::{
    PROCESS_TIMEOUT, SFTP_TRANSFER_CANCELLED, SftpAttributeUpdate, SftpDuplicateDecision,
    SftpDuplicatePolicy, SftpDuplicateRequest, SftpDuplicateResolver, SftpFileEntry,
    SftpFileProperties, SftpFileType, SftpRemoteTextFile, SftpService, SftpTransferControl,
    SftpTransferDirection, SftpTransferOptions, SftpTransferProgress, SftpTransferSummary,
    SftpWriteTextResult, SshMultiplexHandle, SshProcessService, SshSessionConfig,
    close_sftp_session, open_sftp_session, run_sftp_operation,
};

impl SftpService {
    pub fn new(config: SshSessionConfig) -> Self {
        Self {
            config,
            multiplex: None,
        }
    }

    pub fn with_multiplex(
        config: SshSessionConfig,
        multiplex: SshMultiplexHandle,
    ) -> anyhow::Result<Self> {
        multiplex.ensure_matches_config(&config)?;
        Ok(Self {
            config,
            multiplex: Some(multiplex),
        })
    }

    fn run_operation<T, F>(&self, operation: F) -> anyhow::Result<T>
    where
        T: Send + 'static,
        F: Future<Output = anyhow::Result<T>> + Send + 'static,
    {
        if let Some(multiplex) = self.multiplex.as_ref() {
            multiplex.block_on(operation)
        } else {
            run_sftp_operation(operation)
        }
    }

    pub fn list_dir(&self, remote_path: impl AsRef<str>) -> anyhow::Result<Vec<SftpFileEntry>> {
        let remote_path = remote_path.as_ref().to_string();
        let config = self.config.clone();
        let multiplex = self.multiplex.clone();
        self.run_operation(async move {
            let session = open_sftp_session(&config, multiplex.as_ref()).await?;
            let sftp = &session.sftp;
            let mut entries = Vec::new();
            for entry in sftp.read_dir(remote_path).await? {
                let metadata = entry.metadata();
                entries.push(SftpFileEntry {
                    name: entry.file_name(),
                    path: entry.path(),
                    file_type: match entry.file_type() {
                        russh_sftp::protocol::FileType::File => SftpFileType::File,
                        russh_sftp::protocol::FileType::Dir => SftpFileType::Directory,
                        russh_sftp::protocol::FileType::Symlink => SftpFileType::Symlink,
                        russh_sftp::protocol::FileType::Other => SftpFileType::Other,
                    },
                    size: metadata.size,
                    permissions: metadata.permissions,
                    owner: metadata.uid.map(|uid| uid.to_string()).unwrap_or_default(),
                    group: metadata.gid.map(|gid| gid.to_string()).unwrap_or_default(),
                    modified_at: metadata.mtime,
                });
            }
            entries.sort_by(|left, right| {
                (left.file_type != SftpFileType::Directory)
                    .cmp(&(right.file_type != SftpFileType::Directory))
                    .then(left.name.cmp(&right.name))
            });
            close_sftp_session(session).await;
            Ok(entries)
        })
    }

    pub fn rename_path(
        &self,
        old_path: impl AsRef<str>,
        new_path: impl AsRef<str>,
    ) -> anyhow::Result<()> {
        let old_path = old_path.as_ref().to_string();
        let new_path = new_path.as_ref().to_string();
        let config = self.config.clone();
        let multiplex = self.multiplex.clone();
        self.run_operation(async move {
            let session = open_sftp_session(&config, multiplex.as_ref()).await?;
            let result = session.sftp.rename(old_path, new_path).await;
            close_sftp_session(session).await;
            result?;
            Ok(())
        })
    }

    pub fn delete_path(&self, remote_path: impl AsRef<str>) -> anyhow::Result<()> {
        let remote_path = remote_path.as_ref().to_string();
        let config = self.config.clone();
        let multiplex = self.multiplex.clone();
        self.run_operation(async move {
            let session = open_sftp_session(&config, multiplex.as_ref()).await?;
            let result = delete_remote_path_recursive(&session.sftp, &remote_path).await;
            close_sftp_session(session).await;
            result
        })
    }

    pub fn create_dir_path(
        &self,
        remote_path: impl AsRef<str>,
        mode: Option<u32>,
    ) -> anyhow::Result<()> {
        let remote_path = remote_path.as_ref().to_string();
        let config = self.config.clone();
        let multiplex = self.multiplex.clone();
        self.run_operation(async move {
            let session = open_sftp_session(&config, multiplex.as_ref()).await?;
            let result = async {
                session.sftp.create_dir(remote_path.clone()).await?;
                if let Some(mode) = mode {
                    session
                        .sftp
                        .set_metadata(
                            remote_path,
                            russh_sftp::protocol::FileAttributes {
                                permissions: Some(mode),
                                ..russh_sftp::protocol::FileAttributes::empty()
                            },
                        )
                        .await?;
                }
                Ok(())
            }
            .await;
            close_sftp_session(session).await;
            result
        })
    }

    pub fn create_file_path(
        &self,
        remote_path: impl AsRef<str>,
        mode: Option<u32>,
    ) -> anyhow::Result<()> {
        let remote_path = remote_path.as_ref().to_string();
        let config = self.config.clone();
        let multiplex = self.multiplex.clone();
        self.run_operation(async move {
            let session = open_sftp_session(&config, multiplex.as_ref()).await?;
            let result = async {
                let _file = session.sftp.create(remote_path.clone()).await?;
                if let Some(mode) = mode {
                    session
                        .sftp
                        .set_metadata(
                            remote_path,
                            russh_sftp::protocol::FileAttributes {
                                permissions: Some(mode),
                                ..russh_sftp::protocol::FileAttributes::empty()
                            },
                        )
                        .await?;
                }
                Ok(())
            }
            .await;
            close_sftp_session(session).await;
            result
        })
    }

    pub fn create_symlink_path(
        &self,
        link_path: impl AsRef<str>,
        target_path: impl AsRef<str>,
    ) -> anyhow::Result<()> {
        let link_path = link_path.as_ref().to_string();
        let target_path = target_path.as_ref().to_string();
        let config = self.config.clone();
        let multiplex = self.multiplex.clone();
        self.run_operation(async move {
            let session = open_sftp_session(&config, multiplex.as_ref()).await?;
            let result = session
                .sftp
                .symlink_openssh(target_path, link_path)
                .await
                .map_err(Into::into);
            close_sftp_session(session).await;
            result
        })
    }

    pub fn file_properties(
        &self,
        remote_path: impl AsRef<str>,
    ) -> anyhow::Result<SftpFileProperties> {
        let remote_path = remote_path.as_ref().to_string();
        let config = self.config.clone();
        let multiplex = self.multiplex.clone();
        self.run_operation(async move {
            let session = open_sftp_session(&config, multiplex.as_ref()).await?;
            let result = async {
                let attrs = session.sftp.symlink_metadata(remote_path.clone()).await?;
                let file_type = attrs_to_sftp_file_type(&attrs);
                let owner = resolve_remote_user_name(&config, multiplex.clone(), attrs.uid)
                    .unwrap_or_else(|| {
                        attrs.uid.map(|value| value.to_string()).unwrap_or_default()
                    });
                let group = resolve_remote_group_name(&config, multiplex.clone(), attrs.gid)
                    .unwrap_or_else(|| {
                        attrs.gid.map(|value| value.to_string()).unwrap_or_default()
                    });
                let permissions = attrs.permissions;
                Ok(SftpFileProperties {
                    name: remote_file_name(&remote_path),
                    path: remote_path,
                    file_type,
                    size: attrs.size,
                    permissions,
                    permissions_symbolic: permissions
                        .map(|mode| format_sftp_permissions(file_type, mode))
                        .unwrap_or_else(|| "-".to_string()),
                    owner,
                    group,
                    uid: attrs.uid,
                    gid: attrs.gid,
                    modified_at: attrs.mtime,
                    accessed_at: attrs.atime,
                })
            }
            .await;
            close_sftp_session(session).await;
            result
        })
    }

    pub fn update_path_attributes(
        &self,
        remote_path: impl AsRef<str>,
        update: SftpAttributeUpdate,
    ) -> anyhow::Result<()> {
        let remote_path = remote_path.as_ref().to_string();
        let config = self.config.clone();
        let multiplex = self.multiplex.clone();
        let uid = resolve_remote_user_value(&config, multiplex.clone(), update.owner.as_deref())?;
        let gid = resolve_remote_group_value(&config, multiplex.clone(), update.group.as_deref())?;
        let mode = update.mode;
        if mode.is_none() && uid.is_none() && gid.is_none() {
            return Ok(());
        }
        self.run_operation(async move {
            let session = open_sftp_session(&config, multiplex.as_ref()).await?;
            let result = async {
                let mut paths = vec![remote_path.clone()];
                if update.recursive {
                    paths = collect_sftp_recursive_paths(&session.sftp, &remote_path).await?;
                }
                for path in paths {
                    session
                        .sftp
                        .set_metadata(
                            path,
                            russh_sftp::protocol::FileAttributes {
                                permissions: mode,
                                uid,
                                gid,
                                ..russh_sftp::protocol::FileAttributes::empty()
                            },
                        )
                        .await?;
                }
                Ok(())
            }
            .await;
            close_sftp_session(session).await;
            result
        })
    }

    pub fn read_text_file(
        &self,
        remote_path: impl AsRef<str>,
        max_bytes: u64,
    ) -> anyhow::Result<SftpRemoteTextFile> {
        let remote_path = remote_path.as_ref().to_string();
        let config = self.config.clone();
        let multiplex = self.multiplex.clone();
        self.run_operation(async move {
            let session = open_sftp_session(&config, multiplex.as_ref()).await?;
            let result = async {
                let attrs = session.sftp.metadata(remote_path.clone()).await?;
                if attrs.file_type() == russh_sftp::protocol::FileType::Dir {
                    anyhow::bail!("Directories cannot be opened as text");
                }
                let size = attrs.size.unwrap_or(0);
                if size > max_bytes {
                    anyhow::bail!(
                        "File is too large to open as text ({size} bytes > {max_bytes} bytes)"
                    );
                }
                let mut file = session.sftp.open(remote_path.clone()).await?;
                let mut bytes = Vec::with_capacity(size as usize);
                file.read_to_end(&mut bytes).await?;
                ensure_remote_text_bytes(&bytes, max_bytes)?;
                let content = String::from_utf8(bytes)
                    .map_err(|_| anyhow::anyhow!("Only UTF-8 text files are supported"))?;
                Ok(SftpRemoteTextFile {
                    path: remote_path,
                    content,
                    size,
                    modified_at: u64::from(attrs.mtime.unwrap_or(0)),
                })
            }
            .await;
            close_sftp_session(session).await;
            result
        })
    }

    pub fn write_text_file(
        &self,
        remote_path: impl AsRef<str>,
        content: impl AsRef<str>,
        expected_modified_at: Option<u64>,
        expected_size: Option<u64>,
        force: bool,
    ) -> anyhow::Result<SftpWriteTextResult> {
        let remote_path = remote_path.as_ref().to_string();
        let content = content.as_ref().to_string();
        let config = self.config.clone();
        let multiplex = self.multiplex.clone();
        self.run_operation(async move {
            let session = open_sftp_session(&config, multiplex.as_ref()).await?;
            let result = async {
                if !force {
                    let attrs = session.sftp.metadata(remote_path.clone()).await?;
                    let current_modified_at = u64::from(attrs.mtime.unwrap_or(0));
                    let current_size = attrs.size.unwrap_or(0);
                    if expected_modified_at.is_some_and(|value| value != current_modified_at)
                        || expected_size.is_some_and(|value| value != current_size)
                    {
                        return Ok(SftpWriteTextResult::Conflict {
                            modified_at: current_modified_at,
                            size: current_size,
                        });
                    }
                }

                let mut file = session.sftp.create(remote_path.clone()).await?;
                file.write_all(content.as_bytes()).await?;
                file.flush().await?;
                drop(file);
                let attrs = session.sftp.metadata(remote_path).await?;
                Ok(SftpWriteTextResult::Saved {
                    modified_at: u64::from(attrs.mtime.unwrap_or(0)),
                    size: attrs.size.unwrap_or(content.len() as u64),
                })
            }
            .await;
            close_sftp_session(session).await;
            result
        })
    }

    pub fn download_file(
        &self,
        remote_path: impl AsRef<str>,
        local_path: impl Into<PathBuf>,
    ) -> anyhow::Result<SftpTransferSummary> {
        self.download_file_with_progress(remote_path, local_path, |_| {})
    }

    pub fn download_file_with_progress<F>(
        &self,
        remote_path: impl AsRef<str>,
        local_path: impl Into<PathBuf>,
        progress: F,
    ) -> anyhow::Result<SftpTransferSummary>
    where
        F: FnMut(SftpTransferProgress) + Send + 'static,
    {
        self.download_file_with_progress_and_control(
            remote_path,
            local_path,
            SftpTransferControl::default(),
            progress,
        )
    }

    pub fn download_file_with_progress_and_control<F>(
        &self,
        remote_path: impl AsRef<str>,
        local_path: impl Into<PathBuf>,
        control: SftpTransferControl,
        progress: F,
    ) -> anyhow::Result<SftpTransferSummary>
    where
        F: FnMut(SftpTransferProgress) + Send + 'static,
    {
        self.download_file_with_progress_and_control_options(
            remote_path,
            local_path,
            control,
            SftpTransferOptions::default(),
            progress,
        )
    }

    pub fn download_file_with_progress_and_control_options<F>(
        &self,
        remote_path: impl AsRef<str>,
        local_path: impl Into<PathBuf>,
        control: SftpTransferControl,
        options: SftpTransferOptions,
        mut progress: F,
    ) -> anyhow::Result<SftpTransferSummary>
    where
        F: FnMut(SftpTransferProgress) + Send + 'static,
    {
        let remote_path = remote_path.as_ref().to_string();
        let local_path = local_path.into();
        let config = self.config.clone();
        let multiplex = self.multiplex.clone();
        self.run_operation(async move {
            let mut last_error = None;
            for _attempt in 0..=options.max_retries() {
                control.check_cancelled()?;
                let result = async {
                    let session = open_sftp_session(&config, multiplex.as_ref()).await?;
                    let bytes = download_remote_file(
                        &session.sftp,
                        &remote_path,
                        &local_path,
                        &control,
                        &options,
                        &mut progress,
                    )
                    .await?;
                    close_sftp_session(session).await;
                    Ok(SftpTransferSummary {
                        remote_path: remote_path.clone(),
                        local_path: local_path.clone(),
                        bytes,
                        skipped: false,
                    })
                }
                .await;
                match result {
                    Ok(summary) => return Ok(summary),
                    Err(error) if is_sftp_transfer_cancelled(&error) => return Err(error),
                    Err(error) => last_error = Some(error),
                }
            }
            Err(last_sftp_retry_error(last_error))
        })
    }

    pub fn download_path_with_progress<F>(
        &self,
        remote_path: impl AsRef<str>,
        local_path: impl Into<PathBuf>,
        progress: F,
    ) -> anyhow::Result<SftpTransferSummary>
    where
        F: FnMut(SftpTransferProgress) + Send + 'static,
    {
        self.download_path_with_progress_and_control(
            remote_path,
            local_path,
            SftpTransferControl::default(),
            progress,
        )
    }

    pub fn download_path_with_progress_and_control<F>(
        &self,
        remote_path: impl AsRef<str>,
        local_path: impl Into<PathBuf>,
        control: SftpTransferControl,
        progress: F,
    ) -> anyhow::Result<SftpTransferSummary>
    where
        F: FnMut(SftpTransferProgress) + Send + 'static,
    {
        self.download_path_with_progress_options(
            remote_path,
            local_path,
            control,
            SftpDuplicatePolicy::Overwrite,
            progress,
        )
    }

    pub fn download_path_with_progress_options<F>(
        &self,
        remote_path: impl AsRef<str>,
        local_path: impl Into<PathBuf>,
        control: SftpTransferControl,
        duplicate_policy: SftpDuplicatePolicy,
        progress: F,
    ) -> anyhow::Result<SftpTransferSummary>
    where
        F: FnMut(SftpTransferProgress) + Send + 'static,
    {
        self.download_path_with_progress_options_and_resolver(
            remote_path,
            local_path,
            control,
            duplicate_policy,
            None,
            progress,
        )
    }

    pub fn download_path_with_progress_options_and_resolver<F>(
        &self,
        remote_path: impl AsRef<str>,
        local_path: impl Into<PathBuf>,
        control: SftpTransferControl,
        duplicate_policy: SftpDuplicatePolicy,
        duplicate_resolver: Option<Arc<dyn SftpDuplicateResolver>>,
        progress: F,
    ) -> anyhow::Result<SftpTransferSummary>
    where
        F: FnMut(SftpTransferProgress) + Send + 'static,
    {
        self.download_path_with_progress_options_and_resolver_options(
            remote_path,
            local_path,
            control,
            duplicate_policy,
            duplicate_resolver,
            SftpTransferOptions::default(),
            progress,
        )
    }

    pub fn download_path_with_progress_options_and_resolver_options<F>(
        &self,
        remote_path: impl AsRef<str>,
        local_path: impl Into<PathBuf>,
        control: SftpTransferControl,
        duplicate_policy: SftpDuplicatePolicy,
        duplicate_resolver: Option<Arc<dyn SftpDuplicateResolver>>,
        options: SftpTransferOptions,
        mut progress: F,
    ) -> anyhow::Result<SftpTransferSummary>
    where
        F: FnMut(SftpTransferProgress) + Send + 'static,
    {
        let remote_path = remote_path.as_ref().to_string();
        let local_path = local_path.into();
        let config = self.config.clone();
        let multiplex = self.multiplex.clone();
        self.run_operation(async move {
            let mut last_error = None;
            for _attempt in 0..=options.max_retries() {
                control.check_cancelled()?;
                let result = async {
                    let session = open_sftp_session(&config, multiplex.as_ref()).await?;
                    let sftp = &session.sftp;
                    control.wait_if_paused().await?;
                    let metadata = sftp.metadata(remote_path.clone()).await?;
                    let is_directory = metadata.file_type() == russh_sftp::protocol::FileType::Dir;
                    let Some(local_target) = resolve_local_download_target(
                        &remote_path,
                        &local_path,
                        is_directory,
                        duplicate_policy,
                        duplicate_resolver.as_deref(),
                    )?
                    else {
                        close_sftp_session(session).await;
                        return Ok(SftpTransferSummary {
                            remote_path: remote_path.clone(),
                            local_path: local_path.clone(),
                            bytes: 0,
                            skipped: true,
                        });
                    };
                    let bytes = if is_directory {
                        download_remote_directory(
                            sftp,
                            &remote_path,
                            &local_target,
                            &control,
                            duplicate_policy,
                            duplicate_resolver.as_deref(),
                            &options,
                            &mut progress,
                        )
                        .await?
                    } else {
                        download_remote_file(
                            sftp,
                            &remote_path,
                            &local_target,
                            &control,
                            &options,
                            &mut progress,
                        )
                        .await?
                    };
                    close_sftp_session(session).await;
                    Ok(SftpTransferSummary {
                        remote_path: remote_path.clone(),
                        local_path: local_target,
                        bytes,
                        skipped: false,
                    })
                }
                .await;
                match result {
                    Ok(summary) => return Ok(summary),
                    Err(error) if is_sftp_transfer_cancelled(&error) => return Err(error),
                    Err(error) => last_error = Some(error),
                }
            }
            Err(last_sftp_retry_error(last_error))
        })
    }

    pub fn upload_file(
        &self,
        local_path: impl Into<PathBuf>,
        remote_path: impl AsRef<str>,
    ) -> anyhow::Result<SftpTransferSummary> {
        self.upload_file_with_progress(local_path, remote_path, |_| {})
    }

    pub fn upload_file_with_progress<F>(
        &self,
        local_path: impl Into<PathBuf>,
        remote_path: impl AsRef<str>,
        progress: F,
    ) -> anyhow::Result<SftpTransferSummary>
    where
        F: FnMut(SftpTransferProgress) + Send + 'static,
    {
        self.upload_file_with_progress_and_control(
            local_path,
            remote_path,
            SftpTransferControl::default(),
            progress,
        )
    }

    pub fn upload_file_with_progress_and_control<F>(
        &self,
        local_path: impl Into<PathBuf>,
        remote_path: impl AsRef<str>,
        control: SftpTransferControl,
        progress: F,
    ) -> anyhow::Result<SftpTransferSummary>
    where
        F: FnMut(SftpTransferProgress) + Send + 'static,
    {
        self.upload_file_with_progress_and_control_options(
            local_path,
            remote_path,
            control,
            SftpTransferOptions::default(),
            progress,
        )
    }

    pub fn upload_file_with_progress_and_control_options<F>(
        &self,
        local_path: impl Into<PathBuf>,
        remote_path: impl AsRef<str>,
        control: SftpTransferControl,
        options: SftpTransferOptions,
        mut progress: F,
    ) -> anyhow::Result<SftpTransferSummary>
    where
        F: FnMut(SftpTransferProgress) + Send + 'static,
    {
        let local_path = local_path.into();
        let remote_path = remote_path.as_ref().to_string();
        let config = self.config.clone();
        let multiplex = self.multiplex.clone();
        self.run_operation(async move {
            let remote_path = resolve_remote_upload_target(&local_path, &remote_path)?;
            let mut last_error = None;
            for _attempt in 0..=options.max_retries() {
                control.check_cancelled()?;
                let result = async {
                    let session = open_sftp_session(&config, multiplex.as_ref()).await?;
                    let bytes = upload_local_file(
                        &session.sftp,
                        &local_path,
                        &remote_path,
                        &control,
                        &options,
                        &mut progress,
                    )
                    .await?;
                    close_sftp_session(session).await;
                    Ok(SftpTransferSummary {
                        remote_path: remote_path.clone(),
                        local_path: local_path.clone(),
                        bytes,
                        skipped: false,
                    })
                }
                .await;
                match result {
                    Ok(summary) => return Ok(summary),
                    Err(error) if is_sftp_transfer_cancelled(&error) => return Err(error),
                    Err(error) => last_error = Some(error),
                }
            }
            Err(last_sftp_retry_error(last_error))
        })
    }

    pub fn upload_path_with_progress<F>(
        &self,
        local_path: impl Into<PathBuf>,
        remote_path: impl AsRef<str>,
        progress: F,
    ) -> anyhow::Result<SftpTransferSummary>
    where
        F: FnMut(SftpTransferProgress) + Send + 'static,
    {
        self.upload_path_with_progress_and_control(
            local_path,
            remote_path,
            SftpTransferControl::default(),
            progress,
        )
    }

    pub fn upload_path_with_progress_and_control<F>(
        &self,
        local_path: impl Into<PathBuf>,
        remote_path: impl AsRef<str>,
        control: SftpTransferControl,
        progress: F,
    ) -> anyhow::Result<SftpTransferSummary>
    where
        F: FnMut(SftpTransferProgress) + Send + 'static,
    {
        self.upload_path_with_progress_options(
            local_path,
            remote_path,
            control,
            SftpDuplicatePolicy::Overwrite,
            progress,
        )
    }

    pub fn upload_path_with_progress_options<F>(
        &self,
        local_path: impl Into<PathBuf>,
        remote_path: impl AsRef<str>,
        control: SftpTransferControl,
        duplicate_policy: SftpDuplicatePolicy,
        progress: F,
    ) -> anyhow::Result<SftpTransferSummary>
    where
        F: FnMut(SftpTransferProgress) + Send + 'static,
    {
        self.upload_path_with_progress_options_and_resolver(
            local_path,
            remote_path,
            control,
            duplicate_policy,
            None,
            progress,
        )
    }

    pub fn upload_path_with_progress_options_and_resolver<F>(
        &self,
        local_path: impl Into<PathBuf>,
        remote_path: impl AsRef<str>,
        control: SftpTransferControl,
        duplicate_policy: SftpDuplicatePolicy,
        duplicate_resolver: Option<Arc<dyn SftpDuplicateResolver>>,
        progress: F,
    ) -> anyhow::Result<SftpTransferSummary>
    where
        F: FnMut(SftpTransferProgress) + Send + 'static,
    {
        self.upload_path_with_progress_options_and_resolver_options(
            local_path,
            remote_path,
            control,
            duplicate_policy,
            duplicate_resolver,
            SftpTransferOptions::default(),
            progress,
        )
    }

    pub fn upload_path_with_progress_options_and_resolver_options<F>(
        &self,
        local_path: impl Into<PathBuf>,
        remote_path: impl AsRef<str>,
        control: SftpTransferControl,
        duplicate_policy: SftpDuplicatePolicy,
        duplicate_resolver: Option<Arc<dyn SftpDuplicateResolver>>,
        options: SftpTransferOptions,
        mut progress: F,
    ) -> anyhow::Result<SftpTransferSummary>
    where
        F: FnMut(SftpTransferProgress) + Send + 'static,
    {
        let local_path = local_path.into();
        let remote_path = remote_path.as_ref().to_string();
        let config = self.config.clone();
        let multiplex = self.multiplex.clone();
        self.run_operation(async move {
            let metadata = tokio::fs::metadata(&local_path).await?;
            let remote_path = resolve_remote_upload_target(&local_path, &remote_path)?;
            let mut last_error = None;
            for _attempt in 0..=options.max_retries() {
                control.check_cancelled()?;
                let result = async {
                    let session = open_sftp_session(&config, multiplex.as_ref()).await?;
                    let sftp = &session.sftp;
                    control.wait_if_paused().await?;
                    let Some(remote_target) = resolve_remote_write_target(
                        sftp,
                        &local_path.display().to_string(),
                        &remote_path,
                        metadata.is_dir(),
                        duplicate_policy,
                        duplicate_resolver.as_deref(),
                    )
                    .await?
                    else {
                        close_sftp_session(session).await;
                        return Ok(SftpTransferSummary {
                            remote_path: remote_path.clone(),
                            local_path: local_path.clone(),
                            bytes: 0,
                            skipped: true,
                        });
                    };
                    let bytes = if metadata.is_dir() {
                        upload_local_directory(
                            sftp,
                            &local_path,
                            &remote_target,
                            &control,
                            duplicate_policy,
                            duplicate_resolver.as_deref(),
                            &options,
                            &mut progress,
                        )
                        .await?
                    } else {
                        upload_local_file(
                            sftp,
                            &local_path,
                            &remote_target,
                            &control,
                            &options,
                            &mut progress,
                        )
                        .await?
                    };
                    close_sftp_session(session).await;
                    Ok(SftpTransferSummary {
                        remote_path: remote_target,
                        local_path: local_path.clone(),
                        bytes,
                        skipped: false,
                    })
                }
                .await;
                match result {
                    Ok(summary) => return Ok(summary),
                    Err(error) if is_sftp_transfer_cancelled(&error) => return Err(error),
                    Err(error) => last_error = Some(error),
                }
            }
            Err(last_sftp_retry_error(last_error))
        })
    }
}

async fn collect_sftp_recursive_paths(
    sftp: &SftpSession,
    remote_path: &str,
) -> anyhow::Result<Vec<String>> {
    let mut paths = Vec::new();
    let mut stack = vec![remote_path.to_string()];
    while let Some(path) = stack.pop() {
        let metadata = sftp.symlink_metadata(path.clone()).await?;
        let is_directory = metadata.file_type() == russh_sftp::protocol::FileType::Dir;
        paths.push(path.clone());
        if is_directory {
            for entry in sftp.read_dir(path).await? {
                let name = entry.file_name();
                if name == "." || name == ".." {
                    continue;
                }
                stack.push(entry.path());
            }
        }
    }
    Ok(paths)
}

fn attrs_to_sftp_file_type(attrs: &russh_sftp::protocol::FileAttributes) -> SftpFileType {
    match attrs.file_type() {
        russh_sftp::protocol::FileType::File => SftpFileType::File,
        russh_sftp::protocol::FileType::Dir => SftpFileType::Directory,
        russh_sftp::protocol::FileType::Symlink => SftpFileType::Symlink,
        russh_sftp::protocol::FileType::Other => SftpFileType::Other,
    }
}

fn remote_file_name(path: &str) -> String {
    path.trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(path)
        .to_string()
}

fn format_sftp_permissions(file_type: SftpFileType, mode: u32) -> String {
    let mut output = String::with_capacity(10);
    output.push(match file_type {
        SftpFileType::Directory => 'd',
        SftpFileType::Symlink => 'l',
        _ => '-',
    });
    for shift in [6, 3, 0] {
        let bits = (mode >> shift) & 0o7;
        output.push(if bits & 0o4 != 0 { 'r' } else { '-' });
        output.push(if bits & 0o2 != 0 { 'w' } else { '-' });
        output.push(if bits & 0o1 != 0 { 'x' } else { '-' });
    }
    output
}

fn ensure_remote_text_bytes(bytes: &[u8], max_bytes: u64) -> anyhow::Result<()> {
    if bytes.len() as u64 > max_bytes {
        anyhow::bail!(
            "File is too large to open as text ({} bytes > {} bytes)",
            bytes.len(),
            max_bytes
        );
    }
    if bytes.contains(&0) {
        anyhow::bail!("Only text files can be opened in the internal editor");
    }
    Ok(())
}

fn resolve_remote_user_value(
    config: &SshSessionConfig,
    multiplex: Option<SshMultiplexHandle>,
    value: Option<&str>,
) -> anyhow::Result<Option<u32>> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if let Ok(uid) = value.parse::<u32>() {
        return Ok(Some(uid));
    }
    let output = run_remote_identity_command(
        config,
        multiplex,
        format!("id -u {}", shell_quote(value)),
        "resolve remote user",
    )?;
    Ok(Some(output.trim().parse()?))
}

fn resolve_remote_group_value(
    config: &SshSessionConfig,
    multiplex: Option<SshMultiplexHandle>,
    value: Option<&str>,
) -> anyhow::Result<Option<u32>> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if let Ok(gid) = value.parse::<u32>() {
        return Ok(Some(gid));
    }
    let output = run_remote_identity_command(
        config,
        multiplex,
        format!(
            "getent group {} | awk -F: 'NR==1 {{print $3}}'",
            shell_quote(value)
        ),
        "resolve remote group",
    )?;
    Ok(Some(output.trim().parse()?))
}

fn resolve_remote_user_name(
    config: &SshSessionConfig,
    multiplex: Option<SshMultiplexHandle>,
    uid: Option<u32>,
) -> Option<String> {
    let uid = uid?;
    run_remote_identity_command(
        config,
        multiplex,
        format!("getent passwd {uid} | awk -F: 'NR==1 {{print $1}}'"),
        "resolve remote uid",
    )
    .ok()
    .map(|value| value.trim().to_string())
    .filter(|value| !value.is_empty())
}

fn resolve_remote_group_name(
    config: &SshSessionConfig,
    multiplex: Option<SshMultiplexHandle>,
    gid: Option<u32>,
) -> Option<String> {
    let gid = gid?;
    run_remote_identity_command(
        config,
        multiplex,
        format!("getent group {gid} | awk -F: 'NR==1 {{print $1}}'"),
        "resolve remote gid",
    )
    .ok()
    .map(|value| value.trim().to_string())
    .filter(|value| !value.is_empty())
}

fn run_remote_identity_command(
    config: &SshSessionConfig,
    multiplex: Option<SshMultiplexHandle>,
    command: String,
    context: &'static str,
) -> anyhow::Result<String> {
    let service = match multiplex {
        Some(multiplex) => SshProcessService::with_multiplex(config.clone(), multiplex)?,
        None => SshProcessService::new(config.clone()),
    };
    let output = service.run_command(command, PROCESS_TIMEOUT)?;
    let status = output.exit_status.unwrap_or(1);
    if status != 0 {
        anyhow::bail!("{context} failed: {}", output.stderr.trim());
    }
    Ok(output.stdout)
}

fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}

async fn delete_remote_path_recursive(sftp: &SftpSession, remote_path: &str) -> anyhow::Result<()> {
    let metadata = match sftp.symlink_metadata(remote_path.to_string()).await {
        Ok(metadata) => metadata,
        Err(error) => {
            let message = error.to_string().to_ascii_lowercase();
            if message.contains("no such")
                || message.contains("not found")
                || message.contains("does not exist")
            {
                return Ok(());
            }
            return Err(error.into());
        }
    };

    match metadata.file_type() {
        russh_sftp::protocol::FileType::Dir => {
            let mut children = Vec::new();
            for entry in sftp.read_dir(remote_path.to_string()).await? {
                let name = entry.file_name();
                if name == "." || name == ".." {
                    continue;
                }
                children.push(entry.path());
            }
            for child in children {
                Box::pin(delete_remote_path_recursive(sftp, &child)).await?;
            }
            sftp.remove_dir(remote_path.to_string()).await?;
        }
        _ => {
            sftp.remove_file(remote_path.to_string()).await?;
        }
    }
    Ok(())
}

fn is_sftp_transfer_cancelled(error: &anyhow::Error) -> bool {
    error.to_string().contains(SFTP_TRANSFER_CANCELLED)
}

fn last_sftp_retry_error(last_error: Option<anyhow::Error>) -> anyhow::Error {
    last_error.unwrap_or_else(|| anyhow::anyhow!("SFTP transfer failed before starting"))
}

async fn apply_remote_default_file_mode(sftp: &SftpSession, remote_path: &str, mode: Option<u32>) {
    let Some(mode) = mode else {
        return;
    };
    let attrs = russh_sftp::protocol::FileAttributes {
        permissions: Some(mode),
        ..russh_sftp::protocol::FileAttributes::empty()
    };
    let _ = sftp.set_metadata(remote_path.to_string(), attrs).await;
}

fn preserve_local_modified_time(local_path: &Path, remote_mtime: Option<u32>) {
    let Some(remote_mtime) = remote_mtime.filter(|mtime| *mtime > 0) else {
        return;
    };
    let modified = UNIX_EPOCH + Duration::from_secs(u64::from(remote_mtime));
    if let Ok(file) = std::fs::File::open(local_path) {
        let _ = file.set_modified(modified);
    }
}

async fn preserve_remote_modified_time(
    sftp: &SftpSession,
    remote_path: &str,
    local_metadata: Option<std::fs::Metadata>,
) {
    let Some(local_metadata) = local_metadata else {
        return;
    };
    let Some(mtime) = local_metadata.modified().ok().and_then(sftp_timestamp_secs) else {
        return;
    };
    let atime = local_metadata
        .accessed()
        .ok()
        .and_then(sftp_timestamp_secs)
        .unwrap_or(mtime);
    let attrs = russh_sftp::protocol::FileAttributes {
        atime: Some(atime),
        mtime: Some(mtime),
        ..russh_sftp::protocol::FileAttributes::empty()
    };
    let _ = sftp.set_metadata(remote_path.to_string(), attrs).await;
}

fn sftp_timestamp_secs(time: SystemTime) -> Option<u32> {
    let seconds = time.duration_since(UNIX_EPOCH).ok()?.as_secs();
    Some(seconds.min(u64::from(u32::MAX)) as u32)
}

fn transfer_resume_offset(
    local_path: &Path,
    total_bytes: Option<u64>,
    options: &SftpTransferOptions,
) -> u64 {
    if !options.resume_broken_transfer {
        return 0;
    }
    let Some(total_bytes) = total_bytes.filter(|total| *total > 0) else {
        return 0;
    };
    let Ok(metadata) = std::fs::metadata(local_path) else {
        return 0;
    };
    if !metadata.is_file() {
        return 0;
    }
    let local_size = metadata.len();
    if local_size > 0 && local_size < total_bytes {
        local_size
    } else {
        0
    }
}

async fn download_remote_file<F>(
    sftp: &SftpSession,
    remote_path: &str,
    local_path: &Path,
    control: &SftpTransferControl,
    options: &SftpTransferOptions,
    progress: &mut F,
) -> anyhow::Result<u64>
where
    F: FnMut(SftpTransferProgress) + Send,
{
    control.wait_if_paused().await?;
    if let Some(parent) = local_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        tokio::fs::create_dir_all(parent).await?;
    }
    let mut remote = sftp.open(remote_path.to_string()).await?;
    let remote_metadata = remote.metadata().await?;
    let total_bytes = remote_metadata.size;
    let resume_offset = transfer_resume_offset(local_path, total_bytes, options);
    let mut local = if resume_offset > 0 {
        tokio::fs::OpenOptions::new()
            .append(true)
            .open(local_path)
            .await?
    } else {
        tokio::fs::File::create(local_path).await?
    };
    let mut buffer = vec![0_u8; options.buffer_size_bytes()];
    let mut bytes = resume_offset;
    progress(SftpTransferProgress {
        remote_path: remote_path.to_string(),
        local_path: local_path.to_path_buf(),
        bytes_transferred: bytes,
        total_bytes,
        item_count_completed: None,
        item_count_total: None,
    });
    loop {
        control.wait_if_paused().await?;
        let read = if resume_offset > 0 {
            let data = remote.read_at(bytes, buffer.len()).await?;
            let read = data.len();
            buffer[..read].copy_from_slice(&data);
            read
        } else {
            remote.read(&mut buffer).await?
        };
        if read == 0 {
            break;
        }
        local.write_all(&buffer[..read]).await?;
        control.wait_if_paused().await?;
        bytes += read as u64;
        progress(SftpTransferProgress {
            remote_path: remote_path.to_string(),
            local_path: local_path.to_path_buf(),
            bytes_transferred: bytes,
            total_bytes,
            item_count_completed: None,
            item_count_total: None,
        });
    }
    local.flush().await?;
    if options.preserve_timestamps {
        preserve_local_modified_time(local_path, remote_metadata.mtime);
    }
    remote.shutdown().await?;
    Ok(bytes)
}

async fn download_remote_directory<F>(
    sftp: &SftpSession,
    remote_path: &str,
    local_path: &Path,
    control: &SftpTransferControl,
    duplicate_policy: SftpDuplicatePolicy,
    duplicate_resolver: Option<&dyn SftpDuplicateResolver>,
    options: &SftpTransferOptions,
    progress: &mut F,
) -> anyhow::Result<u64>
where
    F: FnMut(SftpTransferProgress) + Send,
{
    control.wait_if_paused().await?;
    tokio::fs::create_dir_all(local_path).await?;
    let (expected_bytes, item_count_total) =
        remote_directory_transfer_totals(sftp, remote_path, control).await?;
    let mut total_bytes = 0_u64;
    let mut item_count_completed = 0_u64;
    let mut pending = vec![(remote_path.to_string(), local_path.to_path_buf())];
    while let Some((remote_dir, local_dir)) = pending.pop() {
        control.wait_if_paused().await?;
        tokio::fs::create_dir_all(&local_dir).await?;
        for entry in sftp.read_dir(remote_dir.clone()).await? {
            control.wait_if_paused().await?;
            let name = entry.file_name();
            if name == "." || name == ".." {
                continue;
            }
            let remote_child = remote_join(&remote_dir, &name);
            let local_child = local_dir.join(&name);
            match entry.file_type() {
                russh_sftp::protocol::FileType::Dir => {
                    if let Some(local_child) = resolve_local_download_target(
                        &remote_child,
                        &local_child,
                        true,
                        duplicate_policy,
                        duplicate_resolver,
                    )? {
                        pending.push((remote_child, local_child));
                    }
                }
                russh_sftp::protocol::FileType::File | russh_sftp::protocol::FileType::Symlink => {
                    if let Some(local_child) = resolve_local_download_target(
                        &remote_child,
                        &local_child,
                        false,
                        duplicate_policy,
                        duplicate_resolver,
                    )? {
                        let completed_bytes = total_bytes;
                        let mut aggregate_progress = |current| {
                            progress(directory_transfer_progress(
                                current,
                                completed_bytes,
                                expected_bytes,
                                item_count_completed,
                                item_count_total,
                            ));
                        };
                        total_bytes += download_remote_file(
                            sftp,
                            &remote_child,
                            &local_child,
                            control,
                            options,
                            &mut aggregate_progress,
                        )
                        .await?;
                    }
                    item_count_completed = item_count_completed.saturating_add(1);
                    progress(SftpTransferProgress {
                        remote_path: remote_child,
                        local_path: local_child,
                        bytes_transferred: total_bytes,
                        total_bytes: (expected_bytes > 0).then_some(expected_bytes),
                        item_count_completed: Some(item_count_completed.min(item_count_total)),
                        item_count_total: Some(item_count_total),
                    });
                }
                russh_sftp::protocol::FileType::Other => {}
            }
        }
    }
    Ok(total_bytes)
}

async fn upload_local_file<F>(
    sftp: &SftpSession,
    local_path: &Path,
    remote_path: &str,
    control: &SftpTransferControl,
    options: &SftpTransferOptions,
    progress: &mut F,
) -> anyhow::Result<u64>
where
    F: FnMut(SftpTransferProgress) + Send,
{
    control.wait_if_paused().await?;
    let mut local = tokio::fs::File::open(local_path).await?;
    let local_metadata = local.metadata().await.ok();
    let total_bytes = local_metadata.as_ref().map(|metadata| metadata.len());
    let mut remote = sftp.create(remote_path.to_string()).await?;
    let mut buffer = vec![0_u8; options.buffer_size_bytes()];
    let mut bytes = 0_u64;
    progress(SftpTransferProgress {
        remote_path: remote_path.to_string(),
        local_path: local_path.to_path_buf(),
        bytes_transferred: bytes,
        total_bytes,
        item_count_completed: None,
        item_count_total: None,
    });
    loop {
        control.wait_if_paused().await?;
        let read = local.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        remote.write_all(&buffer[..read]).await?;
        control.wait_if_paused().await?;
        bytes += read as u64;
        progress(SftpTransferProgress {
            remote_path: remote_path.to_string(),
            local_path: local_path.to_path_buf(),
            bytes_transferred: bytes,
            total_bytes,
            item_count_completed: None,
            item_count_total: None,
        });
    }
    remote.flush().await?;
    remote.shutdown().await?;
    apply_remote_default_file_mode(sftp, remote_path, options.default_file_mode).await;
    if options.preserve_timestamps {
        preserve_remote_modified_time(sftp, remote_path, local_metadata).await;
    }
    Ok(bytes)
}

async fn upload_local_directory<F>(
    sftp: &SftpSession,
    local_path: &Path,
    remote_path: &str,
    control: &SftpTransferControl,
    duplicate_policy: SftpDuplicatePolicy,
    duplicate_resolver: Option<&dyn SftpDuplicateResolver>,
    options: &SftpTransferOptions,
    progress: &mut F,
) -> anyhow::Result<u64>
where
    F: FnMut(SftpTransferProgress) + Send,
{
    control.wait_if_paused().await?;
    ensure_remote_dir(sftp, remote_path, control).await?;
    let (expected_bytes, item_count_total) =
        local_directory_transfer_totals(local_path, control).await?;
    let mut total_bytes = 0_u64;
    let mut item_count_completed = 0_u64;
    let mut pending = vec![(local_path.to_path_buf(), remote_path.to_string())];
    while let Some((local_dir, remote_dir)) = pending.pop() {
        control.wait_if_paused().await?;
        ensure_remote_dir(sftp, &remote_dir, control).await?;
        let mut entries = tokio::fs::read_dir(&local_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            control.wait_if_paused().await?;
            let local_child = entry.path();
            let file_type = entry.file_type().await?;
            let name = entry.file_name().to_string_lossy().to_string();
            let remote_child = remote_join(&remote_dir, &name);
            if file_type.is_dir() {
                if let Some(remote_child) = resolve_remote_write_target(
                    sftp,
                    &local_child.display().to_string(),
                    &remote_child,
                    true,
                    duplicate_policy,
                    duplicate_resolver,
                )
                .await?
                {
                    pending.push((local_child, remote_child));
                }
            } else if file_type.is_file() {
                if let Some(remote_child) = resolve_remote_write_target(
                    sftp,
                    &local_child.display().to_string(),
                    &remote_child,
                    false,
                    duplicate_policy,
                    duplicate_resolver,
                )
                .await?
                {
                    let completed_bytes = total_bytes;
                    let mut aggregate_progress = |current| {
                        progress(directory_transfer_progress(
                            current,
                            completed_bytes,
                            expected_bytes,
                            item_count_completed,
                            item_count_total,
                        ));
                    };
                    total_bytes += upload_local_file(
                        sftp,
                        &local_child,
                        &remote_child,
                        control,
                        options,
                        &mut aggregate_progress,
                    )
                    .await?;
                }
                item_count_completed = item_count_completed.saturating_add(1);
                progress(SftpTransferProgress {
                    remote_path: remote_child,
                    local_path: local_child,
                    bytes_transferred: total_bytes,
                    total_bytes: (expected_bytes > 0).then_some(expected_bytes),
                    item_count_completed: Some(item_count_completed.min(item_count_total)),
                    item_count_total: Some(item_count_total),
                });
            }
        }
    }
    Ok(total_bytes)
}

fn directory_transfer_progress(
    current: SftpTransferProgress,
    completed_bytes: u64,
    expected_bytes: u64,
    item_count_completed: u64,
    item_count_total: u64,
) -> SftpTransferProgress {
    SftpTransferProgress {
        remote_path: current.remote_path,
        local_path: current.local_path,
        bytes_transferred: completed_bytes.saturating_add(current.bytes_transferred),
        total_bytes: (expected_bytes > 0).then_some(expected_bytes),
        item_count_completed: Some(item_count_completed.min(item_count_total)),
        item_count_total: Some(item_count_total),
    }
}

async fn remote_directory_transfer_totals(
    sftp: &SftpSession,
    remote_path: &str,
    control: &SftpTransferControl,
) -> anyhow::Result<(u64, u64)> {
    let mut total_bytes = 0_u64;
    let mut total_items = 0_u64;
    let mut pending = vec![remote_path.to_string()];
    while let Some(remote_dir) = pending.pop() {
        control.wait_if_paused().await?;
        for entry in sftp.read_dir(remote_dir.clone()).await? {
            let name = entry.file_name();
            if name == "." || name == ".." {
                continue;
            }
            match entry.file_type() {
                russh_sftp::protocol::FileType::Dir => {
                    pending.push(remote_join(&remote_dir, &name));
                }
                russh_sftp::protocol::FileType::File | russh_sftp::protocol::FileType::Symlink => {
                    total_items = total_items.saturating_add(1);
                    total_bytes = total_bytes.saturating_add(entry.metadata().size.unwrap_or(0));
                }
                russh_sftp::protocol::FileType::Other => {}
            }
        }
    }
    Ok((total_bytes, total_items))
}

async fn local_directory_transfer_totals(
    local_path: &Path,
    control: &SftpTransferControl,
) -> anyhow::Result<(u64, u64)> {
    let mut total_bytes = 0_u64;
    let mut total_items = 0_u64;
    let mut pending = vec![local_path.to_path_buf()];
    while let Some(local_dir) = pending.pop() {
        control.wait_if_paused().await?;
        let mut entries = tokio::fs::read_dir(local_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let file_type = entry.file_type().await?;
            if file_type.is_dir() {
                pending.push(entry.path());
            } else if file_type.is_file() {
                total_items = total_items.saturating_add(1);
                total_bytes = total_bytes.saturating_add(entry.metadata().await?.len());
            }
        }
    }
    Ok((total_bytes, total_items))
}

async fn ensure_remote_dir(
    sftp: &SftpSession,
    remote_path: &str,
    control: &SftpTransferControl,
) -> anyhow::Result<()> {
    control.wait_if_paused().await?;
    if sftp.try_exists(remote_path.to_string()).await? {
        return Ok(());
    }
    control.wait_if_paused().await?;
    sftp.create_dir(remote_path.to_string()).await?;
    Ok(())
}

fn resolve_remote_upload_target(local_path: &Path, remote_path: &str) -> anyhow::Result<String> {
    if remote_path == "." || remote_path.ends_with('/') {
        Ok(remote_join(remote_path, &local_file_name(local_path)?))
    } else {
        Ok(remote_path.to_string())
    }
}

fn local_file_name(path: &Path) -> anyhow::Result<String> {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .filter(|name| !name.is_empty())
        .ok_or_else(|| anyhow::anyhow!("local path has no file name: {}", path.display()))
}

fn resolve_local_download_target(
    remote_path: &str,
    local_path: &Path,
    is_directory: bool,
    duplicate_policy: SftpDuplicatePolicy,
    duplicate_resolver: Option<&dyn SftpDuplicateResolver>,
) -> anyhow::Result<Option<PathBuf>> {
    if !local_path.exists() {
        return Ok(Some(local_path.to_path_buf()));
    }

    match resolve_duplicate_decision(
        SftpTransferDirection::Download,
        remote_path,
        &local_path.display().to_string(),
        is_directory,
        duplicate_policy,
        duplicate_resolver,
    )? {
        SftpDuplicateDecision::Overwrite => Ok(Some(local_path.to_path_buf())),
        SftpDuplicateDecision::Skip => Ok(None),
        SftpDuplicateDecision::Rename => resolve_renamed_local_target(local_path).map(Some),
    }
}

async fn resolve_remote_write_target(
    sftp: &SftpSession,
    local_path: &str,
    remote_path: &str,
    is_directory: bool,
    duplicate_policy: SftpDuplicatePolicy,
    duplicate_resolver: Option<&dyn SftpDuplicateResolver>,
) -> anyhow::Result<Option<String>> {
    if !sftp.try_exists(remote_path.to_string()).await? {
        return Ok(Some(remote_path.to_string()));
    }

    match resolve_duplicate_decision(
        SftpTransferDirection::Upload,
        local_path,
        remote_path,
        is_directory,
        duplicate_policy,
        duplicate_resolver,
    )? {
        SftpDuplicateDecision::Overwrite => Ok(Some(remote_path.to_string())),
        SftpDuplicateDecision::Skip => Ok(None),
        SftpDuplicateDecision::Rename => resolve_renamed_remote_target(sftp, remote_path)
            .await
            .map(Some),
    }
}

fn resolve_duplicate_decision(
    direction: SftpTransferDirection,
    source_path: &str,
    target_path: &str,
    is_directory: bool,
    duplicate_policy: SftpDuplicatePolicy,
    duplicate_resolver: Option<&dyn SftpDuplicateResolver>,
) -> anyhow::Result<SftpDuplicateDecision> {
    match duplicate_policy {
        SftpDuplicatePolicy::Overwrite => Ok(SftpDuplicateDecision::Overwrite),
        SftpDuplicatePolicy::Skip => Ok(SftpDuplicateDecision::Skip),
        SftpDuplicatePolicy::Rename => Ok(SftpDuplicateDecision::Rename),
        SftpDuplicatePolicy::Ask => {
            let resolver = duplicate_resolver.ok_or_else(|| {
                anyhow::anyhow!("SFTP duplicate policy is ask but no resolver is available")
            })?;
            resolver
                .resolve_duplicate(&SftpDuplicateRequest {
                    direction,
                    source_path: source_path.to_string(),
                    target_path: target_path.to_string(),
                    is_directory,
                })
                .map_err(anyhow::Error::msg)
        }
    }
}

fn resolve_renamed_local_target(local_path: &Path) -> anyhow::Result<PathBuf> {
    let stem = local_path
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .filter(|stem| !stem.is_empty())
        .unwrap_or_else(|| local_file_name(local_path).unwrap_or_else(|_| "download".to_string()));
    let extension = local_path
        .extension()
        .map(|extension| format!(".{}", extension.to_string_lossy()))
        .unwrap_or_default();
    let parent = local_path.parent().unwrap_or_else(|| Path::new("."));
    for index in 1..=999 {
        let candidate = parent.join(format!("{stem}({index}){extension}"));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    anyhow::bail!(
        "unable to find a non-conflicting local path for {}",
        local_path.display()
    )
}

async fn resolve_renamed_remote_target(
    sftp: &SftpSession,
    remote_path: &str,
) -> anyhow::Result<String> {
    for index in 1..=999 {
        let candidate = remote_conflict_candidate(remote_path, index);
        if !sftp.try_exists(candidate.clone()).await? {
            return Ok(candidate);
        }
    }
    anyhow::bail!("unable to find a non-conflicting remote path for {remote_path}")
}

fn remote_conflict_candidate(remote_path: &str, index: usize) -> String {
    let (parent, name) = remote_split_parent_name(remote_path);
    let (stem, extension) = match name.rsplit_once('.') {
        Some((stem, extension)) if !stem.is_empty() => (stem.to_string(), format!(".{extension}")),
        _ => (name, String::new()),
    };
    remote_join(&parent, &format!("{stem}({index}){extension}"))
}

fn remote_split_parent_name(remote_path: &str) -> (String, String) {
    let trimmed = remote_path.trim_end_matches('/');
    match trimmed.rsplit_once('/') {
        Some(("", name)) => ("/".to_string(), name.to_string()),
        Some((parent, name)) => (parent.to_string(), name.to_string()),
        None => (".".to_string(), trimmed.to_string()),
    }
}

fn remote_join(base: &str, child: &str) -> String {
    if base.is_empty() || base == "." {
        child.to_string()
    } else if base == "/" {
        format!("/{child}")
    } else if base.ends_with('/') {
        format!("{base}{child}")
    } else {
        format!("{base}/{child}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directory_progress_accumulates_file_bytes_and_preserves_item_counts() {
        let current = SftpTransferProgress {
            remote_path: "/remote/two.txt".to_string(),
            local_path: PathBuf::from("/local/two.txt"),
            bytes_transferred: 25,
            total_bytes: Some(100),
            item_count_completed: None,
            item_count_total: None,
        };

        let aggregate = directory_transfer_progress(current, 100, 400, 1, 4);

        assert_eq!(aggregate.bytes_transferred, 125);
        assert_eq!(aggregate.total_bytes, Some(400));
        assert_eq!(aggregate.item_count_completed, Some(1));
        assert_eq!(aggregate.item_count_total, Some(4));
        assert_eq!(aggregate.remote_path, "/remote/two.txt");
    }

    #[test]
    fn sftp_transfer_retry_helpers_detect_cancelled_errors() {
        assert!(is_sftp_transfer_cancelled(&anyhow::anyhow!(
            SFTP_TRANSFER_CANCELLED
        )));
        assert!(!is_sftp_transfer_cancelled(&anyhow::anyhow!(
            "permission denied"
        )));
    }

    #[test]
    fn sftp_timestamp_secs_handles_unix_bounds() {
        assert_eq!(sftp_timestamp_secs(UNIX_EPOCH), Some(0));
        assert_eq!(
            sftp_timestamp_secs(UNIX_EPOCH + Duration::from_secs(42)),
            Some(42)
        );
        assert_eq!(
            sftp_timestamp_secs(UNIX_EPOCH - Duration::from_secs(1)),
            None
        );
        assert_eq!(
            sftp_timestamp_secs(UNIX_EPOCH + Duration::from_secs(u64::from(u32::MAX) + 100)),
            Some(u32::MAX)
        );
    }

    #[test]
    fn transfer_resume_offset_requires_partial_local_file() {
        let dir =
            std::env::temp_dir().join(format!("nyaterm-resume-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let file = dir.join("partial.bin");
        let enabled = SftpTransferOptions::default().with_resume_broken_transfer(true);
        let disabled = SftpTransferOptions::default();

        assert_eq!(transfer_resume_offset(&file, Some(10), &enabled), 0);
        std::fs::write(&file, [1_u8, 2, 3, 4]).expect("partial");
        assert_eq!(transfer_resume_offset(&file, Some(10), &disabled), 0);
        assert_eq!(transfer_resume_offset(&file, None, &enabled), 0);
        assert_eq!(transfer_resume_offset(&file, Some(10), &enabled), 4);
        assert_eq!(transfer_resume_offset(&file, Some(4), &enabled), 0);
        assert_eq!(transfer_resume_offset(&file, Some(3), &enabled), 0);

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn remote_join_handles_common_sftp_paths() {
        assert_eq!(remote_join(".", "file.txt"), "file.txt");
        assert_eq!(remote_join("", "file.txt"), "file.txt");
        assert_eq!(remote_join("/", "file.txt"), "/file.txt");
        assert_eq!(remote_join("/opt", "file.txt"), "/opt/file.txt");
        assert_eq!(remote_join("/opt/", "file.txt"), "/opt/file.txt");
    }

    #[test]
    fn upload_target_uses_local_name_for_remote_directories() {
        let local = PathBuf::from("/tmp/archive.tar");
        assert_eq!(
            resolve_remote_upload_target(&local, ".").expect("target"),
            "archive.tar"
        );
        assert_eq!(
            resolve_remote_upload_target(&local, "/srv/").expect("target"),
            "/srv/archive.tar"
        );
        assert_eq!(
            resolve_remote_upload_target(&local, "/srv/custom.tar").expect("target"),
            "/srv/custom.tar"
        );
    }

    #[test]
    fn local_download_target_applies_skip_and_rename_policy() {
        let dir = std::env::temp_dir().join(format!("nyaterm-sftp-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let target = dir.join("archive.tar.gz");
        std::fs::write(&target, b"existing").expect("target");

        assert_eq!(
            resolve_local_download_target(
                "/remote/archive.tar.gz",
                &target,
                false,
                SftpDuplicatePolicy::Skip,
                None,
            )
            .expect("skip"),
            None
        );
        assert_eq!(
            resolve_local_download_target(
                "/remote/archive.tar.gz",
                &target,
                false,
                SftpDuplicatePolicy::Rename,
                None,
            )
            .expect("rename"),
            Some(dir.join("archive.tar(1).gz"))
        );

        std::fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn ask_duplicate_policy_requires_resolver() {
        let error = resolve_duplicate_decision(
            SftpTransferDirection::Download,
            "/remote/file",
            "/tmp/file",
            false,
            SftpDuplicatePolicy::Ask,
            None,
        )
        .expect_err("missing resolver");

        assert!(error.to_string().contains("no resolver"));
    }

    #[test]
    fn remote_conflict_candidates_preserve_parent_and_extension() {
        assert_eq!(
            remote_conflict_candidate("/srv/archive.tar.gz", 2),
            "/srv/archive.tar(2).gz"
        );
        assert_eq!(remote_conflict_candidate("file", 1), "file(1)");
        assert_eq!(remote_conflict_candidate("/file", 1), "/file(1)");
    }

    #[test]
    fn sftp_transfer_control_reports_standard_cancel_error() {
        let control = SftpTransferControl::new();
        assert!(!control.is_cancelled());
        assert!(!control.is_paused());
        control.check_cancelled().expect("not cancelled");

        control.pause();
        assert!(control.is_paused());
        control.resume();
        assert!(!control.is_paused());

        control.pause();
        control.cancel();
        assert!(control.is_cancelled());
        assert!(!control.is_paused());
        let error = control.check_cancelled().expect_err("cancelled");
        assert_eq!(error.to_string(), SFTP_TRANSFER_CANCELLED);
    }
}
