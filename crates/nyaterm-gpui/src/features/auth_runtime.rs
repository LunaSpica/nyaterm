use super::*;

pub(in crate::ui::view) struct NativeHostKeyVerifier {
    pub(in crate::ui::view) config_dir: PathBuf,
    pub(in crate::ui::view) portable_key_path: Option<PathBuf>,
    pub(in crate::ui::view) policy: String,
    pub(in crate::ui::view) prompt_broker: Arc<HostKeyPromptBroker>,
}

impl SshHostKeyVerifier for NativeHostKeyVerifier {
    fn verify(&self, host_key: &SshHostKey) -> Result<SshHostKeyDecision, String> {
        let store = ConnectionStore::open_with_portable_key_path(
            &self.config_dir,
            self.portable_key_path.clone(),
        )
        .map_err(|error| error.to_string())?;
        let line = format!(
            "{} {} {}",
            host_key.host_identifier, host_key.key_type, host_key.key_base64
        );
        match store
            .check_known_host(
                &host_key.host_identifier,
                &host_key.key_type,
                &host_key.key_base64,
            )
            .map_err(|error| error.to_string())?
        {
            KnownHostCheck::Match => Ok(SshHostKeyDecision::Accept),
            KnownHostCheck::UnknownHost if self.policy == "strict" => {
                Ok(SshHostKeyDecision::Reject(format!(
                    "unknown SSH host key for {} ({})",
                    host_key.host_identifier, host_key.fingerprint
                )))
            }
            KnownHostCheck::UnknownHost if self.policy == "prompt" => {
                match self
                    .prompt_broker
                    .request_decision(host_key.clone(), HostKeyPromptIssue::Unknown)
                {
                    Ok(HostKeyPromptChoice::Accept) => {
                        store
                            .upsert_known_host(&line)
                            .map_err(|error| error.to_string())?;
                        Ok(SshHostKeyDecision::Accept)
                    }
                    Ok(HostKeyPromptChoice::Reject) => Ok(SshHostKeyDecision::Reject(format!(
                        "unknown SSH host key rejected for {} ({})",
                        host_key.host_identifier, host_key.fingerprint
                    ))),
                    Err(error) => Ok(SshHostKeyDecision::Reject(error)),
                }
            }
            KnownHostCheck::UnknownHost => {
                store
                    .upsert_known_host(&line)
                    .map_err(|error| error.to_string())?;
                Ok(SshHostKeyDecision::Accept)
            }
            KnownHostCheck::HostSeen if self.policy == "accept" => {
                store
                    .replace_known_host_for_host(&host_key.host_identifier, &line)
                    .map_err(|error| error.to_string())?;
                Ok(SshHostKeyDecision::Accept)
            }
            KnownHostCheck::HostSeen if self.policy == "prompt" => {
                match self
                    .prompt_broker
                    .request_decision(host_key.clone(), HostKeyPromptIssue::Changed)
                {
                    Ok(HostKeyPromptChoice::Accept) => {
                        store
                            .replace_known_host_for_host(&host_key.host_identifier, &line)
                            .map_err(|error| error.to_string())?;
                        Ok(SshHostKeyDecision::Accept)
                    }
                    Ok(HostKeyPromptChoice::Reject) => Ok(SshHostKeyDecision::Reject(format!(
                        "changed SSH host key rejected for {} ({})",
                        host_key.host_identifier, host_key.fingerprint
                    ))),
                    Err(error) => Ok(SshHostKeyDecision::Reject(error)),
                }
            }
            KnownHostCheck::HostSeen => Ok(SshHostKeyDecision::Reject(format!(
                "SSH host key changed for {} ({})",
                host_key.host_identifier, host_key.fingerprint
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TotpUseRecord {
    code: String,
    time_step: u64,
}

#[derive(Debug)]
pub(in crate::ui::view) struct NativeOtpProvider {
    config_dir: PathBuf,
    portable_key_path: Option<PathBuf>,
    used_totp_codes: Mutex<HashMap<String, TotpUseRecord>>,
}

impl NativeOtpProvider {
    pub(in crate::ui::view) fn new(
        config_dir: PathBuf,
        portable_key_path: Option<PathBuf>,
    ) -> Self {
        Self {
            config_dir,
            portable_key_path,
            used_totp_codes: Mutex::new(HashMap::new()),
        }
    }

    fn load_entry(&self, otp_id: &str) -> Result<Option<DecryptedOtpEntry>, String> {
        let store = ConnectionStore::open_with_portable_key_path(
            &self.config_dir,
            self.portable_key_path.clone(),
        )
        .map_err(|error| error.to_string())?;
        store
            .load_decrypted_otp_entry_by_id(otp_id)
            .map_err(|error| error.to_string())
    }

    fn generate_totp_code(&self, entry: &DecryptedOtpEntry, now: u64) -> Result<TotpCode, String> {
        let (algorithm, secret, digits) = otp_material(entry)?;
        let period = if entry.period > 0 { entry.period } else { 30 };
        let totp = nyaterm_otp::Totp::new(
            algorithm,
            entry.issuer.clone(),
            entry.username.clone(),
            digits,
            period,
            secret,
        );
        let raw = totp.generate_at(now);
        Ok(TotpCode {
            code: format!("{:0>width$}", raw, width = digits as usize),
            time_step: now / period,
            period,
        })
    }

    fn generate_hotp_code(&self, entry: &DecryptedOtpEntry) -> Result<String, String> {
        let (algorithm, secret, digits) = otp_material(entry)?;
        let mut hotp = nyaterm_otp::Hotp::new(
            algorithm,
            entry.issuer.clone(),
            entry.username.clone(),
            digits,
            entry.counter,
            secret,
        );
        let raw = hotp.generate();
        Ok(format!("{:0>width$}", raw, width = digits as usize))
    }

    fn increment_counter(&self, otp_id: &str) -> Result<(), String> {
        let store = ConnectionStore::open_with_portable_key_path(
            &self.config_dir,
            self.portable_key_path.clone(),
        )
        .map_err(|error| error.to_string())?;
        store
            .increment_otp_counter(otp_id)
            .map_err(|error| error.to_string())
    }

    fn has_used_totp_code(&self, otp_id: &str, candidate: &TotpCode) -> Result<bool, String> {
        let used = self
            .used_totp_codes
            .lock()
            .map_err(|_| "TOTP use cache is poisoned".to_string())?;
        Ok(used.get(otp_id).is_some_and(|record| {
            record.code == candidate.code && record.time_step == candidate.time_step
        }))
    }

    fn record_totp_code(&self, otp_id: &str, candidate: &TotpCode) -> Result<(), String> {
        let mut used = self
            .used_totp_codes
            .lock()
            .map_err(|_| "TOTP use cache is poisoned".to_string())?;
        used.insert(
            otp_id.to_string(),
            TotpUseRecord {
                code: candidate.code.clone(),
                time_step: candidate.time_step,
            },
        );
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct TotpCode {
    code: String,
    time_step: u64,
    period: u64,
}

impl SshOtpProvider for NativeOtpProvider {
    fn request_otp_code(&self, otp_id: &str) -> Result<Option<String>, String> {
        let Some(entry) = self.load_entry(otp_id)? else {
            return Ok(None);
        };
        if entry.otp_type == "hotp" {
            let code = self.generate_hotp_code(&entry)?;
            self.increment_counter(otp_id)?;
            return Ok(Some(code));
        }

        let mut now = unix_seconds_now();
        let mut code = self.generate_totp_code(&entry, now)?;
        if self.has_used_totp_code(otp_id, &code)? {
            let wait = seconds_until_next_totp_step(now, code.period);
            std::thread::sleep(Duration::from_secs(wait));
            now = unix_seconds_now();
            code = self.generate_totp_code(&entry, now)?;
        }
        self.record_totp_code(otp_id, &code)?;
        Ok(Some(code.code))
    }
}

fn otp_material(
    entry: &DecryptedOtpEntry,
) -> Result<(nyaterm_otp::Algorithm, nyaterm_otp::Secret, u8), String> {
    let secret = entry
        .secret
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("OTP entry '{}' has no secret", entry.id))?;
    let algorithm = match entry.algorithm.as_str() {
        "SHA256" => nyaterm_otp::Algorithm::SHA256,
        "SHA512" => nyaterm_otp::Algorithm::SHA512,
        _ => nyaterm_otp::Algorithm::SHA1,
    };
    let secret = nyaterm_otp::Secret::from_base32(secret)
        .map_err(|error| format!("invalid OTP secret for '{}': {error:?}", entry.id))?;
    let digits = if entry.digits > 0 { entry.digits } else { 6 };
    Ok((algorithm, secret, digits))
}

fn unix_seconds_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn seconds_until_next_totp_step(now: u64, period: u64) -> u64 {
    let period = period.max(1);
    let remaining = period - (now % period);
    remaining.max(1)
}

#[derive(Debug)]
pub(in crate::ui::view) struct SftpDuplicatePromptRequest {
    pub(in crate::ui::view) id: String,
    pub(in crate::ui::view) request: SftpDuplicateRequest,
    pub(in crate::ui::view) response_tx: mpsc::Sender<SftpDuplicateDecision>,
}

#[derive(Debug, Clone)]
pub(in crate::ui::view) struct SftpDuplicatePromptState {
    pub(in crate::ui::view) id: String,
    pub(in crate::ui::view) request: SftpDuplicateRequest,
    pub(in crate::ui::view) response_tx: mpsc::Sender<SftpDuplicateDecision>,
}

#[derive(Debug, Default)]
pub(in crate::ui::view) struct SftpDuplicatePromptBroker {
    pending: Mutex<VecDeque<SftpDuplicatePromptRequest>>,
}

impl SftpDuplicatePromptBroker {
    fn request_decision(
        &self,
        request: SftpDuplicateRequest,
    ) -> Result<SftpDuplicateDecision, String> {
        let (response_tx, response_rx) = mpsc::channel();
        let request = SftpDuplicatePromptRequest {
            id: sftp_duplicate_prompt_id(&request),
            request,
            response_tx,
        };
        self.pending
            .lock()
            .map_err(|_| "SFTP duplicate prompt queue is poisoned".to_string())?
            .push_back(request);

        response_rx
            .recv_timeout(Duration::from_secs(300))
            .map_err(|_| "SFTP duplicate prompt timed out".to_string())
    }

    pub(in crate::ui::view) fn pop_pending(&self) -> Option<SftpDuplicatePromptRequest> {
        self.pending
            .lock()
            .ok()
            .and_then(|mut pending| pending.pop_front())
    }
}

impl SftpDuplicateResolver for SftpDuplicatePromptBroker {
    fn resolve_duplicate(
        &self,
        request: &SftpDuplicateRequest,
    ) -> Result<SftpDuplicateDecision, String> {
        self.request_decision(request.clone())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::ui::view) enum HostKeyPromptIssue {
    Unknown,
    Changed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::ui::view) enum HostKeyPromptChoice {
    Accept,
    Reject,
}

#[derive(Debug, Clone)]
pub(in crate::ui::view) struct HostKeyPromptRequest {
    pub(in crate::ui::view) id: String,
    pub(in crate::ui::view) host_key: SshHostKey,
    pub(in crate::ui::view) issue: HostKeyPromptIssue,
    pub(in crate::ui::view) response_tx: mpsc::Sender<HostKeyPromptChoice>,
}

#[derive(Debug, Default)]
pub(in crate::ui::view) struct HostKeyPromptBroker {
    pending: Mutex<VecDeque<HostKeyPromptRequest>>,
}

impl HostKeyPromptBroker {
    fn request_decision(
        &self,
        host_key: SshHostKey,
        issue: HostKeyPromptIssue,
    ) -> Result<HostKeyPromptChoice, String> {
        let (response_tx, response_rx) = mpsc::channel();
        let request = HostKeyPromptRequest {
            id: uuid_like_prompt_id(&host_key),
            host_key,
            issue,
            response_tx,
        };
        self.pending
            .lock()
            .map_err(|_| "host-key prompt queue is poisoned".to_string())?
            .push_back(request);

        response_rx
            .recv_timeout(Duration::from_secs(300))
            .map_err(|_| "SSH host-key prompt timed out".to_string())
    }

    pub(in crate::ui::view) fn pop_pending(&self) -> Option<HostKeyPromptRequest> {
        self.pending
            .lock()
            .ok()
            .and_then(|mut pending| pending.pop_front())
    }
}

#[derive(Debug)]
pub(in crate::ui::view) struct CredentialPromptRequest {
    pub(in crate::ui::view) id: String,
    pub(in crate::ui::view) prompt: SshCredentialPrompt,
    pub(in crate::ui::view) response_tx: mpsc::Sender<Option<String>>,
}

#[derive(Debug, Clone)]
pub(in crate::ui::view) struct CredentialPromptState {
    pub(in crate::ui::view) id: String,
    pub(in crate::ui::view) prompt: SshCredentialPrompt,
    pub(in crate::ui::view) response_tx: mpsc::Sender<Option<String>>,
    pub(in crate::ui::view) value: String,
}

#[derive(Debug, Default)]
pub(in crate::ui::view) struct CredentialPromptBroker {
    pending: Mutex<VecDeque<CredentialPromptRequest>>,
}

impl CredentialPromptBroker {
    fn request_secret(&self, prompt: SshCredentialPrompt) -> Result<Option<String>, String> {
        let (response_tx, response_rx) = mpsc::channel();
        let request = CredentialPromptRequest {
            id: credential_prompt_id(&prompt),
            prompt,
            response_tx,
        };
        self.pending
            .lock()
            .map_err(|_| "credential prompt queue is poisoned".to_string())?
            .push_back(request);

        response_rx
            .recv_timeout(Duration::from_secs(300))
            .map_err(|_| "SSH credential prompt timed out".to_string())
    }

    pub(in crate::ui::view) fn pop_pending(&self) -> Option<CredentialPromptRequest> {
        self.pending
            .lock()
            .ok()
            .and_then(|mut pending| pending.pop_front())
    }
}

impl SshCredentialProvider for CredentialPromptBroker {
    fn request_secret(&self, prompt: &SshCredentialPrompt) -> Result<Option<String>, String> {
        CredentialPromptBroker::request_secret(self, prompt.clone())
    }
}
