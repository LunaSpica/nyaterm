use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn sftp_transfer_options(&self) -> SftpTransferOptions {
        SftpTransferOptions::default()
            .with_buffer_size_bytes(self.settings.transfer_buffer_size as usize * 1024)
            .with_max_retries(self.settings.transfer_max_retries)
            .with_preserve_timestamps(self.settings.transfer_preserve_timestamps)
            .with_default_file_permissions(&self.settings.transfer_default_file_permissions)
            .with_resume_broken_transfer(self.settings.transfer_resume_broken_transfer)
    }
}
