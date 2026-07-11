pub mod agent_capture;
pub mod ai;
pub mod cloud_sync;
pub mod command_search;
pub mod command_suggestion_suppression;
pub mod credential_autofill;
pub mod terminal_input_tracker;
pub mod terminal_file_drop;
pub mod credentials_crypto;
pub mod diagnostics;
pub mod models;
pub mod portable_snapshot;
pub mod runtime;
pub mod services;
pub mod storage;
pub mod translation;
pub mod updater;

pub use agent_capture::{
    AgentCaptureCancelResult, AgentCaptureProcessResult, AgentCapturedOutput,
    AgentOutputCaptureProcessor, build_agent_capture_command,
};
pub use ai::{
    AI_AUDIT_MAX_LOGS, AI_HISTORY_MAX_MESSAGES, AI_HISTORY_MAX_SESSIONS,
    AI_REQUEST_USER_AGENT_DEFAULT, AgentApprovalDecision, AgentCommandExecutionMode,
    AgentCommandRiskAssessment, AgentLlmResponse, AiAction, AiAuditFile, AiAuditLog,
    AiChatCompletion, AiChatRequest, AiChatStreamDelta, AiCommandCard, AiContext,
    AiCustomActionConfig, AiHistoryFile, AiMessage, AiMessageRole, AiMode, AiModelConfigItem,
    AiModelDiscovery, AiModelError, AiModelOutput, AiModelSource, AiProviderCredential,
    AiProviderKind, AiProviderProfile, AiRequestOptions, AiSession, AiSettings, AiToolCall,
    AiToolCallDelta, AppendAiAuditRequest, CommandObservation, ResolvedAiModel, RiskLevel,
    agent_response_action, agent_system_prompt, ai_model_id_for_credential,
    ai_model_id_for_provider, ai_settings_has_secret, anthropic_messages_url,
    assess_agent_command_risk, assess_local_command_risk, build_agent_prompt,
    build_anthropic_chat_request_body, build_anthropic_chat_request_body_with_stream,
    build_gemini_chat_request_body, build_observation_message,
    build_openai_compatible_chat_request_body,
    build_openai_compatible_chat_request_body_with_stream, build_prompt,
    decide_agent_command_execution, effective_ai_request_user_agent, extract_json_object,
    extract_text_from_assistant, gemini_generate_content_url, gemini_stream_generate_content_url,
    genai_model_name, infer_provider_kind_from_model_id, mask_ai_settings,
    merge_masked_ai_settings, merge_model_discoveries, normalize_ai_settings, now_rfc3339,
    openai_compatible_chat_completions_url, openai_compatible_models_url, parse_agent_model_output,
    parse_agent_tool_call, parse_anthropic_chat_response, parse_anthropic_stream_chunk,
    parse_gemini_chat_response, parse_gemini_stream_chunk, parse_model_output,
    parse_openai_compatible_chat_response, parse_openai_compatible_models_response,
    parse_openai_compatible_stream_chunk, parse_risk_level_label, redact_context,
    redact_sensitive_text, resolve_model_credential, resolve_request_model, risk_label,
    system_prompt, trim_ai_audit, trim_ai_history, trim_optional_to_option, trim_string_to_option,
    truncate_preview, uuid, validate_model_credential,
};
pub use cloud_sync::{
    AliyunDriveSyncSettings, CLOUD_SYNC_HISTORY_DOMAIN, CLOUD_SYNC_HISTORY_EVENT,
    CLOUD_SYNC_HISTORY_LIMIT, CloudConflictPreview, CloudSyncError, CloudSyncHistoryEntry,
    CloudSyncRemote, CloudSyncResult, CloudSyncSettings, CloudSyncState, CloudSyncStatus,
    GiteeSnippetHttpBackend, GiteeSnippetSyncSettings, GithubGistHttpBackend,
    GithubGistSyncSettings, LocalCloudSyncOptions, LocalDirectoryRemote, MASKED_SECRET_VALUE,
    OAuthDriveSyncSettings, RemoteSyncPointer, S3HttpMethod, S3SignedRequest, S3SyncSettings,
    SNIPPET_REMOTE_FILE_PREFIX, SNIPPET_REMOTE_FILE_SUFFIX, SnippetBlobBackend, SnippetHttpClient,
    SnippetHttpDocument, SnippetHttpFile, SnippetHttpMethod, SnippetHttpRequest,
    SnippetHttpResponse, SnippetRemote, WebdavSyncSettings, append_cloud_sync_history,
    build_s3_signed_request, decode_snippet_blob, drive_remote_segments, encode_snippet_blob,
    gitee_snippet_patch_body, github_gist_patch_body, github_gist_update_conflict_is_retryable,
    google_drive_query_literal, legacy_sync_snapshot_file, load_sync_pointer,
    load_sync_pointer_from_remote, mask_cloud_sync_settings, merge_masked_cloud_sync_settings,
    pull_local_snapshot, pull_snapshot_with_remote, push_local_snapshot, push_snapshot_with_remote,
    read_cloud_sync_history, remote_path, s3_payload_sha256, snippet_remote_filename,
    snippet_remote_path,
};
pub use command_search::{fuzzy_search_items, search_command_sources};
pub use command_suggestion_suppression::{
    command_starts_suggestion_suppressing_program, is_pager_search_or_command_input,
    is_pager_single_key_input,
};
pub use credential_autofill::{
    CredentialPromptKind, compile_prompt_regex, credential_matches_prompt,
    detect_credential_prompt_kind, extract_credential_prompt_text,
    find_matching_credentials, find_password_only_fallback_credentials,
    get_credential_prompt_pattern, is_default_password_prompt,
    strip_terminal_control_sequences, validate_prompt_regex,
};
pub use credentials_crypto::{CredentialCrypto, CredentialCryptoError};
pub use terminal_file_drop::{format_local_terminal_drop_input, quote_local_path, terminal_drop_overlay_copy};
pub use terminal_input_tracker::{
    InputSelectionRange, TerminalInputState, apply_terminal_input_data,
    build_move_input_cursor_data, byte_index_to_char, can_register_command_from_tracker,
    can_suggest_from_tracker, char_index_to_byte, delete_terminal_input_range,
    get_tracked_command, get_tracked_submission_command, resync_from_terminal_line,
    sanitize_terminal_command, strip_terminal_command_prompt,
};
pub use diagnostics::{
    DiagnosticsError, DiagnosticsExportInfo, DiagnosticsExportOptions, DiagnosticsRuntimeSnapshot,
    LOG_FILE_PREFIX, LOG_FILE_SUFFIX, export_diagnostics_archive,
};
pub use models::*;
pub use portable_snapshot::{
    PortableSnapshotError, PortableSnapshotKind, PortableSnapshotMeta, RawPortableSnapshot,
    decode_encrypted_raw_portable_snapshot, decode_raw_portable_snapshot,
    encode_encrypted_raw_portable_snapshot, encode_raw_portable_snapshot,
};
pub use runtime::{AppRuntime, RuntimeMode};
pub use services::{MigrationCapability, NativeServiceStatus, NativeServices};
pub use storage::{ConfigBackupInfo, ConnectionStore, KnownHostCheck, StorageError};
pub use translation::{
    AliSignature, TranslateResult, TranslationError, TranslationSettings, ali_content_sha256,
    ali_signature, ali_translate_body, ali_translate_lang, baidu_translate_lang,
    baidu_translate_signature, deepl_api_base_url, deepl_translate_lang, format_ali_timestamp,
    google_translate_lang, merge_masked_translation_settings, microsoft_translate_lang,
    normalize_translation_provider, parse_ali_translate_response, parse_baidu_translate_response,
    parse_deepl_translate_response, parse_google_translate_response,
    parse_microsoft_translate_response, parse_youdao_translate_response,
    translation_settings_has_secret, youdao_translate_lang, youdao_translate_signature,
    youdao_truncate_for_sign,
};
pub use updater::{NativeUpdateInfo, parse_github_latest_release};
