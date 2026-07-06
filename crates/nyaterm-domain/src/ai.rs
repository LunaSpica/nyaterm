use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use regex::Regex;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::cloud_sync::MASKED_SECRET_VALUE;

pub const AI_REQUEST_USER_AGENT_DEFAULT: &str =
    "codex-tui/0.125.0 (Ubuntu 22.4.0; x86_64) xterm-256color (codex-tui; 0.125.0)";
pub const AI_HISTORY_MAX_SESSIONS: usize = 200;
pub const AI_HISTORY_MAX_MESSAGES: usize = 2_000;
pub const AI_AUDIT_MAX_LOGS: usize = 2_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AiProviderKind {
    Openai,
    Anthropic,
    Gemini,
    Deepseek,
    Groq,
    Ollama,
    Xai,
    Cohere,
    Mimo,
    Zai,
    OpenaiCompatible,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AiMode {
    Ask,
    Agent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl Default for RiskLevel {
    fn default() -> Self {
        Self::Medium
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentCommandExecutionMode {
    ConfirmEach,
    Smart,
    Auto,
}

impl Default for AgentCommandExecutionMode {
    fn default() -> Self {
        Self::ConfirmEach
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AiModelSource {
    RustGenai,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiProviderProfile {
    pub id: String,
    pub name: String,
    pub provider_kind: AiProviderKind,
    pub model: String,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiModelConfigItem {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub provider_kind: Option<AiProviderKind>,
    #[serde(default)]
    pub credential_id: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_model_source")]
    pub source: AiModelSource,
    #[serde(default)]
    pub last_seen_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiProviderCredential {
    pub id: String,
    pub name: String,
    pub provider_kind: AiProviderKind,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiCustomActionConfig {
    pub id: String,
    pub name: String,
    pub prompt: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiSettings {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_context_line_limit")]
    pub context_line_limit: u32,
    #[serde(default = "default_true")]
    pub redaction_enabled: bool,
    #[serde(default = "default_true")]
    pub allow_save_command: bool,
    #[serde(default = "default_true")]
    pub record_history: bool,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default = "default_request_user_agent")]
    pub request_user_agent: String,
    #[serde(default = "default_active_profile_id")]
    pub active_profile_id: String,
    #[serde(default = "default_provider_profiles")]
    pub provider_profiles: Vec<AiProviderProfile>,
    #[serde(default = "default_mode")]
    pub default_mode: AiMode,
    #[serde(default)]
    pub default_model_id: Option<String>,
    #[serde(default)]
    pub models: Vec<AiModelConfigItem>,
    #[serde(default)]
    pub provider_credentials: Vec<AiProviderCredential>,
    #[serde(default)]
    pub terminal_ai_actions: Vec<AiCustomActionConfig>,
    #[serde(default)]
    pub file_ai_actions: Vec<AiCustomActionConfig>,
    #[serde(default = "default_max_ai_file_size_bytes")]
    pub max_ai_file_size_bytes: u64,
    #[serde(default)]
    pub max_agent_steps: Option<u16>,
    #[serde(default)]
    pub agent_step_timeout_ms: Option<u64>,
    #[serde(default = "default_terminal_output_lines")]
    pub terminal_output_lines: u16,
    #[serde(default)]
    pub agent_background_execution_enabled: bool,
    #[serde(default)]
    pub agent_command_execution_mode: AgentCommandExecutionMode,
    #[serde(default = "default_agent_smart_auto_execute_max_risk")]
    pub agent_smart_auto_execute_max_risk: RiskLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiCommandCard {
    pub id: String,
    pub title: String,
    pub command: String,
    pub explanation: String,
    #[serde(default)]
    pub risk_level: Option<RiskLevel>,
    #[serde(default)]
    pub risk_reason: Option<String>,
    pub expected_effect: String,
    #[serde(default)]
    pub rollback: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub references: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AiMessageRole {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiSession {
    pub id: String,
    #[serde(default)]
    pub connection_id: Option<String>,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiMessage {
    pub id: String,
    pub session_id: String,
    pub role: AiMessageRole,
    pub content: String,
    pub created_at: String,
    #[serde(default)]
    pub reasoning_content: Option<String>,
    #[serde(default)]
    pub command_cards: Vec<AiCommandCard>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct AiHistoryFile {
    #[serde(default)]
    pub sessions: Vec<AiSession>,
    #[serde(default)]
    pub messages: Vec<AiMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiAuditLog {
    pub id: String,
    #[serde(default)]
    pub connection_id: Option<String>,
    pub action: String,
    #[serde(default)]
    pub user_input: Option<String>,
    #[serde(default)]
    pub generated_command: Option<String>,
    #[serde(default)]
    pub risk_level: Option<RiskLevel>,
    #[serde(default)]
    pub inserted_to_terminal: bool,
    #[serde(default)]
    pub executed: bool,
    #[serde(default)]
    pub blocked: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppendAiAuditRequest {
    #[serde(default)]
    pub connection_id: Option<String>,
    pub action: String,
    #[serde(default)]
    pub user_input: Option<String>,
    #[serde(default)]
    pub generated_command: Option<String>,
    #[serde(default)]
    pub risk_level: Option<RiskLevel>,
    #[serde(default)]
    pub inserted_to_terminal: bool,
    #[serde(default)]
    pub executed: bool,
    #[serde(default)]
    pub blocked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct AiAuditFile {
    #[serde(default)]
    pub logs: Vec<AiAuditLog>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiContext {
    #[serde(default)]
    pub connection_name: Option<String>,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub os: Option<String>,
    #[serde(default)]
    pub arch: Option<String>,
    #[serde(default)]
    pub recent_output: String,
    #[serde(default)]
    pub selected_text: String,
    #[serde(default)]
    pub input_buffer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AiAction {
    GenerateCommand,
    ExplainOutput,
    ExplainSelected,
    AnalyzeError,
    RepairFromSelection,
    CustomTerminalAction,
    CustomFileAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiRequestOptions {
    #[serde(default = "default_max_output_commands")]
    pub max_output_commands: u8,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default = "default_safety_mode")]
    pub safety_mode: String,
    #[serde(default = "default_history_turns")]
    pub history_turns: u16,
}

impl Default for AiRequestOptions {
    fn default() -> Self {
        Self {
            max_output_commands: default_max_output_commands(),
            language: default_language(),
            safety_mode: default_safety_mode(),
            history_turns: default_history_turns(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiChatRequest {
    #[serde(default)]
    pub stream_id: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub connection_id: Option<String>,
    #[serde(default)]
    pub terminal_session_id: Option<String>,
    #[serde(default = "default_mode")]
    pub mode: AiMode,
    #[serde(default)]
    pub model_id: Option<String>,
    #[serde(default)]
    pub model_name: Option<String>,
    pub action: AiAction,
    pub user_input: String,
    #[serde(default)]
    pub context: AiContext,
    #[serde(default)]
    pub options: AiRequestOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommandObservation {
    pub output: String,
    #[serde(default)]
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiModelOutput {
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub reasoning: Option<String>,
    #[serde(default)]
    pub command_cards: Vec<AiCommandCard>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedAiModel {
    pub model_name: String,
    pub provider_kind: AiProviderKind,
    pub credential: Option<AiProviderCredential>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiChatCompletion {
    pub text: String,
    pub reasoning_content: Option<String>,
    pub tool_calls: Vec<AiToolCall>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiToolCall {
    #[serde(default)]
    pub id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AiChatStreamDelta {
    pub text_delta: String,
    pub reasoning_delta: Option<String>,
    pub tool_call_deltas: Vec<AiToolCallDelta>,
    pub done: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AiToolCallDelta {
    pub index: usize,
    pub id_delta: Option<String>,
    pub name_delta: Option<String>,
    pub arguments_delta: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentLlmResponse {
    #[serde(default)]
    pub thought: String,
    pub action: String,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_risk_level")]
    pub risk_level: Option<RiskLevel>,
    #[serde(default)]
    pub risk_reason: Option<String>,
    #[serde(default)]
    pub answer: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct ExecuteCommandToolArgs {
    thought: String,
    command: String,
    #[serde(deserialize_with = "deserialize_required_risk_level")]
    risk_level: RiskLevel,
    risk_reason: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct FinalAnswerToolArgs {
    thought: String,
    answer: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentCommandRiskAssessment {
    pub model_risk: RiskLevel,
    pub local_risk: RiskLevel,
    pub effective_risk: RiskLevel,
    pub risk_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentApprovalDecision {
    Auto,
    NeedsApproval,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiModelDiscovery {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub provider_kind: Option<AiProviderKind>,
    #[serde(default)]
    pub credential_id: Option<String>,
    pub source: AiModelSource,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AiModelError {
    #[error("no enabled AI model configured")]
    NoEnabledModel,
    #[error("AI model '{model}' is missing provider information")]
    MissingProvider { model: String },
    #[error("no enabled AI credential found for model '{model}'")]
    MissingCredentialForModel { model: String },
    #[error("no enabled OpenAI-compatible AI credential configured")]
    MissingOpenAiCompatibleCredential,
    #[error("no enabled AI credential configured for {provider:?}")]
    MissingCredential { provider: AiProviderKind },
    #[error("no API key configured for AI credential '{credential}'")]
    MissingApiKey { credential: String },
    #[error("invalid AI base URL '{base_url}': {message}")]
    InvalidBaseUrl { base_url: String, message: String },
    #[error("invalid AI models JSON: {0}")]
    InvalidModelsJson(String),
    #[error("invalid AI chat JSON: {0}")]
    InvalidChatJson(String),
    #[error("AI chat response did not include assistant content")]
    MissingChatContent,
}

const SYSTEM_PROMPT_ZH: &str = r#"你是一个专业、谨慎、安全优先的 Linux / DevOps / 云原生终端助手。
你的任务是帮助用户解释终端输出、生成 Shell 命令、分析错误、提供排查步骤。

必须遵守：
1. 不要建议不可逆高危操作，除非明确说明风险和安全替代方案。
2. 默认生成只读诊断命令。
3. 对任何删除、格式化、重启、停服务、改权限、批量变更命令标记风险。
4. 命令必须适配用户当前系统、架构、shell 和权限上下文。
5. 输出必须结构化，包含命令、说明、风险等级、影响范围和回滚建议。
6. 不要编造当前系统不存在的信息；不确定时给出验证命令。
7. 不要要求用户粘贴密码、私钥、token。

只返回一个 JSON 对象，不要使用 Markdown 代码块。格式：
{
  "text": "给用户看的说明",
  "commandCards": [
    {
      "id": "cmd-uuid",
      "title": "标题",
      "command": "shell command",
      "explanation": "命令说明",
      "riskLevel": "low|medium|high|critical",
      "riskReason": "风险原因",
      "expectedEffect": "预计影响",
      "rollback": "回滚方式或无需回滚",
      "category": "Linux 性能"
    }
  ]
}"#;

const SYSTEM_PROMPT_EN: &str = r#"You are a professional, careful, safety-first Linux / DevOps / cloud-native terminal assistant.
Your job is to explain terminal output, generate Shell commands, analyze errors, and suggest next troubleshooting steps.

You must follow these rules:
1. Do not suggest irreversible high-risk actions unless you clearly explain the risk and provide safer alternatives.
2. Prefer read-only diagnostic commands by default.
3. Mark any delete, format, restart, stop-service, permission-change, or bulk-change command with the appropriate risk.
4. Commands must fit the user's current system, architecture, shell, and privilege context.
5. Output must be structured and include commands, explanations, risk level, expected effect, and rollback guidance.
6. Do not invent facts about the current system. If uncertain, provide verification commands.
7. Do not ask the user to paste passwords, private keys, or tokens.

Return exactly one JSON object and do not use Markdown code fences. Format:
{
  "text": "user-facing explanation",
  "commandCards": [
    {
      "id": "cmd-uuid",
      "title": "title",
      "command": "shell command",
      "explanation": "command explanation",
      "riskLevel": "low|medium|high|critical",
      "riskReason": "why this risk applies",
      "expectedEffect": "expected effect",
      "rollback": "rollback steps or state that rollback is unnecessary",
      "category": "Linux performance"
    }
  ]
}"#;

const AGENT_SYSTEM_PROMPT_ZH: &str = r#"你是一个终端自动化 Agent，通过"思考—执行—观察"循环完成用户的任务。

每一轮你只能做一件事：调用 execute_command 工具执行一条命令，或调用 final_answer 工具给出最终回答。

规则：
1. 每轮必须且只能调用一个工具，不要在普通正文里输出 JSON。
2. 如果需要执行命令，调用 execute_command。
3. 任务完成或无需执行命令时，调用 final_answer。
4. thought 和 answer 尽量使用用户请求指定的目标语言。
5. 优先使用只读命令收集信息，再做修改操作。
6. 不要执行不可逆高危命令（如 rm -rf /、mkfs、停止 SSH 等），改为在 thought 中说明风险并调用 final_answer。
7. 不要编造信息；不确定时先用验证命令确认。
8. 不要要求用户提供密码、私钥、token。
9. 命令必须适配用户当前的系统和 shell 环境。
10. riskLevel 规则：只读命令 -> low，普通写操作 -> medium，删除/重启/权限修改 -> high，不可逆破坏 -> critical。
11. 调用 execute_command 时必须同时提供 riskLevel 和 riskReason；riskReason 要简短说明为什么这样分级。"#;

const AGENT_SYSTEM_PROMPT_EN: &str = r#"You are a terminal automation agent that completes tasks using a think-execute-observe loop.

In each turn, do exactly one thing: call the execute_command tool to execute one command, or call the final_answer tool to finish.

Rules:
1. You must call exactly one tool per turn. Do not put protocol JSON in normal assistant text.
2. If a command must be executed, call execute_command.
3. If the task is complete or no command is needed, call final_answer.
4. Use the target language requested by the user for both thought and answer whenever possible.
5. Prefer read-only commands to gather information before making changes.
6. Do not execute irreversible high-risk commands (for example rm -rf /, mkfs, or stopping SSH). Explain the risk in thought and call final_answer instead.
7. Do not invent facts. If uncertain, verify first.
8. Do not ask the user for passwords, private keys, or tokens.
9. Commands must fit the user's current system and shell environment.
10. riskLevel guidance: read-only commands -> low, normal write actions -> medium, delete/restart/permission changes -> high, irreversible destructive actions -> critical.
11. execute_command calls must include both riskLevel and riskReason. Keep riskReason brief and explain why the risk applies."#;

impl Default for AiSettings {
    fn default() -> Self {
        let models = default_models();
        let default_model_id = models
            .iter()
            .find(|item| item.enabled)
            .map(|item| item.id.clone());

        Self {
            schema_version: default_schema_version(),
            enabled: true,
            context_line_limit: default_context_line_limit(),
            redaction_enabled: true,
            allow_save_command: true,
            record_history: true,
            timeout_ms: default_timeout_ms(),
            request_user_agent: default_request_user_agent(),
            active_profile_id: default_active_profile_id(),
            provider_profiles: default_provider_profiles(),
            default_mode: default_mode(),
            default_model_id,
            models,
            provider_credentials: default_provider_credentials(),
            terminal_ai_actions: default_terminal_ai_actions(),
            file_ai_actions: default_file_ai_actions(),
            max_ai_file_size_bytes: default_max_ai_file_size_bytes(),
            max_agent_steps: Some(10),
            agent_step_timeout_ms: Some(30_000),
            terminal_output_lines: default_terminal_output_lines(),
            agent_background_execution_enabled: false,
            agent_command_execution_mode: AgentCommandExecutionMode::ConfirmEach,
            agent_smart_auto_execute_max_risk: default_agent_smart_auto_execute_max_risk(),
        }
    }
}

pub fn ai_model_id_for_provider(kind: &AiProviderKind, name: &str) -> String {
    format!("{}:{name}", provider_kind_key(kind))
}

pub fn ai_model_id_for_credential(credential_id: &str, name: &str) -> String {
    format!("{credential_id}:{name}")
}

pub fn resolve_request_model(
    settings: &AiSettings,
    request: &AiChatRequest,
) -> Result<ResolvedAiModel, AiModelError> {
    let selected_model = request
        .model_id
        .as_deref()
        .and_then(|id| {
            settings
                .models
                .iter()
                .find(|model| model.enabled && model.id == id)
        })
        .or_else(|| {
            settings.default_model_id.as_deref().and_then(|id| {
                settings
                    .models
                    .iter()
                    .find(|model| model.enabled && model.id == id)
            })
        })
        .or_else(|| settings.models.iter().find(|model| model.enabled))
        .ok_or(AiModelError::NoEnabledModel)?;

    let model_provider_kind = selected_model
        .provider_kind
        .clone()
        .or_else(|| infer_provider_kind_from_model_id(&selected_model.id));

    let credential =
        resolve_model_credential(settings, selected_model, model_provider_kind.as_ref())?;
    let provider_kind = credential
        .as_ref()
        .map(|credential| credential.provider_kind.clone())
        .or(model_provider_kind)
        .ok_or_else(|| AiModelError::MissingProvider {
            model: selected_model.name.clone(),
        })?;
    validate_model_credential(&provider_kind, credential.as_ref())?;

    Ok(ResolvedAiModel {
        model_name: selected_model.name.clone(),
        provider_kind,
        credential,
    })
}

pub fn infer_provider_kind_from_model_id(model_id: &str) -> Option<AiProviderKind> {
    let (prefix, _) = model_id.split_once(':')?;
    match prefix {
        "openai" => Some(AiProviderKind::Openai),
        "anthropic" => Some(AiProviderKind::Anthropic),
        "gemini" => Some(AiProviderKind::Gemini),
        "deepseek" => Some(AiProviderKind::Deepseek),
        "groq" => Some(AiProviderKind::Groq),
        "ollama" => Some(AiProviderKind::Ollama),
        "xai" => Some(AiProviderKind::Xai),
        "cohere" => Some(AiProviderKind::Cohere),
        "mimo" => Some(AiProviderKind::Mimo),
        "zai" => Some(AiProviderKind::Zai),
        "openai_compatible" => Some(AiProviderKind::OpenaiCompatible),
        _ => None,
    }
}

pub fn resolve_model_credential(
    settings: &AiSettings,
    model: &AiModelConfigItem,
    provider_kind: Option<&AiProviderKind>,
) -> Result<Option<AiProviderCredential>, AiModelError> {
    if let Some(credential_id) = model.credential_id.as_deref() {
        let credential = settings
            .provider_credentials
            .iter()
            .find(|item| item.id == credential_id && item.enabled)
            .cloned()
            .ok_or_else(|| AiModelError::MissingCredentialForModel {
                model: model.name.clone(),
            })?;
        return Ok(Some(credential));
    }

    Ok(provider_kind.and_then(|provider_kind| {
        settings
            .provider_credentials
            .iter()
            .find(|item| item.enabled && &item.provider_kind == provider_kind)
            .cloned()
    }))
}

pub fn validate_model_credential(
    provider_kind: &AiProviderKind,
    credential: Option<&AiProviderCredential>,
) -> Result<(), AiModelError> {
    match provider_kind {
        AiProviderKind::Ollama => Ok(()),
        AiProviderKind::OpenaiCompatible => {
            if credential.is_none() {
                return Err(AiModelError::MissingOpenAiCompatibleCredential);
            }
            Ok(())
        }
        _ => {
            let credential = credential.ok_or_else(|| AiModelError::MissingCredential {
                provider: provider_kind.clone(),
            })?;
            if credential
                .api_key
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
            {
                return Err(AiModelError::MissingApiKey {
                    credential: credential.name.clone(),
                });
            }
            Ok(())
        }
    }
}

pub fn genai_model_name(provider_kind: &AiProviderKind, model_name: &str) -> String {
    if matches!(provider_kind, AiProviderKind::Deepseek)
        && let Some(base_model_name) = model_name.strip_suffix("-none")
    {
        return base_model_name.to_string();
    }

    model_name.to_string()
}

pub fn effective_ai_request_user_agent(settings: &AiSettings) -> &str {
    let value = settings.request_user_agent.trim();
    if value.is_empty() {
        AI_REQUEST_USER_AGENT_DEFAULT
    } else {
        value
    }
}

pub fn openai_compatible_models_url(base_url: &str) -> Result<String, AiModelError> {
    join_api_base_url(base_url, "models")
}

pub fn openai_compatible_chat_completions_url(base_url: &str) -> Result<String, AiModelError> {
    join_api_base_url(base_url, "chat/completions")
}

pub fn anthropic_messages_url(base_url: &str) -> Result<String, AiModelError> {
    join_api_base_url(base_url, "messages")
}

pub fn gemini_generate_content_url(base_url: &str, model: &str) -> Result<String, AiModelError> {
    let model = model.trim();
    if model.is_empty() {
        return Err(AiModelError::InvalidBaseUrl {
            base_url: base_url.to_string(),
            message: "Gemini model name is empty".to_string(),
        });
    }
    join_api_base_url(base_url, &format!("models/{model}:generateContent"))
}

pub fn gemini_stream_generate_content_url(
    base_url: &str,
    model: &str,
) -> Result<String, AiModelError> {
    let model = model.trim();
    if model.is_empty() {
        return Err(AiModelError::InvalidBaseUrl {
            base_url: base_url.to_string(),
            message: "Gemini model name is empty".to_string(),
        });
    }
    join_api_base_url(
        base_url,
        &format!("models/{model}:streamGenerateContent?alt=sse"),
    )
}

pub fn parse_openai_compatible_models_response(
    body: &str,
    credential: &AiProviderCredential,
) -> Result<Vec<AiModelDiscovery>, AiModelError> {
    let value: serde_json::Value = serde_json::from_str(body)
        .map_err(|error| AiModelError::InvalidModelsJson(error.to_string()))?;
    let mut models = Vec::new();
    if let Some(items) = value.get("data").and_then(serde_json::Value::as_array) {
        for item in items {
            let Some(name) = item
                .get("id")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|name| !name.is_empty())
            else {
                continue;
            };
            models.push(AiModelDiscovery {
                id: ai_model_id_for_credential(&credential.id, name),
                name: name.to_string(),
                provider_kind: Some(credential.provider_kind.clone()),
                credential_id: Some(credential.id.clone()),
                source: AiModelSource::RustGenai,
            });
        }
    }
    Ok(models)
}

pub fn build_openai_compatible_chat_request_body(
    resolved_model: &ResolvedAiModel,
    request: &AiChatRequest,
    settings: &AiSettings,
    history: &[AiMessage],
) -> serde_json::Value {
    build_openai_compatible_chat_request_body_with_stream(
        resolved_model,
        request,
        settings,
        history,
        false,
    )
}

pub fn build_openai_compatible_chat_request_body_with_stream(
    resolved_model: &ResolvedAiModel,
    request: &AiChatRequest,
    settings: &AiSettings,
    history: &[AiMessage],
    stream: bool,
) -> serde_json::Value {
    let mut messages = Vec::new();
    messages.push(serde_json::json!({
        "role": "system",
        "content": request_system_prompt(request),
    }));

    if let Some(session_id) = request.session_id.as_deref() {
        let max_turns = request.options.history_turns as usize;
        if max_turns > 0 {
            let session_messages = history
                .iter()
                .filter(|message| message.session_id == session_id)
                .collect::<Vec<_>>();
            let skip = session_messages.len().saturating_sub(max_turns);
            for message in session_messages.into_iter().skip(skip) {
                match message.role {
                    AiMessageRole::User => {
                        messages.push(serde_json::json!({
                            "role": "user",
                            "content": message.content,
                        }));
                    }
                    AiMessageRole::Assistant => {
                        let content = extract_text_from_assistant(&message.content);
                        if !content.is_empty() {
                            messages.push(serde_json::json!({
                                "role": "assistant",
                                "content": content,
                            }));
                        }
                    }
                    AiMessageRole::System => {}
                }
            }
        }
    }

    messages.push(serde_json::json!({
        "role": "user",
        "content": request_user_prompt(request, settings),
    }));

    let mut body = serde_json::json!({
        "model": genai_model_name(&resolved_model.provider_kind, &resolved_model.model_name),
        "messages": messages,
        "stream": stream,
    });
    if request.mode == AiMode::Agent {
        body["tools"] = agent_openai_tools();
        body["tool_choice"] = serde_json::json!("required");
    }
    body
}

pub fn build_anthropic_chat_request_body(
    resolved_model: &ResolvedAiModel,
    request: &AiChatRequest,
    settings: &AiSettings,
    history: &[AiMessage],
) -> serde_json::Value {
    build_anthropic_chat_request_body_with_stream(resolved_model, request, settings, history, false)
}

pub fn build_anthropic_chat_request_body_with_stream(
    resolved_model: &ResolvedAiModel,
    request: &AiChatRequest,
    settings: &AiSettings,
    history: &[AiMessage],
    stream: bool,
) -> serde_json::Value {
    let messages = chat_history_for_request(request, settings, history, "assistant");
    let mut body = serde_json::json!({
        "model": genai_model_name(&resolved_model.provider_kind, &resolved_model.model_name),
        "system": request_system_prompt(request),
        "max_tokens": 4096,
        "messages": messages,
        "stream": stream,
    });
    if request.mode == AiMode::Agent {
        body["tools"] = agent_anthropic_tools();
        body["tool_choice"] = serde_json::json!({ "type": "any" });
    }
    body
}

pub fn build_gemini_chat_request_body(
    request: &AiChatRequest,
    settings: &AiSettings,
    history: &[AiMessage],
) -> serde_json::Value {
    let contents = chat_history_for_request(request, settings, history, "model")
        .into_iter()
        .map(|message| {
            serde_json::json!({
                "role": message["role"].clone(),
                "parts": [{
                    "text": message["content"].clone(),
                }],
            })
        })
        .collect::<Vec<_>>();

    let mut body = serde_json::json!({
        "systemInstruction": {
                "parts": [{
                    "text": request_system_prompt(request),
                }],
            },
        "contents": contents,
        "generationConfig": {
            "temperature": 0,
        },
    });
    if request.mode == AiMode::Agent {
        body["tools"] = agent_gemini_tools();
        body["toolConfig"] = serde_json::json!({
            "functionCallingConfig": {
                "mode": "ANY",
                "allowedFunctionNames": ["execute_command", "final_answer"],
            }
        });
    }
    body
}

pub fn parse_openai_compatible_chat_response(body: &str) -> Result<AiChatCompletion, AiModelError> {
    let value: serde_json::Value = serde_json::from_str(body)
        .map_err(|error| AiModelError::InvalidChatJson(error.to_string()))?;
    let Some(message) = value
        .get("choices")
        .and_then(serde_json::Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
    else {
        return Err(AiModelError::MissingChatContent);
    };
    let tool_calls = extract_openai_compatible_tool_calls(message)?;
    let text = extract_openai_compatible_message_content(message).unwrap_or_default();
    if text.is_empty() && tool_calls.is_empty() {
        return Err(AiModelError::MissingChatContent);
    }
    let reasoning_content = ["reasoning_content", "reasoningContent", "reasoning"]
        .iter()
        .find_map(|key| {
            message
                .get(key)
                .and_then(serde_json::Value::as_str)
                .and_then(|value| trim_string_to_option(value.to_string()))
        });
    Ok(AiChatCompletion {
        text,
        reasoning_content,
        tool_calls,
    })
}

pub fn parse_openai_compatible_stream_chunk(
    chunk: &str,
) -> Result<Vec<AiChatStreamDelta>, AiModelError> {
    let mut deltas = Vec::new();
    for data in sse_data_payloads(chunk) {
        if data.trim() == "[DONE]" {
            deltas.push(AiChatStreamDelta {
                done: true,
                ..Default::default()
            });
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(&data)
            .map_err(|error| AiModelError::InvalidChatJson(error.to_string()))?;
        let Some(delta) = value
            .get("choices")
            .and_then(serde_json::Value::as_array)
            .and_then(|choices| choices.first())
            .and_then(|choice| choice.get("delta"))
        else {
            continue;
        };
        let text_delta = extract_openai_compatible_stream_content(delta);
        let tool_call_deltas = extract_openai_compatible_stream_tool_call_deltas(delta);
        let reasoning_delta = ["reasoning_content", "reasoningContent", "reasoning"]
            .iter()
            .find_map(|key| {
                delta
                    .get(key)
                    .and_then(serde_json::Value::as_str)
                    .map(ToOwned::to_owned)
            })
            .filter(|value| !value.is_empty());
        if !text_delta.is_empty() || reasoning_delta.is_some() || !tool_call_deltas.is_empty() {
            deltas.push(AiChatStreamDelta {
                text_delta,
                reasoning_delta,
                tool_call_deltas,
                done: false,
            });
        }
    }
    Ok(deltas)
}

pub fn parse_anthropic_stream_chunk(chunk: &str) -> Result<Vec<AiChatStreamDelta>, AiModelError> {
    let mut deltas = Vec::new();
    for data in sse_data_payloads(chunk) {
        let value: serde_json::Value = serde_json::from_str(&data)
            .map_err(|error| AiModelError::InvalidChatJson(error.to_string()))?;
        if value
            .get("type")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|event_type| event_type == "message_stop")
        {
            deltas.push(AiChatStreamDelta {
                done: true,
                ..Default::default()
            });
            continue;
        }

        let delta = value.get("delta");
        let content_block = value.get("content_block");
        let tool_call_deltas = extract_anthropic_stream_tool_call_deltas(&value);
        let text_delta = delta
            .and_then(|delta| delta.get("text"))
            .and_then(serde_json::Value::as_str)
            .or_else(|| {
                content_block
                    .and_then(|block| block.get("text"))
                    .and_then(serde_json::Value::as_str)
            })
            .unwrap_or_default()
            .to_string();
        let reasoning_delta = delta
            .and_then(|delta| delta.get("thinking"))
            .and_then(serde_json::Value::as_str)
            .or_else(|| {
                content_block
                    .and_then(|block| block.get("thinking"))
                    .and_then(serde_json::Value::as_str)
            })
            .map(ToOwned::to_owned)
            .filter(|value| !value.is_empty());
        if !text_delta.is_empty() || reasoning_delta.is_some() || !tool_call_deltas.is_empty() {
            deltas.push(AiChatStreamDelta {
                text_delta,
                reasoning_delta,
                tool_call_deltas,
                done: false,
            });
        }
    }
    Ok(deltas)
}

pub fn parse_gemini_stream_chunk(chunk: &str) -> Result<Vec<AiChatStreamDelta>, AiModelError> {
    let mut deltas = Vec::new();
    for data in sse_data_payloads(chunk) {
        if data.trim() == "[DONE]" {
            deltas.push(AiChatStreamDelta {
                done: true,
                ..Default::default()
            });
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(&data)
            .map_err(|error| AiModelError::InvalidChatJson(error.to_string()))?;
        let Some(parts) = value
            .get("candidates")
            .and_then(serde_json::Value::as_array)
            .and_then(|candidates| candidates.first())
            .and_then(|candidate| candidate.get("content"))
            .and_then(|content| content.get("parts"))
            .and_then(serde_json::Value::as_array)
        else {
            continue;
        };

        let mut text_delta = String::new();
        let mut reasoning_delta = String::new();
        let mut tool_call_deltas = Vec::new();
        for part in parts {
            if let Some(function_call) = part.get("functionCall") {
                let name_delta = function_call
                    .get("name")
                    .and_then(serde_json::Value::as_str)
                    .map(ToOwned::to_owned)
                    .filter(|value| !value.is_empty());
                let arguments_delta = function_call
                    .get("args")
                    .map(serde_json::Value::to_string)
                    .unwrap_or_default();
                if name_delta.is_some() || !arguments_delta.is_empty() {
                    tool_call_deltas.push(AiToolCallDelta {
                        index: tool_call_deltas.len(),
                        id_delta: None,
                        name_delta,
                        arguments_delta,
                    });
                }
                continue;
            }
            let Some(text) = part.get("text").and_then(serde_json::Value::as_str) else {
                continue;
            };
            if part
                .get("thought")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or_default()
            {
                reasoning_delta.push_str(text);
            } else {
                text_delta.push_str(text);
            }
        }
        if !text_delta.is_empty() || !reasoning_delta.is_empty() || !tool_call_deltas.is_empty() {
            deltas.push(AiChatStreamDelta {
                text_delta,
                reasoning_delta: trim_string_to_option(reasoning_delta),
                tool_call_deltas,
                done: false,
            });
        }
    }
    Ok(deltas)
}

pub fn parse_anthropic_chat_response(body: &str) -> Result<AiChatCompletion, AiModelError> {
    let value: serde_json::Value = serde_json::from_str(body)
        .map_err(|error| AiModelError::InvalidChatJson(error.to_string()))?;
    let Some(parts) = value.get("content").and_then(serde_json::Value::as_array) else {
        return Err(AiModelError::MissingChatContent);
    };

    let mut text_parts = Vec::new();
    let mut reasoning_parts = Vec::new();
    let mut tool_calls = Vec::new();
    for part in parts {
        let part_type = part
            .get("type")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        match part_type {
            "text" => {
                if let Some(text) = part.get("text").and_then(serde_json::Value::as_str) {
                    text_parts.push(text);
                }
            }
            "thinking" | "redacted_thinking" => {
                if let Some(reasoning) = part
                    .get("thinking")
                    .and_then(serde_json::Value::as_str)
                    .or_else(|| part.get("text").and_then(serde_json::Value::as_str))
                {
                    reasoning_parts.push(reasoning);
                }
            }
            "tool_use" => {
                let name = part
                    .get("name")
                    .and_then(serde_json::Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| {
                        AiModelError::InvalidChatJson(
                            "Anthropic tool_use is missing name".to_string(),
                        )
                    })?
                    .to_string();
                tool_calls.push(AiToolCall {
                    id: part
                        .get("id")
                        .and_then(serde_json::Value::as_str)
                        .map(ToOwned::to_owned),
                    name,
                    arguments: part
                        .get("input")
                        .cloned()
                        .unwrap_or_else(|| serde_json::Value::Object(Default::default())),
                });
            }
            _ => {}
        }
    }

    let text = trim_string_to_option(text_parts.join("")).unwrap_or_default();
    if text.is_empty() && tool_calls.is_empty() {
        return Err(AiModelError::MissingChatContent);
    }
    Ok(AiChatCompletion {
        text,
        reasoning_content: trim_string_to_option(reasoning_parts.join("\n\n")),
        tool_calls,
    })
}

pub fn parse_gemini_chat_response(body: &str) -> Result<AiChatCompletion, AiModelError> {
    let value: serde_json::Value = serde_json::from_str(body)
        .map_err(|error| AiModelError::InvalidChatJson(error.to_string()))?;
    let Some(parts) = value
        .get("candidates")
        .and_then(serde_json::Value::as_array)
        .and_then(|candidates| candidates.first())
        .and_then(|candidate| candidate.get("content"))
        .and_then(|content| content.get("parts"))
        .and_then(serde_json::Value::as_array)
    else {
        return Err(AiModelError::MissingChatContent);
    };

    let mut text_parts = Vec::new();
    let mut reasoning_parts = Vec::new();
    let mut tool_calls = Vec::new();
    for part in parts {
        if let Some(function_call) = part.get("functionCall") {
            let name = function_call
                .get("name")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    AiModelError::InvalidChatJson("Gemini functionCall is missing name".to_string())
                })?
                .to_string();
            tool_calls.push(AiToolCall {
                id: None,
                name,
                arguments: function_call
                    .get("args")
                    .cloned()
                    .unwrap_or_else(|| serde_json::Value::Object(Default::default())),
            });
            continue;
        }
        if part
            .get("thought")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or_default()
        {
            if let Some(text) = part.get("text").and_then(serde_json::Value::as_str) {
                reasoning_parts.push(text);
            }
            continue;
        }
        if let Some(text) = part.get("text").and_then(serde_json::Value::as_str) {
            text_parts.push(text);
        }
    }

    let text = trim_string_to_option(text_parts.join("")).unwrap_or_default();
    if text.is_empty() && tool_calls.is_empty() {
        return Err(AiModelError::MissingChatContent);
    }
    Ok(AiChatCompletion {
        text,
        reasoning_content: trim_string_to_option(reasoning_parts.join("\n\n")),
        tool_calls,
    })
}

pub fn merge_model_discoveries(models: Vec<AiModelDiscovery>) -> Vec<AiModelDiscovery> {
    let mut deduped = std::collections::BTreeMap::new();
    for model in models {
        deduped.entry(model.id.clone()).or_insert(model);
    }
    deduped.into_values().collect()
}

pub fn mask_ai_settings(mut settings: AiSettings) -> AiSettings {
    for profile in &mut settings.provider_profiles {
        profile.api_key = mask_secret(profile.api_key.take());
    }
    for credential in &mut settings.provider_credentials {
        credential.api_key = mask_secret(credential.api_key.take());
    }
    settings
}

pub fn merge_masked_ai_settings(current: &AiSettings, mut next: AiSettings) -> AiSettings {
    for profile in &mut next.provider_profiles {
        let current_secret = current
            .provider_profiles
            .iter()
            .find(|item| item.id == profile.id)
            .and_then(|item| item.api_key.as_ref());
        profile.api_key = merge_secret(current_secret, profile.api_key.as_ref());
    }
    for credential in &mut next.provider_credentials {
        let current_secret = current
            .provider_credentials
            .iter()
            .find(|item| item.id == credential.id)
            .and_then(|item| item.api_key.as_ref());
        credential.api_key = merge_secret(current_secret, credential.api_key.as_ref());
    }
    normalize_ai_settings(&mut next);
    next
}

pub fn normalize_ai_settings(settings: &mut AiSettings) -> bool {
    let original = serde_json::to_string(settings).unwrap_or_default();

    settings.schema_version = default_schema_version();
    if settings.request_user_agent.trim().is_empty() {
        settings.request_user_agent = default_request_user_agent();
    }

    if settings.provider_profiles.is_empty() {
        settings.provider_profiles = default_provider_profiles();
    }
    if settings.provider_credentials.is_empty() {
        settings.provider_credentials = settings
            .provider_profiles
            .iter()
            .map(credential_from_profile)
            .collect();
    }

    if settings.models.is_empty() {
        let mut seen = HashSet::new();
        settings.models = settings
            .provider_profiles
            .iter()
            .filter_map(model_from_profile)
            .filter(|model| seen.insert(model.id.clone()))
            .collect();
    }

    if settings.terminal_ai_actions.is_empty() {
        settings.terminal_ai_actions = default_terminal_ai_actions();
    }
    if settings.file_ai_actions.is_empty() {
        settings.file_ai_actions = default_file_ai_actions();
    }
    if settings.max_ai_file_size_bytes == 0 {
        settings.max_ai_file_size_bytes = default_max_ai_file_size_bytes();
    }
    if settings.context_line_limit == 0 {
        settings.context_line_limit = default_context_line_limit();
    }
    if settings.timeout_ms == 0 {
        settings.timeout_ms = default_timeout_ms();
    }
    if settings.terminal_output_lines == 0 {
        settings.terminal_output_lines = default_terminal_output_lines();
    }

    for model in &mut settings.models {
        if model.id.trim().is_empty() {
            model.id = if let Some(credential_id) = model.credential_id.as_deref() {
                ai_model_id_for_credential(credential_id, &model.name)
            } else if let Some(kind) = &model.provider_kind {
                ai_model_id_for_provider(kind, &model.name)
            } else {
                model.name.clone()
            };
        }
    }

    if settings.default_model_id.as_deref().is_none_or(|id| {
        !settings
            .models
            .iter()
            .any(|model| model.enabled && model.id == id)
    }) {
        let active_model = settings
            .provider_profiles
            .iter()
            .find(|profile| profile.id == settings.active_profile_id && profile.enabled)
            .and_then(model_from_profile)
            .and_then(|legacy_model| {
                settings
                    .models
                    .iter()
                    .find(|model| model.enabled && model.id == legacy_model.id)
                    .map(|model| model.id.clone())
            });

        settings.default_model_id = active_model.or_else(|| {
            settings
                .models
                .iter()
                .find(|model| model.enabled)
                .map(|model| model.id.clone())
        });
    }

    serde_json::to_string(settings).unwrap_or_default() != original
}

pub fn ai_settings_has_secret(settings: &AiSettings) -> bool {
    settings
        .provider_profiles
        .iter()
        .any(|profile| optional_secret_present(&profile.api_key))
        || settings
            .provider_credentials
            .iter()
            .any(|credential| optional_secret_present(&credential.api_key))
}

pub fn trim_ai_history(history: &mut AiHistoryFile) {
    history
        .sessions
        .sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    if history.sessions.len() > AI_HISTORY_MAX_SESSIONS {
        history.sessions.truncate(AI_HISTORY_MAX_SESSIONS);
    }

    let retained_sessions: HashSet<&str> = history
        .sessions
        .iter()
        .map(|session| session.id.as_str())
        .collect();
    history
        .messages
        .retain(|message| retained_sessions.contains(message.session_id.as_str()));

    if history.messages.len() > AI_HISTORY_MAX_MESSAGES {
        history
            .messages
            .sort_by(|left, right| left.created_at.cmp(&right.created_at));
        let remove_count = history.messages.len() - AI_HISTORY_MAX_MESSAGES;
        history.messages.drain(0..remove_count);
    }

    let sessions_with_messages: HashSet<&str> = history
        .messages
        .iter()
        .map(|message| message.session_id.as_str())
        .collect();
    history
        .sessions
        .retain(|session| sessions_with_messages.contains(session.id.as_str()));
}

pub fn trim_ai_audit(file: &mut AiAuditFile) {
    if file.logs.len() > AI_AUDIT_MAX_LOGS {
        let keep_from = file.logs.len().saturating_sub(AI_AUDIT_MAX_LOGS);
        file.logs = file.logs.split_off(keep_from);
    }
}

pub fn now_rfc3339() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

pub fn uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub fn redact_context(context: &mut AiContext) {
    context.recent_output = redact_sensitive_text(&context.recent_output);
    context.selected_text = redact_sensitive_text(&context.selected_text);
    context.input_buffer = redact_sensitive_text(&context.input_buffer);
}

pub fn redact_sensitive_text(input: &str) -> String {
    let mut output = input.to_string();
    for (pattern, replacement) in redaction_patterns() {
        output = pattern.replace_all(&output, *replacement).to_string();
    }
    output
}

pub fn parse_model_output(
    raw_text: &str,
    stream_reasoning: Option<String>,
) -> (String, Option<String>, Vec<AiCommandCard>) {
    let candidate = extract_json_object(raw_text).unwrap_or_else(|| raw_text.trim().to_string());
    match serde_json::from_str::<AiModelOutput>(&candidate) {
        Ok(output) => {
            let text = if output.text.trim().is_empty() {
                raw_text.trim().to_string()
            } else {
                output.text
            };
            let reasoning_content = trim_optional_to_option(output.reasoning)
                .or_else(|| trim_optional_to_option(stream_reasoning));
            let (text, extracted_reasoning) = extract_think_block(&text);
            let result = (
                text,
                extracted_reasoning.or(reasoning_content),
                output.command_cards,
            );
            if !result.0.is_empty() {
                return result;
            }
            promote_reasoning_to_text(result)
        }
        Err(_) => {
            let normalized_reasoning = trim_optional_to_option(stream_reasoning);
            let (text, extracted_reasoning) = extract_think_block(raw_text);
            let result = (text, extracted_reasoning.or(normalized_reasoning), vec![]);
            if !result.0.is_empty() {
                return result;
            }
            promote_reasoning_to_text(result)
        }
    }
}

pub fn extract_json_object(raw_text: &str) -> Option<String> {
    let trimmed = raw_text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        return Some(trimmed.to_string());
    }
    let start = trimmed.find('{')?;
    let end = trimmed.rfind('}')?;
    if start >= end {
        return None;
    }
    Some(trimmed[start..=end].to_string())
}

pub fn extract_text_from_assistant(content: &str) -> String {
    let trimmed = content.trim();
    if let Some(json_str) = extract_json_object(trimmed) {
        if let Ok(output) = serde_json::from_str::<AiModelOutput>(&json_str) {
            if !output.text.trim().is_empty() {
                return output.text;
            }
        }
    }
    trimmed.to_string()
}

pub fn truncate_preview(s: &str, max_len: usize) -> String {
    let trimmed = s.trim();
    if trimmed.len() <= max_len {
        trimmed.to_string()
    } else {
        let boundary = trimmed
            .char_indices()
            .map(|(i, _)| i)
            .take_while(|&i| i <= max_len)
            .last()
            .unwrap_or(0);
        format!("{}...", &trimmed[..boundary])
    }
}

pub fn trim_string_to_option(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub fn trim_optional_to_option(value: Option<String>) -> Option<String> {
    value.and_then(trim_string_to_option)
}

fn deserialize_optional_risk_level<'de, D>(deserializer: D) -> Result<Option<RiskLevel>, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    Ok(value.and_then(|raw| parse_risk_level_label(&raw)))
}

fn deserialize_required_risk_level<'de, D>(deserializer: D) -> Result<RiskLevel, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    parse_risk_level_label(&value)
        .ok_or_else(|| serde::de::Error::custom(format!("invalid riskLevel '{value}'")))
}

pub fn parse_risk_level_label(value: &str) -> Option<RiskLevel> {
    match value.trim().replace('-', "_").to_ascii_lowercase().as_str() {
        "low" => Some(RiskLevel::Low),
        "medium" | "moderate" => Some(RiskLevel::Medium),
        "high" => Some(RiskLevel::High),
        "critical" | "danger" | "dangerous" => Some(RiskLevel::Critical),
        _ => None,
    }
}

pub fn system_prompt(language: &str) -> &'static str {
    match resolve_prompt_language(language) {
        PromptLanguage::ZhCn => SYSTEM_PROMPT_ZH,
        PromptLanguage::En => SYSTEM_PROMPT_EN,
    }
}

pub fn agent_system_prompt(language: &str) -> &'static str {
    match resolve_prompt_language(language) {
        PromptLanguage::ZhCn => AGENT_SYSTEM_PROMPT_ZH,
        PromptLanguage::En => AGENT_SYSTEM_PROMPT_EN,
    }
}

fn request_system_prompt(request: &AiChatRequest) -> &'static str {
    match request.mode {
        AiMode::Ask => system_prompt(&request.options.language),
        AiMode::Agent => agent_system_prompt(&request.options.language),
    }
}

fn request_user_prompt(request: &AiChatRequest, settings: &AiSettings) -> String {
    match request.mode {
        AiMode::Ask => build_prompt(request, settings),
        AiMode::Agent => build_agent_prompt(request, settings),
    }
}

fn agent_openai_tools() -> serde_json::Value {
    serde_json::json!([
        {
            "type": "function",
            "function": {
                "name": "execute_command",
                "description": "Execute exactly one shell command in the active terminal session. Use this when more observation is needed or when the user requested an action that requires a command.",
                "strict": true,
                "parameters": {
                    "type": "object",
                    "properties": {
                        "thought": {
                            "type": "string",
                            "description": "Brief reasoning for this step and why this command is needed."
                        },
                        "command": {
                            "type": "string",
                            "description": "A single shell command to execute."
                        },
                        "riskLevel": {
                            "type": "string",
                            "enum": ["low", "medium", "high", "critical"],
                            "description": "Risk level of this command."
                        },
                        "riskReason": {
                            "type": "string",
                            "description": "Brief reason for the selected risk level."
                        }
                    },
                    "required": ["thought", "command", "riskLevel", "riskReason"],
                    "additionalProperties": false
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "final_answer",
                "description": "Finish the agent task and provide the user-facing final answer. Use this when no more command execution is needed.",
                "strict": true,
                "parameters": {
                    "type": "object",
                    "properties": {
                        "thought": {
                            "type": "string",
                            "description": "Brief reason why the task is complete or cannot continue."
                        },
                        "answer": {
                            "type": "string",
                            "description": "Final user-facing answer."
                        }
                    },
                    "required": ["thought", "answer"],
                    "additionalProperties": false
                }
            }
        }
    ])
}

fn agent_anthropic_tools() -> serde_json::Value {
    serde_json::json!([
        {
            "name": "execute_command",
            "description": "Execute exactly one shell command in the active terminal session. Use this when more observation is needed or when the user requested an action that requires a command.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "thought": {
                        "type": "string",
                        "description": "Brief reasoning for this step and why this command is needed."
                    },
                    "command": {
                        "type": "string",
                        "description": "A single shell command to execute."
                    },
                    "riskLevel": {
                        "type": "string",
                        "enum": ["low", "medium", "high", "critical"],
                        "description": "Risk level of this command."
                    },
                    "riskReason": {
                        "type": "string",
                        "description": "Brief reason for the selected risk level."
                    }
                },
                "required": ["thought", "command", "riskLevel", "riskReason"],
                "additionalProperties": false
            }
        },
        {
            "name": "final_answer",
            "description": "Finish the agent task and provide the user-facing final answer. Use this when no more command execution is needed.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "thought": {
                        "type": "string",
                        "description": "Brief reason why the task is complete or cannot continue."
                    },
                    "answer": {
                        "type": "string",
                        "description": "Final user-facing answer."
                    }
                },
                "required": ["thought", "answer"],
                "additionalProperties": false
            }
        }
    ])
}

fn agent_gemini_tools() -> serde_json::Value {
    serde_json::json!([{
        "functionDeclarations": [
            {
                "name": "execute_command",
                "description": "Execute exactly one shell command in the active terminal session. Use this when more observation is needed or when the user requested an action that requires a command.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "thought": {
                            "type": "string",
                            "description": "Brief reasoning for this step and why this command is needed."
                        },
                        "command": {
                            "type": "string",
                            "description": "A single shell command to execute."
                        },
                        "riskLevel": {
                            "type": "string",
                            "enum": ["low", "medium", "high", "critical"],
                            "description": "Risk level of this command."
                        },
                        "riskReason": {
                            "type": "string",
                            "description": "Brief reason for the selected risk level."
                        }
                    },
                    "required": ["thought", "command", "riskLevel", "riskReason"]
                }
            },
            {
                "name": "final_answer",
                "description": "Finish the agent task and provide the user-facing final answer. Use this when no more command execution is needed.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "thought": {
                            "type": "string",
                            "description": "Brief reason why the task is complete or cannot continue."
                        },
                        "answer": {
                            "type": "string",
                            "description": "Final user-facing answer."
                        }
                    },
                    "required": ["thought", "answer"]
                }
            }
        ]
    }])
}

pub fn parse_agent_model_output(raw_text: &str) -> Result<AgentLlmResponse, AiModelError> {
    let candidate = extract_json_object(raw_text).unwrap_or_else(|| raw_text.trim().to_string());
    let response: AgentLlmResponse = serde_json::from_str(&candidate)
        .map_err(|error| AiModelError::InvalidChatJson(error.to_string()))?;
    let action = response.action.trim().to_ascii_lowercase();
    match action.as_str() {
        "execute_command" => {
            if response
                .command
                .as_deref()
                .map(str::trim)
                .filter(|command| !command.is_empty())
                .is_none()
            {
                return Err(AiModelError::MissingChatContent);
            }
        }
        "final_answer" => {
            if response
                .answer
                .as_deref()
                .map(str::trim)
                .filter(|answer| !answer.is_empty())
                .is_none()
            {
                return Err(AiModelError::MissingChatContent);
            }
        }
        _ => {
            return Err(AiModelError::InvalidChatJson(format!(
                "unknown AI agent action '{}'",
                response.action
            )));
        }
    }
    Ok(response)
}

pub fn parse_agent_tool_call(tool_calls: &[AiToolCall]) -> Result<AgentLlmResponse, AiModelError> {
    let calls = tool_calls
        .iter()
        .filter(|call| !call.name.trim().is_empty())
        .collect::<Vec<_>>();
    if calls.len() != 1 {
        return Err(AiModelError::InvalidChatJson(format!(
            "expected exactly one AI agent tool call, got {}",
            calls.len()
        )));
    }
    let call = calls[0];
    match call.name.as_str() {
        "execute_command" => {
            let args: ExecuteCommandToolArgs = serde_json::from_value(call.arguments.clone())
                .map_err(|error| AiModelError::InvalidChatJson(error.to_string()))?;
            if args.command.trim().is_empty() {
                return Err(AiModelError::MissingChatContent);
            }
            Ok(AgentLlmResponse {
                thought: args.thought,
                action: "execute_command".to_string(),
                command: Some(args.command),
                risk_level: Some(args.risk_level),
                risk_reason: Some(args.risk_reason),
                answer: None,
            })
        }
        "final_answer" => {
            let args: FinalAnswerToolArgs = serde_json::from_value(call.arguments.clone())
                .map_err(|error| AiModelError::InvalidChatJson(error.to_string()))?;
            if args.answer.trim().is_empty() {
                return Err(AiModelError::MissingChatContent);
            }
            Ok(AgentLlmResponse {
                thought: args.thought,
                action: "final_answer".to_string(),
                command: None,
                risk_level: None,
                risk_reason: None,
                answer: Some(args.answer),
            })
        }
        other => Err(AiModelError::InvalidChatJson(format!(
            "unknown AI agent tool call '{other}'"
        ))),
    }
}

pub fn agent_response_action(response: &AgentLlmResponse) -> &str {
    match response.action.trim().to_ascii_lowercase().as_str() {
        "execute_command" => "execute_command",
        "final_answer" => "final_answer",
        _ => "unknown",
    }
}

pub fn assess_agent_command_risk(
    response: &AgentLlmResponse,
    command: &str,
) -> AgentCommandRiskAssessment {
    let model_risk = response.risk_level.clone().unwrap_or(RiskLevel::Medium);
    let (local_risk, local_reason) = assess_local_command_risk(command);
    let effective_risk = max_risk(model_risk.clone(), local_risk.clone());
    let risk_reason = response
        .risk_reason
        .as_ref()
        .filter(|value| !value.trim().is_empty())
        .map(|value| format!("AI: {}; local: {}", value.trim(), local_reason))
        .or_else(|| Some(format!("local: {local_reason}")));

    AgentCommandRiskAssessment {
        model_risk,
        local_risk,
        effective_risk,
        risk_reason,
    }
}

pub fn decide_agent_command_execution(
    settings: &AiSettings,
    assessment: &AgentCommandRiskAssessment,
) -> (AgentApprovalDecision, Option<String>) {
    match settings.agent_command_execution_mode {
        AgentCommandExecutionMode::ConfirmEach => (
            AgentApprovalDecision::NeedsApproval,
            Some("execution policy requires confirmation for every command".to_string()),
        ),
        AgentCommandExecutionMode::Auto => (AgentApprovalDecision::Auto, None),
        AgentCommandExecutionMode::Smart => {
            if assessment.effective_risk == RiskLevel::Critical {
                return (
                    AgentApprovalDecision::NeedsApproval,
                    Some(
                        "critical risk always requires manual confirmation in smart mode"
                            .to_string(),
                    ),
                );
            }
            if assessment.effective_risk <= settings.agent_smart_auto_execute_max_risk {
                (AgentApprovalDecision::Auto, None)
            } else {
                (
                    AgentApprovalDecision::NeedsApproval,
                    Some(format!(
                        "effective risk {} exceeds smart auto-execute threshold {}",
                        risk_label(&assessment.effective_risk),
                        risk_label(&settings.agent_smart_auto_execute_max_risk)
                    )),
                )
            }
        }
    }
}

pub fn risk_label(risk: &RiskLevel) -> &'static str {
    match risk {
        RiskLevel::Low => "low",
        RiskLevel::Medium => "medium",
        RiskLevel::High => "high",
        RiskLevel::Critical => "critical",
    }
}

fn max_risk(a: RiskLevel, b: RiskLevel) -> RiskLevel {
    if a >= b { a } else { b }
}

pub fn assess_local_command_risk(command: &str) -> (RiskLevel, String) {
    let normalized = normalize_command(command);
    let compact = normalized.split_whitespace().collect::<Vec<_>>().join(" ");

    if compact.is_empty() {
        return (RiskLevel::Medium, "empty command".to_string());
    }

    if is_root_rm_command(&compact) || is_dangerous_dd_command(&compact) {
        return (
            RiskLevel::Critical,
            "matches irreversible or system-disruptive command pattern".to_string(),
        );
    }

    let critical_patterns = [
        "mkfs",
        "wipefs",
        ":(){",
        "shutdown",
        "poweroff",
        "reboot",
        "halt",
        "systemctl stop ssh",
        "systemctl stop sshd",
        "service ssh stop",
        "service sshd stop",
    ];
    if command_contains_any(&compact, &critical_patterns) {
        return (
            RiskLevel::Critical,
            "matches irreversible or system-disruptive command pattern".to_string(),
        );
    }

    let high_patterns = [
        "rm -r",
        "rm -f",
        " rmdir ",
        " chmod -r",
        " chown -r",
        "systemctl restart",
        "systemctl stop",
        "service ",
        "apt install",
        "apt remove",
        "apt purge",
        "yum install",
        "yum remove",
        "dnf install",
        "dnf remove",
        "pacman -s",
        "pacman -r",
        "brew install",
        "brew uninstall",
        "npm install -g",
        "pip install",
        "docker rm",
        "docker rmi",
        "docker system prune",
        "kubectl delete",
        "kubectl drain",
        "kubectl apply",
        "kubectl replace",
        "git reset --hard",
        "git clean -fd",
    ];
    if compact.starts_with("sudo ") || command_contains_any(&compact, &high_patterns) {
        return (
            RiskLevel::High,
            "matches privileged, destructive, restart, package, container, or cluster mutation pattern"
                .to_string(),
        );
    }

    let medium_patterns = [
        " > ",
        ">>",
        " tee ",
        " touch ",
        " mkdir ",
        " cp ",
        " mv ",
        " chmod ",
        " chown ",
        " setfacl ",
        " export ",
        "git checkout",
        "git switch",
        "git pull",
        "git merge",
        "npm run",
        "make install",
    ];
    if command_contains_any(&format!(" {compact} "), &medium_patterns) {
        return (
            RiskLevel::Medium,
            "matches local write or state-changing command pattern".to_string(),
        );
    }

    let readonly_prefixes = [
        "ls",
        "pwd",
        "whoami",
        "id",
        "uname",
        "cat",
        "less",
        "head",
        "tail",
        "grep",
        "rg",
        "find",
        "df",
        "du",
        "free",
        "top",
        "ps",
        "ss",
        "netstat",
        "ip ",
        "journalctl",
        "systemctl status",
        "docker ps",
        "docker logs",
        "kubectl get",
        "kubectl describe",
        "git status",
        "git log",
        "git diff",
    ];
    if readonly_prefixes
        .iter()
        .any(|prefix| compact == prefix.trim() || compact.starts_with(&format!("{prefix} ")))
    {
        return (
            RiskLevel::Low,
            "matches read-only diagnostic pattern".to_string(),
        );
    }

    (
        RiskLevel::Medium,
        "no explicit read-only pattern matched; defaulting to medium".to_string(),
    )
}

fn normalize_command(command: &str) -> String {
    command
        .trim()
        .replace("\r\n", "\n")
        .replace('\n', " ")
        .to_ascii_lowercase()
}

fn command_contains_any(command: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| command.contains(pattern))
}

fn is_root_rm_command(command: &str) -> bool {
    let tokens: Vec<&str> = command.split_whitespace().collect();
    if tokens.first() != Some(&"rm") {
        return false;
    }
    let has_recursive_force = tokens
        .iter()
        .any(|token| token.starts_with('-') && token.contains('r') && token.contains('f'));
    has_recursive_force
        && tokens
            .iter()
            .skip(1)
            .any(|token| matches!(*token, "/" | "/*" | "--no-preserve-root"))
}

fn is_dangerous_dd_command(command: &str) -> bool {
    command.starts_with("dd ") && command.contains("of=/dev/")
}

pub fn build_agent_prompt(request: &AiChatRequest, settings: &AiSettings) -> String {
    let ctx = &request.context;
    if resolve_prompt_language(&request.options.language) == PromptLanguage::ZhCn {
        format!(
            r#"用户任务：
{user_input}

当前连接上下文：
- 连接名：{connection_name}
- 主机：{host}
- 用户：{username}
- 当前目录：{cwd}
- 操作系统：{os}
- 架构：{arch}

最近终端输出（最多 {line_limit} 行）：
{recent_output}

要求：
- 面向用户的说明、总结以及推理过程使用：{language}
- 命令、路径、文件名、配置键名保持原样，不要翻译

请开始执行任务。每轮调用且只调用一个工具。"#,
            user_input = request.user_input,
            connection_name = ctx.connection_name.as_deref().unwrap_or("-"),
            host = ctx.host.as_deref().unwrap_or("-"),
            username = ctx.username.as_deref().unwrap_or("-"),
            cwd = ctx.cwd.as_deref().unwrap_or("-"),
            os = ctx.os.as_deref().unwrap_or("-"),
            arch = ctx.arch.as_deref().unwrap_or(std::env::consts::ARCH),
            line_limit = settings.context_line_limit,
            recent_output = ctx.recent_output.as_str(),
            language = request.options.language,
        )
    } else {
        format!(
            r#"User task:
{user_input}

Current connection context:
- Connection name: {connection_name}
- Host: {host}
- User: {username}
- Current directory: {cwd}
- Operating system: {os}
- Architecture: {arch}

Recent terminal output (up to {line_limit} lines):
{recent_output}

Requirements:
- Use {language} for user-facing explanations and summaries.
- Prefer {language} for reasoning when possible.
- Keep commands, paths, file names, and configuration keys unchanged.

Start the task now. Call exactly one tool per turn."#,
            user_input = request.user_input,
            connection_name = ctx.connection_name.as_deref().unwrap_or("-"),
            host = ctx.host.as_deref().unwrap_or("-"),
            username = ctx.username.as_deref().unwrap_or("-"),
            cwd = ctx.cwd.as_deref().unwrap_or("-"),
            os = ctx.os.as_deref().unwrap_or("-"),
            arch = ctx.arch.as_deref().unwrap_or(std::env::consts::ARCH),
            line_limit = settings.context_line_limit,
            recent_output = ctx.recent_output.as_str(),
            language = request.options.language,
        )
    }
}

pub fn build_observation_message(
    obs: &CommandObservation,
    command: &str,
    language: &str,
) -> String {
    let status = obs
        .exit_code
        .map(|code| format!("exit code {code}"))
        .unwrap_or_else(|| "unknown exit code".to_string());
    let output = if obs.output.len() > 8000 {
        let truncated = &obs.output[obs.output.len() - 8000..];
        format!("...(truncated)\n{truncated}")
    } else {
        obs.output.clone()
    };
    if resolve_prompt_language(language) == PromptLanguage::ZhCn {
        format!(
            "命令 `{command}` 执行完成（{status}，耗时 {duration}ms）。\n\n输出：\n{output}\n\n请根据观察结果决定下一步。每轮必须且只能调用一个工具：execute_command 或 final_answer。不要在普通正文里输出 JSON。",
            duration = obs.duration_ms,
        )
    } else {
        format!(
            "Command `{command}` finished ({status}, {duration}ms).\n\nOutput:\n{output}\n\nDecide the next step based on this observation. Call exactly one tool: execute_command or final_answer. Do not put protocol JSON in normal assistant text.",
            duration = obs.duration_ms,
        )
    }
}

pub fn build_prompt(request: &AiChatRequest, settings: &AiSettings) -> String {
    let ctx = &request.context;
    if resolve_prompt_language(&request.options.language) == PromptLanguage::ZhCn {
        let action = match request.action {
            AiAction::GenerateCommand => "根据自然语言需求生成 1 到 2 条 Shell 命令",
            AiAction::ExplainOutput => "解释最近终端输出并给出下一步建议",
            AiAction::ExplainSelected => "解释用户选中的终端文本并给出下一步建议",
            AiAction::AnalyzeError => "分析终端错误输出并给出排查步骤",
            AiAction::RepairFromSelection => "根据选中内容生成修复或排查命令",
            AiAction::CustomTerminalAction => "根据用户配置的终端 AI 功能处理选中内容",
            AiAction::CustomFileAction => "根据用户配置的文件 AI 功能处理文件内容",
        };
        format!(
            r#"任务：{action}
用户需求：
{user_input}

当前连接上下文：
- 连接名：{connection_name}
- 主机：{host}
- 端口：{port}
- 用户：{username}
- 当前目录：{cwd}
- 操作系统：{os}
- 架构：{arch}
- 当前输入：{input_buffer}

选中文本：
{selected_text}

最近终端输出（最多 {line_limit} 行）：
{recent_output}

要求：
- 语言：{language}
- 面向用户的说明和推理过程使用该语言；命令、路径、文件名、配置键名保持原样
- 安全模式：{safety_mode}
- 最多生成 {max_commands} 条命令
- 优先生成只读诊断命令
- 如果信息不足，请给出验证命令
- 必须返回 JSON 对象，不要返回 Markdown"#,
            user_input = request.user_input,
            connection_name = ctx.connection_name.as_deref().unwrap_or("-"),
            host = ctx.host.as_deref().unwrap_or("-"),
            port = ctx
                .port
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string()),
            username = ctx.username.as_deref().unwrap_or("-"),
            cwd = ctx.cwd.as_deref().unwrap_or("-"),
            os = ctx.os.as_deref().unwrap_or("-"),
            arch = ctx.arch.as_deref().unwrap_or(std::env::consts::ARCH),
            input_buffer = ctx.input_buffer.as_str(),
            selected_text = ctx.selected_text.as_str(),
            line_limit = settings.context_line_limit,
            recent_output = ctx.recent_output.as_str(),
            language = request.options.language,
            safety_mode = request.options.safety_mode,
            max_commands = request.options.max_output_commands,
        )
    } else {
        let action = match request.action {
            AiAction::GenerateCommand => {
                "Generate 1 to 2 Shell commands from the natural language request"
            }
            AiAction::ExplainOutput => {
                "Explain the recent terminal output and suggest the next step"
            }
            AiAction::ExplainSelected => {
                "Explain the selected terminal text and suggest the next step"
            }
            AiAction::AnalyzeError => {
                "Analyze the terminal error output and provide troubleshooting steps"
            }
            AiAction::RepairFromSelection => {
                "Generate repair or troubleshooting commands from the selected content"
            }
            AiAction::CustomTerminalAction => {
                "Handle the selected content using the configured terminal AI action"
            }
            AiAction::CustomFileAction => {
                "Handle the file content using the configured file AI action"
            }
        };
        format!(
            r#"Task: {action}
User request:
{user_input}

Current connection context:
- Connection name: {connection_name}
- Host: {host}
- Port: {port}
- User: {username}
- Current directory: {cwd}
- Operating system: {os}
- Architecture: {arch}
- Current input: {input_buffer}

Selected text:
{selected_text}

Recent terminal output (up to {line_limit} lines):
{recent_output}

Requirements:
- Target language: {language}
- Use that language for user-facing explanation and reasoning when possible.
- Keep commands, paths, file names, and configuration keys unchanged.
- Safety mode: {safety_mode}
- Generate at most {max_commands} commands.
- Prefer read-only diagnostic commands first.
- If information is insufficient, provide verification commands.
- Return a JSON object only. Do not return Markdown."#,
            user_input = request.user_input,
            connection_name = ctx.connection_name.as_deref().unwrap_or("-"),
            host = ctx.host.as_deref().unwrap_or("-"),
            port = ctx
                .port
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string()),
            username = ctx.username.as_deref().unwrap_or("-"),
            cwd = ctx.cwd.as_deref().unwrap_or("-"),
            os = ctx.os.as_deref().unwrap_or("-"),
            arch = ctx.arch.as_deref().unwrap_or(std::env::consts::ARCH),
            input_buffer = ctx.input_buffer.as_str(),
            selected_text = ctx.selected_text.as_str(),
            line_limit = settings.context_line_limit,
            recent_output = ctx.recent_output.as_str(),
            language = request.options.language,
            safety_mode = request.options.safety_mode,
            max_commands = request.options.max_output_commands,
        )
    }
}

fn default_schema_version() -> u32 {
    3
}

fn default_true() -> bool {
    true
}

fn default_context_line_limit() -> u32 {
    200
}

fn default_timeout_ms() -> u64 {
    60_000
}

fn default_request_user_agent() -> String {
    AI_REQUEST_USER_AGENT_DEFAULT.to_string()
}

fn default_mode() -> AiMode {
    AiMode::Ask
}

fn default_model_source() -> AiModelSource {
    AiModelSource::RustGenai
}

fn default_terminal_output_lines() -> u16 {
    10
}

fn default_agent_smart_auto_execute_max_risk() -> RiskLevel {
    RiskLevel::Low
}

fn default_max_ai_file_size_bytes() -> u64 {
    1_048_576
}

fn default_active_profile_id() -> String {
    "openai".to_string()
}

fn default_provider_profiles() -> Vec<AiProviderProfile> {
    vec![
        AiProviderProfile {
            id: "openai".to_string(),
            name: "OpenAI".to_string(),
            provider_kind: AiProviderKind::Openai,
            model: "gpt-4o-mini".to_string(),
            base_url: None,
            api_key: None,
            enabled: false,
        },
        AiProviderProfile {
            id: "anthropic".to_string(),
            name: "Anthropic".to_string(),
            provider_kind: AiProviderKind::Anthropic,
            model: "claude-3-haiku-20240307".to_string(),
            base_url: None,
            api_key: None,
            enabled: false,
        },
        AiProviderProfile {
            id: "gemini".to_string(),
            name: "Google Gemini".to_string(),
            provider_kind: AiProviderKind::Gemini,
            model: "gemini-2.0-flash".to_string(),
            base_url: None,
            api_key: None,
            enabled: false,
        },
        AiProviderProfile {
            id: "deepseek".to_string(),
            name: "DeepSeek".to_string(),
            provider_kind: AiProviderKind::Deepseek,
            model: "deepseek-chat".to_string(),
            base_url: None,
            api_key: None,
            enabled: false,
        },
        AiProviderProfile {
            id: "ollama".to_string(),
            name: "Ollama".to_string(),
            provider_kind: AiProviderKind::Ollama,
            model: "llama3-7b".to_string(),
            base_url: Some("http://localhost:11434/v1/".to_string()),
            api_key: None,
            enabled: false,
        },
        AiProviderProfile {
            id: "xai".to_string(),
            name: "xAI".to_string(),
            provider_kind: AiProviderKind::Xai,
            model: "grok-3".to_string(),
            base_url: Some("https://api.x.ai/v1/".to_string()),
            api_key: None,
            enabled: false,
        },
        AiProviderProfile {
            id: "cohere".to_string(),
            name: "Cohere".to_string(),
            provider_kind: AiProviderKind::Cohere,
            model: "command-a-03-2025".to_string(),
            base_url: Some("https://api.cohere.com/compatibility/v1/".to_string()),
            api_key: None,
            enabled: false,
        },
        AiProviderProfile {
            id: "mimo".to_string(),
            name: "Mimo".to_string(),
            provider_kind: AiProviderKind::Mimo,
            model: "mimo-v2.5-pro".to_string(),
            base_url: Some("https://api.xiaomimimo.com/v1/".to_string()),
            api_key: None,
            enabled: false,
        },
        AiProviderProfile {
            id: "zai".to_string(),
            name: "ZAI".to_string(),
            provider_kind: AiProviderKind::Zai,
            model: "glm-4".to_string(),
            base_url: Some("https://open.bigmodel.cn/api/paas/v4/".to_string()),
            api_key: None,
            enabled: false,
        },
    ]
}

fn default_provider_credentials() -> Vec<AiProviderCredential> {
    default_provider_profiles()
        .iter()
        .map(credential_from_profile)
        .collect()
}

fn default_models() -> Vec<AiModelConfigItem> {
    Vec::new()
}

fn default_terminal_ai_actions() -> Vec<AiCustomActionConfig> {
    vec![
        AiCustomActionConfig {
            id: "explain-selected".to_string(),
            name: "\u{89e3}\u{91ca}\u{9009}\u{4e2d}\u{5185}\u{5bb9}".to_string(),
            prompt: "\u{8bf7}\u{89e3}\u{91ca}\u{7ec8}\u{7aef}\u{4e2d}\u{9009}\u{4e2d}\u{7684}\u{5185}\u{5bb9}\u{ff0c}\u{6307}\u{51fa}\u{542b}\u{4e49}\u{3001}\u{53ef}\u{80fd}\u{539f}\u{56e0}\u{548c}\u{4e0b}\u{4e00}\u{6b65}\u{5efa}\u{8bae}\u{3002}".to_string(),
            enabled: true,
        },
        AiCustomActionConfig {
            id: "generate-fix-command".to_string(),
            name: "\u{751f}\u{6210}\u{4fee}\u{590d}\u{547d}\u{4ee4}".to_string(),
            prompt: "\u{8bf7}\u{6839}\u{636e}\u{7ec8}\u{7aef}\u{9009}\u{4e2d}\u{5185}\u{5bb9}\u{751f}\u{6210}\u{53ef}\u{6267}\u{884c}\u{7684}\u{4fee}\u{590d}\u{547d}\u{4ee4}\u{ff0c}\u{5e76}\u{8bf4}\u{660e}\u{98ce}\u{9669}\u{3002}".to_string(),
            enabled: true,
        },
    ]
}

fn default_file_ai_actions() -> Vec<AiCustomActionConfig> {
    vec![
        AiCustomActionConfig {
            id: "summarize-file".to_string(),
            name: "\u{603b}\u{7ed3}\u{6587}\u{4ef6}".to_string(),
            prompt: "\u{8bf7}\u{603b}\u{7ed3}\u{9009}\u{4e2d}\u{6587}\u{4ef6}\u{7684}\u{4e3b}\u{8981}\u{5185}\u{5bb9}\u{3001}\u{5173}\u{952e}\u{98ce}\u{9669}\u{548c}\u{5efa}\u{8bae}\u{64cd}\u{4f5c}\u{3002}".to_string(),
            enabled: true,
        },
        AiCustomActionConfig {
            id: "explain-file".to_string(),
            name: "\u{89e3}\u{91ca}\u{6587}\u{4ef6}".to_string(),
            prompt: "\u{8bf7}\u{89e3}\u{91ca}\u{9009}\u{4e2d}\u{6587}\u{4ef6}\u{7684}\u{7528}\u{9014}\u{3001}\u{7ed3}\u{6784}\u{548c}\u{5173}\u{952e}\u{5b57}\u{6bb5}\u{3002}".to_string(),
            enabled: true,
        },
    ]
}

fn provider_kind_key(kind: &AiProviderKind) -> &'static str {
    match kind {
        AiProviderKind::Openai => "openai",
        AiProviderKind::Anthropic => "anthropic",
        AiProviderKind::Gemini => "gemini",
        AiProviderKind::Deepseek => "deepseek",
        AiProviderKind::Groq => "groq",
        AiProviderKind::Ollama => "ollama",
        AiProviderKind::Xai => "xai",
        AiProviderKind::Cohere => "cohere",
        AiProviderKind::Mimo => "mimo",
        AiProviderKind::Zai => "zai",
        AiProviderKind::OpenaiCompatible => "openai_compatible",
    }
}

fn credential_from_profile(profile: &AiProviderProfile) -> AiProviderCredential {
    AiProviderCredential {
        id: profile.id.clone(),
        name: profile.name.clone(),
        provider_kind: profile.provider_kind.clone(),
        base_url: profile.base_url.clone(),
        api_key: profile.api_key.clone(),
        enabled: profile.enabled,
    }
}

fn model_from_profile(profile: &AiProviderProfile) -> Option<AiModelConfigItem> {
    let name = profile.model.trim();
    if name.is_empty() {
        return None;
    }

    let is_manual = profile.provider_kind == AiProviderKind::OpenaiCompatible
        || profile
            .base_url
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty());
    let id = if is_manual {
        ai_model_id_for_credential(&profile.id, name)
    } else {
        ai_model_id_for_provider(&profile.provider_kind, name)
    };

    Some(AiModelConfigItem {
        id,
        name: name.to_string(),
        provider_kind: Some(profile.provider_kind.clone()),
        credential_id: is_manual.then(|| profile.id.clone()),
        enabled: profile.enabled,
        source: if is_manual {
            AiModelSource::Manual
        } else {
            AiModelSource::RustGenai
        },
        last_seen_at: None,
    })
}

fn mask_secret(value: Option<String>) -> Option<String> {
    value.and_then(|secret| {
        if secret.is_empty() {
            None
        } else {
            Some(MASKED_SECRET_VALUE.to_string())
        }
    })
}

fn merge_secret(current: Option<&String>, incoming: Option<&String>) -> Option<String> {
    match incoming.map(String::as_str) {
        Some(MASKED_SECRET_VALUE) | None => current.cloned(),
        Some("") => None,
        Some(value) => Some(value.to_string()),
    }
}

fn optional_secret_present(value: &Option<String>) -> bool {
    value.as_deref().is_some_and(|value| !value.is_empty())
}

fn chat_history_for_request(
    request: &AiChatRequest,
    settings: &AiSettings,
    history: &[AiMessage],
    assistant_role: &str,
) -> Vec<serde_json::Value> {
    let mut messages = Vec::new();

    if let Some(session_id) = request.session_id.as_deref() {
        let max_turns = request.options.history_turns as usize;
        if max_turns > 0 {
            let session_messages = history
                .iter()
                .filter(|message| message.session_id == session_id)
                .collect::<Vec<_>>();
            let skip = session_messages.len().saturating_sub(max_turns);
            for message in session_messages.into_iter().skip(skip) {
                match message.role {
                    AiMessageRole::User => {
                        messages.push(serde_json::json!({
                            "role": "user",
                            "content": message.content,
                        }));
                    }
                    AiMessageRole::Assistant => {
                        let content = extract_text_from_assistant(&message.content);
                        if !content.is_empty() {
                            messages.push(serde_json::json!({
                                "role": assistant_role,
                                "content": content,
                            }));
                        }
                    }
                    AiMessageRole::System => {}
                }
            }
        }
    }

    messages.push(serde_json::json!({
        "role": "user",
        "content": request_user_prompt(request, settings),
    }));
    messages
}

fn join_api_base_url(base_url: &str, path: &str) -> Result<String, AiModelError> {
    let trimmed = base_url.trim();
    if trimmed.is_empty() {
        return Err(AiModelError::InvalidBaseUrl {
            base_url: base_url.to_string(),
            message: "base URL is empty".to_string(),
        });
    }
    let (base_without_query, query) = match trimmed.split_once('?') {
        Some((base, query)) => (base, Some(query)),
        None => (trimmed, None),
    };
    let base = base_without_query.trim_end_matches('/');
    if base.is_empty() {
        return Err(AiModelError::InvalidBaseUrl {
            base_url: base_url.to_string(),
            message: "base URL path is empty".to_string(),
        });
    }
    let path = path.trim_start_matches('/');
    let joined = format!("{base}/{path}");
    Ok(match query {
        Some(query) if !query.is_empty() => format!("{joined}?{query}"),
        _ => joined,
    })
}

fn sse_data_payloads(chunk: &str) -> Vec<String> {
    let normalized_chunk = chunk.replace("\r\n", "\n");
    normalized_chunk
        .split("\n\n")
        .filter_map(|event| {
            let data_lines = event
                .lines()
                .filter_map(|line| line.strip_prefix("data:"))
                .map(str::trim_start)
                .collect::<Vec<_>>();
            (!data_lines.is_empty()).then(|| data_lines.join("\n"))
        })
        .collect()
}

fn extract_openai_compatible_message_content(message: &serde_json::Value) -> Option<String> {
    if let Some(content) = message
        .get("content")
        .and_then(serde_json::Value::as_str)
        .and_then(|value| trim_string_to_option(value.to_string()))
    {
        return Some(content);
    }

    let parts = message.get("content")?.as_array()?;
    let text = parts
        .iter()
        .filter_map(|part| {
            part.get("text")
                .and_then(serde_json::Value::as_str)
                .or_else(|| part.get("content").and_then(serde_json::Value::as_str))
        })
        .collect::<Vec<_>>()
        .join("");
    trim_string_to_option(text)
}

fn extract_openai_compatible_tool_calls(
    message: &serde_json::Value,
) -> Result<Vec<AiToolCall>, AiModelError> {
    let Some(calls) = message
        .get("tool_calls")
        .or_else(|| message.get("toolCalls"))
        .and_then(serde_json::Value::as_array)
    else {
        return Ok(Vec::new());
    };
    calls
        .iter()
        .filter(|call| call.get("function").is_some() || call.get("name").is_some())
        .map(|call| {
            let function = call.get("function").unwrap_or(call);
            let name = function
                .get("name")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    AiModelError::InvalidChatJson("tool call is missing function name".to_string())
                })?
                .to_string();
            let arguments = match function.get("arguments") {
                Some(serde_json::Value::String(raw)) => {
                    if raw.trim().is_empty() {
                        serde_json::Value::Object(Default::default())
                    } else {
                        serde_json::from_str(raw)
                            .map_err(|error| AiModelError::InvalidChatJson(error.to_string()))?
                    }
                }
                Some(value) => value.clone(),
                None => serde_json::Value::Object(Default::default()),
            };
            Ok(AiToolCall {
                id: call
                    .get("id")
                    .and_then(serde_json::Value::as_str)
                    .map(ToOwned::to_owned),
                name,
                arguments,
            })
        })
        .collect()
}

fn extract_openai_compatible_stream_content(delta: &serde_json::Value) -> String {
    if let Some(content) = delta.get("content").and_then(serde_json::Value::as_str) {
        return content.to_string();
    }

    delta
        .get("content")
        .and_then(serde_json::Value::as_array)
        .map(|parts| {
            parts
                .iter()
                .filter_map(|part| {
                    part.get("text")
                        .and_then(serde_json::Value::as_str)
                        .or_else(|| part.get("content").and_then(serde_json::Value::as_str))
                })
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default()
}

fn extract_openai_compatible_stream_tool_call_deltas(
    delta: &serde_json::Value,
) -> Vec<AiToolCallDelta> {
    let Some(calls) = delta
        .get("tool_calls")
        .or_else(|| delta.get("toolCalls"))
        .and_then(serde_json::Value::as_array)
    else {
        return Vec::new();
    };

    calls
        .iter()
        .enumerate()
        .filter_map(|(fallback_index, call)| {
            let function = call.get("function").unwrap_or(call);
            let id_delta = call
                .get("id")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
                .filter(|value| !value.is_empty());
            let name_delta = function
                .get("name")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
                .filter(|value| !value.is_empty());
            let arguments_delta = function
                .get("arguments")
                .map(|value| {
                    value
                        .as_str()
                        .map(ToOwned::to_owned)
                        .unwrap_or_else(|| value.to_string())
                })
                .unwrap_or_default();
            if id_delta.is_none() && name_delta.is_none() && arguments_delta.is_empty() {
                return None;
            }
            Some(AiToolCallDelta {
                index: call
                    .get("index")
                    .and_then(serde_json::Value::as_u64)
                    .and_then(|value| usize::try_from(value).ok())
                    .unwrap_or(fallback_index),
                id_delta,
                name_delta,
                arguments_delta,
            })
        })
        .collect()
}

fn extract_anthropic_stream_tool_call_deltas(value: &serde_json::Value) -> Vec<AiToolCallDelta> {
    let event_type = value
        .get("type")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let index = value
        .get("index")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or_default();

    match event_type {
        "content_block_start" => {
            let Some(block) = value.get("content_block") else {
                return Vec::new();
            };
            if block
                .get("type")
                .and_then(serde_json::Value::as_str)
                .is_none_or(|block_type| block_type != "tool_use")
            {
                return Vec::new();
            }
            let id_delta = block
                .get("id")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
                .filter(|value| !value.is_empty());
            let name_delta = block
                .get("name")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
                .filter(|value| !value.is_empty());
            let arguments_delta = block
                .get("input")
                .filter(|input| !input.as_object().is_some_and(serde_json::Map::is_empty))
                .map(serde_json::Value::to_string)
                .unwrap_or_default();
            if id_delta.is_none() && name_delta.is_none() && arguments_delta.is_empty() {
                return Vec::new();
            }
            vec![AiToolCallDelta {
                index,
                id_delta,
                name_delta,
                arguments_delta,
            }]
        }
        "content_block_delta" => {
            let Some(delta) = value.get("delta") else {
                return Vec::new();
            };
            if delta
                .get("type")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|delta_type| delta_type != "input_json_delta")
            {
                return Vec::new();
            }
            let arguments_delta = delta
                .get("partial_json")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
                .unwrap_or_default();
            if arguments_delta.is_empty() {
                return Vec::new();
            }
            vec![AiToolCallDelta {
                index,
                id_delta: None,
                name_delta: None,
                arguments_delta,
            }]
        }
        _ => Vec::new(),
    }
}

fn promote_reasoning_to_text(
    (text, reasoning, cards): (String, Option<String>, Vec<AiCommandCard>),
) -> (String, Option<String>, Vec<AiCommandCard>) {
    if !text.is_empty() {
        return (text, reasoning, cards);
    }
    let reasoning_str = match reasoning.as_deref() {
        Some(reasoning) if !reasoning.trim().is_empty() => reasoning,
        _ => return (text, reasoning, cards),
    };

    if let Some(json_str) = extract_json_object(reasoning_str) {
        if let Ok(output) = serde_json::from_str::<AiModelOutput>(&json_str) {
            let promoted_text = if output.text.trim().is_empty() {
                json_str.clone()
            } else {
                output.text
            };
            let inner_reasoning = trim_optional_to_option(output.reasoning);
            return (promoted_text, inner_reasoning, output.command_cards);
        }
    }

    let (visible, inner_reasoning) = extract_think_block(reasoning_str);
    if !visible.is_empty() {
        return (visible, inner_reasoning, cards);
    }

    (reasoning.unwrap_or_default(), None, cards)
}

fn extract_think_block(raw_text: &str) -> (String, Option<String>) {
    static THINK_REGEX: OnceLock<Regex> = OnceLock::new();
    let regex = THINK_REGEX.get_or_init(|| Regex::new(r"(?is)<think>(.*?)</think>").unwrap());

    let mut reasoning_parts = Vec::new();
    for captures in regex.captures_iter(raw_text) {
        if let Some(value) = captures.get(1) {
            let reasoning = value.as_str().trim();
            if !reasoning.is_empty() {
                reasoning_parts.push(reasoning.to_string());
            }
        }
    }

    let visible_text = regex.replace_all(raw_text, "").to_string();
    (
        visible_text.trim().to_string(),
        trim_string_to_option(reasoning_parts.join("\n\n")),
    )
}

fn redaction_patterns() -> &'static [(Regex, &'static str)] {
    static PATTERNS: OnceLock<Vec<(Regex, &'static str)>> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        vec![
            (
                Regex::new(
                    r"-----BEGIN [A-Z ]*PRIVATE KEY-----[\s\S]*?-----END [A-Z ]*PRIVATE KEY-----",
                )
                .unwrap(),
                "[REDACTED_PRIVATE_KEY]",
            ),
            (
                Regex::new(r"(?i)Authorization:\s*Bearer\s+[A-Za-z0-9._\-]+").unwrap(),
                "Authorization: Bearer [REDACTED]",
            ),
            (
                Regex::new(r"(?i)(password|passwd|pwd)\s*[:=]\s*[^\s;&|]+").unwrap(),
                "$1=[REDACTED]",
            ),
            (
                Regex::new(
                    r"(?i)(token|api[_-]?key|secret[_-]?key|access[_-]?key)\s*[:=]\s*[^\s;&|]+",
                )
                .unwrap(),
                "$1=[REDACTED]",
            ),
            (
                Regex::new(r"AKIA[0-9A-Z]{16}").unwrap(),
                "[REDACTED_AWS_ACCESS_KEY]",
            ),
            (
                Regex::new(r"(?i)(postgres|mysql|mongodb)://[^@\s]+@").unwrap(),
                "$1://[REDACTED]@",
            ),
        ]
    })
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
enum PromptLanguage {
    ZhCn,
    En,
}

fn normalize_prompt_locale(language: &str) -> String {
    let normalized = language.trim().replace('_', "-").to_ascii_lowercase();
    match normalized.as_str() {
        "zh" | "zh-cn" | "zh-hans" | "zh-hans-cn" => "zh-cn".to_string(),
        "en" | "en-us" | "en-gb" => "en".to_string(),
        _ => normalized,
    }
}

fn prompt_language_map() -> &'static HashMap<&'static str, PromptLanguage> {
    static PROMPT_LANGUAGE_MAP: OnceLock<HashMap<&'static str, PromptLanguage>> = OnceLock::new();
    PROMPT_LANGUAGE_MAP.get_or_init(|| {
        HashMap::from([("zh-cn", PromptLanguage::ZhCn), ("en", PromptLanguage::En)])
    })
}

fn resolve_prompt_language(language: &str) -> PromptLanguage {
    let normalized = normalize_prompt_locale(language);
    prompt_language_map()
        .get(normalized.as_str())
        .copied()
        .unwrap_or(PromptLanguage::En)
}

fn default_max_output_commands() -> u8 {
    5
}

fn default_language() -> String {
    "en".to_string()
}

fn default_safety_mode() -> String {
    "strict".to_string()
}

fn default_history_turns() -> u16 {
    20
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_preserves_masked_api_key() {
        let mut current = AiSettings::default();
        current.provider_profiles[0].api_key = Some("real-key".to_string());
        current.provider_credentials[0].api_key = Some("credential-key".to_string());
        let mut next = current.clone();
        next.provider_profiles[0].api_key = Some(MASKED_SECRET_VALUE.to_string());
        next.provider_credentials[0].api_key = Some(MASKED_SECRET_VALUE.to_string());

        let merged = merge_masked_ai_settings(&current, next);
        assert_eq!(
            merged.provider_profiles[0].api_key.as_deref(),
            Some("real-key")
        );
        assert_eq!(
            merged.provider_credentials[0].api_key.as_deref(),
            Some("credential-key")
        );
    }

    #[test]
    fn mask_replaces_configured_api_key() {
        let mut settings = AiSettings::default();
        settings.provider_profiles[0].api_key = Some("real-key".to_string());
        settings.provider_credentials[0].api_key = Some("credential-key".to_string());

        let masked = mask_ai_settings(settings);
        assert_eq!(
            masked.provider_profiles[0].api_key.as_deref(),
            Some(MASKED_SECRET_VALUE)
        );
        assert_eq!(
            masked.provider_credentials[0].api_key.as_deref(),
            Some(MASKED_SECRET_VALUE)
        );
    }

    #[test]
    fn normalize_migrates_legacy_profiles_to_v3_settings() {
        let mut settings = AiSettings {
            schema_version: 2,
            provider_credentials: vec![],
            models: vec![],
            terminal_ai_actions: vec![],
            file_ai_actions: vec![],
            default_model_id: None,
            max_ai_file_size_bytes: 0,
            ..AiSettings::default()
        };
        settings.active_profile_id = "deepseek".to_string();
        settings.provider_profiles[3].enabled = true;

        assert!(normalize_ai_settings(&mut settings));
        assert_eq!(settings.schema_version, 3);
        assert!(!settings.provider_credentials.is_empty());
        assert!(
            settings
                .models
                .iter()
                .any(|model| model.name == "deepseek-chat")
        );
        assert_eq!(
            settings.default_model_id.as_deref(),
            Some("deepseek:deepseek-chat")
        );
        assert_eq!(settings.max_ai_file_size_bytes, 1_048_576);
        assert!(!settings.terminal_ai_actions.is_empty());
        assert!(!settings.file_ai_actions.is_empty());
        assert_eq!(
            settings.agent_command_execution_mode,
            AgentCommandExecutionMode::ConfirmEach
        );
        assert_eq!(settings.agent_smart_auto_execute_max_risk, RiskLevel::Low);
        assert!(!settings.agent_background_execution_enabled);
    }

    #[test]
    fn legacy_ai_settings_default_background_execution_to_disabled() {
        let settings: AiSettings = serde_json::from_value(serde_json::json!({
            "schema_version": 3,
            "enabled": true
        }))
        .expect("legacy settings should deserialize");

        assert!(!settings.agent_background_execution_enabled);
    }

    #[test]
    fn old_history_without_reasoning_defaults_cleanly() {
        let raw = r#"{"sessions":[],"messages":[{"id":"m1","sessionId":"s1","role":"assistant","content":"hello","createdAt":"2026-04-28T00:00:00Z","commandCards":[]}]}"#;
        let history: AiHistoryFile = serde_json::from_str(raw).unwrap();
        assert_eq!(history.messages.len(), 1);
        assert_eq!(history.messages[0].reasoning_content, None);
    }

    #[test]
    fn trims_ai_history_to_session_and_message_limits() {
        let mut history = AiHistoryFile::default();
        for session_idx in 0..220 {
            let session_id = format!("s-{session_idx:03}");
            let updated_at = format!(
                "2026-04-28T00:{:02}:{:02}Z",
                session_idx / 60,
                session_idx % 60
            );
            history.sessions.push(AiSession {
                id: session_id.clone(),
                connection_id: None,
                title: session_id.clone(),
                created_at: updated_at.clone(),
                updated_at,
            });
            for message_idx in 0..10 {
                history.messages.push(AiMessage {
                    id: format!("m-{session_idx:03}-{message_idx:02}"),
                    session_id: session_id.clone(),
                    role: if message_idx % 2 == 0 {
                        AiMessageRole::User
                    } else {
                        AiMessageRole::Assistant
                    },
                    content: "message".to_string(),
                    created_at: format!(
                        "2026-04-28T00:{:02}:{:02}.{:03}Z",
                        session_idx / 60,
                        session_idx % 60,
                        message_idx
                    ),
                    reasoning_content: None,
                    command_cards: vec![],
                });
            }
        }

        trim_ai_history(&mut history);

        assert_eq!(history.sessions.len(), AI_HISTORY_MAX_SESSIONS);
        assert_eq!(history.messages.len(), AI_HISTORY_MAX_MESSAGES);
        let retained_sessions: HashSet<&str> = history
            .sessions
            .iter()
            .map(|session| session.id.as_str())
            .collect();
        assert!(!retained_sessions.contains("s-000"));
        assert!(retained_sessions.contains("s-219"));
        assert!(
            history
                .messages
                .iter()
                .all(|message| retained_sessions.contains(message.session_id.as_str()))
        );
    }

    #[test]
    fn trims_ai_audit_to_latest_entries() {
        let mut file = AiAuditFile::default();
        for index in 0..(AI_AUDIT_MAX_LOGS + 10) {
            file.logs.push(AiAuditLog {
                id: format!("audit-{index}"),
                connection_id: None,
                action: "generate_command".to_string(),
                user_input: None,
                generated_command: None,
                risk_level: None,
                inserted_to_terminal: false,
                executed: false,
                blocked: false,
                created_at: format!("2026-04-28T00:00:{:02}Z", index % 60),
            });
        }

        trim_ai_audit(&mut file);

        assert_eq!(file.logs.len(), AI_AUDIT_MAX_LOGS);
        assert_eq!(file.logs[0].id, "audit-10");
    }

    #[test]
    fn redacts_sensitive_values_in_context() {
        let mut context = AiContext {
            recent_output:
                "password=secret token:abc Authorization: Bearer abc.def AKIA1234567890ABCDEF"
                    .to_string(),
            selected_text: "postgres://user:pass@localhost/db".to_string(),
            input_buffer: "api_key=real".to_string(),
            ..AiContext::default()
        };

        redact_context(&mut context);

        assert!(!context.recent_output.contains("secret"));
        assert!(!context.recent_output.contains("abc.def"));
        assert!(!context.recent_output.contains("AKIA1234567890ABCDEF"));
        assert_eq!(context.selected_text, "postgres://[REDACTED]@localhost/db");
        assert_eq!(context.input_buffer, "api_key=[REDACTED]");
    }

    #[test]
    fn parses_json_command_cards() {
        let raw = r#"{"text":"ok","commandCards":[{"id":"1","title":"CPU","command":"ps aux","explanation":"x","riskLevel":"low","riskReason":"read only","expectedEffect":"list","rollback":"none"}]}"#;
        let (text, reasoning, cards) = parse_model_output(raw, None);
        assert_eq!(text, "ok");
        assert_eq!(reasoning, None);
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].risk_level, Some(RiskLevel::Low));
    }

    #[test]
    fn parser_extracts_think_block_and_keeps_markdown_on_json_failure() {
        let (text, reasoning, cards) =
            parse_model_output("<think>step 1\nstep 2</think>final answer", None);
        assert_eq!(text, "final answer");
        assert_eq!(reasoning.as_deref(), Some("step 1\nstep 2"));
        assert!(cards.is_empty());

        let markdown = "## Summary\n\n- item 1\n- item 2";
        let (text, reasoning, cards) = parse_model_output(markdown, None);
        assert_eq!(text, markdown);
        assert_eq!(reasoning, None);
        assert!(cards.is_empty());
    }

    #[test]
    fn parser_promotes_reasoning_json_when_text_is_empty() {
        let reasoning = r#"{"text":"answer from reasoning","commandCards":[]}"#.to_string();
        let (text, reasoning, cards) = parse_model_output("", Some(reasoning));
        assert_eq!(text, "answer from reasoning");
        assert_eq!(reasoning, None);
        assert!(cards.is_empty());
    }

    #[test]
    fn extract_text_from_assistant_prefers_json_text() {
        let content = r#"```json
{"text":"visible","reasoning":"hidden","commandCards":[]}
```"#;
        assert_eq!(extract_text_from_assistant(content), "visible");
        assert_eq!(extract_text_from_assistant("plain"), "plain");
    }

    #[test]
    fn prompt_builder_uses_locale_and_context() {
        let request = sample_ai_request("zh_CN");
        let settings = AiSettings::default();

        let system = system_prompt("zh_CN");
        let prompt = build_prompt(&request, &settings);
        let agent_prompt = build_agent_prompt(&request, &settings);

        assert!(system.contains("终端助手"));
        assert!(prompt.contains("任务："));
        assert!(prompt.contains("db-01"));
        assert!(prompt.contains("最多生成 5 条命令"));
        assert!(agent_prompt.contains("每轮调用且只调用一个工具"));
        assert!(agent_system_prompt("en-US").contains("terminal automation agent"));
    }

    #[test]
    fn agent_mode_chat_body_uses_agent_protocol_prompts() {
        let settings = AiSettings::default();
        let mut request = sample_ai_request("en");
        request.mode = AiMode::Agent;
        request.user_input = "inspect disk usage".to_string();
        let resolved = ResolvedAiModel {
            model_name: "gpt-test".to_string(),
            provider_kind: AiProviderKind::Openai,
            credential: None,
        };

        let body = build_openai_compatible_chat_request_body(&resolved, &request, &settings, &[]);
        let messages = body["messages"].as_array().expect("messages array");

        assert!(
            messages[0]["content"]
                .as_str()
                .expect("system prompt")
                .contains("terminal automation agent")
        );
        assert!(
            messages[1]["content"]
                .as_str()
                .expect("user prompt")
                .contains("Call exactly one tool per turn")
        );
    }

    #[test]
    fn parses_agent_response_and_assesses_execution_policy() {
        let parsed = parse_agent_model_output(
            r#"{"thought":"Need to inspect files","action":"execute_command","command":"ls -la","riskLevel":"LOW","riskReason":"read only"}"#,
        )
        .expect("agent response");
        assert_eq!(agent_response_action(&parsed), "execute_command");
        assert_eq!(parsed.risk_level, Some(RiskLevel::Low));

        let assessment = assess_agent_command_risk(&parsed, "ls -la");
        assert_eq!(assessment.effective_risk, RiskLevel::Low);

        let mut settings = AiSettings {
            agent_command_execution_mode: AgentCommandExecutionMode::Smart,
            agent_smart_auto_execute_max_risk: RiskLevel::Low,
            ..AiSettings::default()
        };
        assert_eq!(
            decide_agent_command_execution(&settings, &assessment).0,
            AgentApprovalDecision::Auto
        );

        settings.agent_command_execution_mode = AgentCommandExecutionMode::ConfirmEach;
        assert_eq!(
            decide_agent_command_execution(&settings, &assessment).0,
            AgentApprovalDecision::NeedsApproval
        );
    }

    #[test]
    fn local_agent_risk_overrides_unsafe_model_risk() {
        let parsed = parse_agent_model_output(
            r#"{"thought":"danger","action":"execute_command","command":"rm -rf /","riskLevel":"low","riskReason":"claimed safe"}"#,
        )
        .expect("agent response");
        let assessment = assess_agent_command_risk(&parsed, "rm -rf /");

        assert_eq!(assessment.model_risk, RiskLevel::Low);
        assert_eq!(assessment.local_risk, RiskLevel::Critical);
        assert_eq!(assessment.effective_risk, RiskLevel::Critical);
        assert!(assessment.risk_reason.unwrap().contains("claimed safe"));
    }

    #[test]
    fn observation_message_truncates_long_output() {
        let obs = CommandObservation {
            output: "x".repeat(8_100),
            exit_code: Some(0),
            duration_ms: 42,
        };

        let message = build_observation_message(&obs, "ls", "en");

        assert!(message.contains("exit code 0"));
        assert!(message.contains("...(truncated)"));
        assert!(message.len() < 8_500);
    }

    #[test]
    fn resolves_requested_ai_model_with_credential() {
        let mut settings = AiSettings::default();
        settings.provider_profiles[0].enabled = true;
        settings.provider_credentials[0].enabled = true;
        settings.provider_credentials[0].api_key = Some("key".to_string());
        normalize_ai_settings(&mut settings);
        let model_id = "openai:gpt-4o-mini".to_string();
        settings.default_model_id = Some(model_id.clone());

        let mut request = sample_ai_request("en");
        request.model_id = Some(model_id);
        let resolved = resolve_request_model(&settings, &request).expect("resolve model");

        assert_eq!(resolved.model_name, "gpt-4o-mini");
        assert_eq!(resolved.provider_kind, AiProviderKind::Openai);
        assert_eq!(
            resolved
                .credential
                .as_ref()
                .and_then(|credential| credential.api_key.as_deref()),
            Some("key")
        );
    }

    #[test]
    fn resolve_model_allows_ollama_without_api_key() {
        let mut settings = AiSettings::default();
        settings.active_profile_id = "ollama".to_string();
        settings.provider_profiles[4].enabled = true;
        settings.provider_credentials[4].enabled = true;
        normalize_ai_settings(&mut settings);
        settings.default_model_id = Some("ollama:llama3-7b".to_string());

        let resolved = resolve_request_model(&settings, &sample_ai_request("en"))
            .expect("ollama should not require api key");

        assert_eq!(resolved.provider_kind, AiProviderKind::Ollama);
    }

    #[test]
    fn resolve_model_reports_missing_api_key() {
        let mut settings = AiSettings::default();
        settings.provider_profiles[0].enabled = true;
        settings.provider_credentials[0].enabled = true;
        normalize_ai_settings(&mut settings);
        settings.default_model_id = Some("openai:gpt-4o-mini".to_string());

        let error = resolve_request_model(&settings, &sample_ai_request("en")).unwrap_err();

        assert_eq!(
            error,
            AiModelError::MissingApiKey {
                credential: "OpenAI".to_string()
            }
        );
    }

    #[test]
    fn openai_compatible_models_url_matches_legacy_joining() {
        assert_eq!(
            openai_compatible_models_url("https://api.example.com/v1").unwrap(),
            "https://api.example.com/v1/models"
        );
        assert_eq!(
            openai_compatible_models_url("https://api.example.com/v1/").unwrap(),
            "https://api.example.com/v1/models"
        );
        assert_eq!(
            openai_compatible_models_url("https://api.example.com/v1?api-version=1").unwrap(),
            "https://api.example.com/v1/models?api-version=1"
        );
        assert_eq!(
            openai_compatible_chat_completions_url("https://api.example.com/v1/").unwrap(),
            "https://api.example.com/v1/chat/completions"
        );
    }

    #[test]
    fn parses_and_deduplicates_openai_compatible_model_discovery() {
        let credential = AiProviderCredential {
            id: "custom".to_string(),
            name: "Custom".to_string(),
            provider_kind: AiProviderKind::OpenaiCompatible,
            base_url: Some("https://api.example.com/v1".to_string()),
            api_key: None,
            enabled: true,
        };
        let raw = r#"{"data":[{"id":"llama3"},{"id":" "},{"id":"qwen3"}]}"#;

        let models =
            parse_openai_compatible_models_response(raw, &credential).expect("parse models");
        let merged = merge_model_discoveries(vec![
            models[0].clone(),
            models[0].clone(),
            models[1].clone(),
        ]);

        assert_eq!(models.len(), 2);
        assert_eq!(models[0].id, "custom:llama3");
        assert_eq!(models[0].credential_id.as_deref(), Some("custom"));
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn builds_openai_compatible_chat_body_with_history() {
        let mut settings = AiSettings::default();
        settings.context_line_limit = 10;
        let request = sample_ai_request("en");
        let resolved = ResolvedAiModel {
            model_name: "deepseek-chat-none".to_string(),
            provider_kind: AiProviderKind::Deepseek,
            credential: Some(AiProviderCredential {
                id: "deepseek".to_string(),
                name: "DeepSeek".to_string(),
                provider_kind: AiProviderKind::Deepseek,
                base_url: Some("https://api.deepseek.com/v1".to_string()),
                api_key: Some("key".to_string()),
                enabled: true,
            }),
        };
        let history = vec![
            AiMessage {
                id: "m1".to_string(),
                session_id: "session-1".to_string(),
                role: AiMessageRole::User,
                content: "previous question".to_string(),
                created_at: "2026-04-28T00:00:00Z".to_string(),
                reasoning_content: None,
                command_cards: vec![],
            },
            AiMessage {
                id: "m2".to_string(),
                session_id: "session-1".to_string(),
                role: AiMessageRole::Assistant,
                content: r#"{"text":"previous answer","commandCards":[]}"#.to_string(),
                created_at: "2026-04-28T00:00:01Z".to_string(),
                reasoning_content: None,
                command_cards: vec![],
            },
        ];

        let body =
            build_openai_compatible_chat_request_body(&resolved, &request, &settings, &history);
        let stream_body = build_openai_compatible_chat_request_body_with_stream(
            &resolved, &request, &settings, &history, true,
        );
        let messages = body["messages"].as_array().expect("messages array");

        assert_eq!(body["model"], "deepseek-chat");
        assert_eq!(body["stream"], false);
        assert_eq!(stream_body["stream"], true);
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages[1]["content"], "previous question");
        assert_eq!(messages[2]["content"], "previous answer");
        assert!(
            messages[3]["content"]
                .as_str()
                .unwrap()
                .contains("show disk usage")
        );
    }

    #[test]
    fn agent_openai_compatible_body_requires_tools() {
        let settings = AiSettings {
            context_line_limit: 10,
            ..AiSettings::default()
        };
        let mut request = sample_ai_request("en");
        request.mode = AiMode::Agent;
        let resolved = ResolvedAiModel {
            model_name: "gpt-4o-mini".to_string(),
            provider_kind: AiProviderKind::Openai,
            credential: None,
        };

        let body = build_openai_compatible_chat_request_body(&resolved, &request, &settings, &[]);
        let tools = body["tools"].as_array().expect("tools");

        assert_eq!(body["tool_choice"], "required");
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0]["function"]["name"], "execute_command");
        assert_eq!(
            tools[0]["function"]["parameters"]["required"],
            serde_json::json!(["thought", "command", "riskLevel", "riskReason"])
        );
        assert_eq!(tools[1]["function"]["name"], "final_answer");
    }

    #[test]
    fn parses_openai_compatible_stream_deltas_and_done() {
        let raw = concat!(
            "event: message\r\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"hel\"}}]}\r\n\r\n",
            "data: {\"choices\":[{\"delta\":{\"content\":[{\"text\":\"lo\"}],\"reasoning_content\":\"think\"}}]}\r\n\r\n",
            "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"call-1\",\"function\":{\"name\":\"execute_command\",\"arguments\":\"{\\\"thought\\\":\\\"inspect\\\",\"}}]}}]}\r\n\r\n",
            "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"\\\"command\\\":\\\"pwd\\\",\\\"riskLevel\\\":\\\"low\\\",\\\"riskReason\\\":\\\"read only\\\"}\"}}]}}]}\r\n\r\n",
            "data: [DONE]\r\n\r\n",
        );

        let deltas = parse_openai_compatible_stream_chunk(raw).expect("parse stream");

        assert_eq!(deltas.len(), 5);
        assert_eq!(deltas[0].text_delta, "hel");
        assert_eq!(deltas[1].text_delta, "lo");
        assert_eq!(deltas[1].reasoning_delta.as_deref(), Some("think"));
        assert_eq!(deltas[2].tool_call_deltas[0].index, 0);
        assert_eq!(
            deltas[2].tool_call_deltas[0].name_delta.as_deref(),
            Some("execute_command")
        );
        assert_eq!(
            deltas[3].tool_call_deltas[0].arguments_delta,
            "\"command\":\"pwd\",\"riskLevel\":\"low\",\"riskReason\":\"read only\"}"
        );
        assert!(deltas[4].done);
    }

    #[test]
    fn parses_openai_compatible_chat_response_content_and_reasoning() {
        let raw = r#"{
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": [{"type":"text","text":"hello"},{"type":"text","text":" world"}],
                    "reasoning_content": "thought"
                }
            }]
        }"#;

        let completion = parse_openai_compatible_chat_response(raw).expect("parse chat response");

        assert_eq!(completion.text, "hello world");
        assert_eq!(completion.reasoning_content.as_deref(), Some("thought"));
        assert_eq!(
            parse_openai_compatible_chat_response(r#"{"choices":[{"message":{}}]}"#).unwrap_err(),
            AiModelError::MissingChatContent
        );
    }

    #[test]
    fn parses_openai_compatible_agent_tool_calls() {
        let raw = r#"{
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call-1",
                        "type": "function",
                        "function": {
                            "name": "execute_command",
                            "arguments": "{\"thought\":\"Need files\",\"command\":\"ls -la\",\"riskLevel\":\"LOW\",\"riskReason\":\"read only\"}"
                        }
                    }]
                }
            }]
        }"#;

        let completion = parse_openai_compatible_chat_response(raw).expect("parse tool call");
        let parsed = parse_agent_tool_call(&completion.tool_calls).expect("parse agent tool");

        assert_eq!(completion.text, "");
        assert_eq!(completion.tool_calls[0].id.as_deref(), Some("call-1"));
        assert_eq!(agent_response_action(&parsed), "execute_command");
        assert_eq!(parsed.command.as_deref(), Some("ls -la"));
        assert_eq!(parsed.risk_level, Some(RiskLevel::Low));
    }

    #[test]
    fn builds_and_parses_anthropic_chat_payloads() {
        let settings = AiSettings {
            context_line_limit: 10,
            ..AiSettings::default()
        };
        let request = sample_ai_request("en");
        let resolved = ResolvedAiModel {
            model_name: "claude-3-haiku-20240307".to_string(),
            provider_kind: AiProviderKind::Anthropic,
            credential: Some(AiProviderCredential {
                id: "anthropic".to_string(),
                name: "Anthropic".to_string(),
                provider_kind: AiProviderKind::Anthropic,
                base_url: None,
                api_key: Some("key".to_string()),
                enabled: true,
            }),
        };
        let history = sample_ai_history();

        assert_eq!(
            anthropic_messages_url("https://api.anthropic.com/v1/").unwrap(),
            "https://api.anthropic.com/v1/messages"
        );
        let body = build_anthropic_chat_request_body(&resolved, &request, &settings, &history);
        let stream_body = build_anthropic_chat_request_body_with_stream(
            &resolved, &request, &settings, &history, true,
        );
        let messages = body["messages"].as_array().expect("messages array");

        assert_eq!(body["model"], "claude-3-haiku-20240307");
        assert_eq!(body["stream"], false);
        assert_eq!(stream_body["stream"], true);
        assert_eq!(body["max_tokens"], 4096);
        assert_eq!(body["system"], system_prompt("en"));
        assert_eq!(messages[0]["role"], "user");
        assert_eq!(messages[1]["role"], "assistant");
        assert_eq!(messages[1]["content"], "previous answer");
        assert!(
            messages[2]["content"]
                .as_str()
                .unwrap()
                .contains("show disk usage")
        );

        let completion = parse_anthropic_chat_response(
            r#"{"content":[{"type":"thinking","thinking":"plan"},{"type":"text","text":"hello"},{"type":"text","text":" world"}]}"#,
        )
        .expect("parse anthropic response");
        assert_eq!(completion.text, "hello world");
        assert_eq!(completion.reasoning_content.as_deref(), Some("plan"));
        assert_eq!(
            parse_anthropic_chat_response(r#"{"content":[]}"#).unwrap_err(),
            AiModelError::MissingChatContent
        );
        let mut agent_request = request.clone();
        agent_request.mode = AiMode::Agent;
        let agent_body =
            build_anthropic_chat_request_body(&resolved, &agent_request, &settings, &history);
        assert_eq!(agent_body["tool_choice"]["type"], "any");
        assert_eq!(agent_body["tools"][0]["name"], "execute_command");
        let tool_completion = parse_anthropic_chat_response(
            r#"{"content":[{"type":"tool_use","id":"tool-1","name":"final_answer","input":{"thought":"done","answer":"ok"}}]}"#,
        )
        .expect("parse anthropic tool use");
        let parsed_tool =
            parse_agent_tool_call(&tool_completion.tool_calls).expect("parse agent tool");
        assert_eq!(agent_response_action(&parsed_tool), "final_answer");
        assert_eq!(parsed_tool.answer.as_deref(), Some("ok"));

        let deltas = parse_anthropic_stream_chunk(concat!(
            "event: content_block_start\r\n",
            "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"tool_use\",\"id\":\"tool-1\",\"name\":\"execute_command\",\"input\":{}}}\r\n\r\n",
            "event: content_block_delta\r\n",
            "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"thought\\\":\\\"inspect\\\",\\\"command\\\":\\\"pwd\\\",\\\"riskLevel\\\":\\\"low\\\",\\\"riskReason\\\":\\\"read only\\\"}\"}}\r\n\r\n",
            "event: content_block_delta\r\n",
            "data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"plan\"}}\r\n\r\n",
            "event: content_block_delta\r\n",
            "data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"hello\"}}\r\n\r\n",
            "event: message_stop\r\n",
            "data: {\"type\":\"message_stop\"}\r\n\r\n",
        ))
        .expect("parse anthropic stream");
        assert_eq!(deltas.len(), 5);
        assert_eq!(
            deltas[0].tool_call_deltas[0].id_delta.as_deref(),
            Some("tool-1")
        );
        assert_eq!(
            deltas[0].tool_call_deltas[0].name_delta.as_deref(),
            Some("execute_command")
        );
        assert!(
            deltas[1].tool_call_deltas[0]
                .arguments_delta
                .contains("\"command\":\"pwd\"")
        );
        assert_eq!(deltas[2].reasoning_delta.as_deref(), Some("plan"));
        assert_eq!(deltas[3].text_delta, "hello");
        assert!(deltas[4].done);
    }

    #[test]
    fn builds_and_parses_gemini_chat_payloads() {
        let settings = AiSettings {
            context_line_limit: 10,
            ..AiSettings::default()
        };
        let request = sample_ai_request("en");
        let history = sample_ai_history();

        assert_eq!(
            gemini_generate_content_url(
                "https://generativelanguage.googleapis.com/v1beta/",
                "gemini-1.5-flash"
            )
            .unwrap(),
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash:generateContent"
        );
        assert_eq!(
            gemini_stream_generate_content_url(
                "https://generativelanguage.googleapis.com/v1beta/",
                "gemini-1.5-flash"
            )
            .unwrap(),
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash:streamGenerateContent?alt=sse"
        );
        let body = build_gemini_chat_request_body(&request, &settings, &history);
        let contents = body["contents"].as_array().expect("contents array");

        assert_eq!(
            body["systemInstruction"]["parts"][0]["text"],
            system_prompt("en")
        );
        assert_eq!(contents[0]["role"], "user");
        assert_eq!(contents[1]["role"], "model");
        assert_eq!(contents[1]["parts"][0]["text"], "previous answer");
        assert!(
            contents[2]["parts"][0]["text"]
                .as_str()
                .unwrap()
                .contains("show disk usage")
        );

        let completion = parse_gemini_chat_response(
            r#"{"candidates":[{"content":{"parts":[{"thought":true,"text":"plan"},{"text":"hello"},{"text":" world"}]}}]}"#,
        )
        .expect("parse gemini response");
        assert_eq!(completion.text, "hello world");
        assert_eq!(completion.reasoning_content.as_deref(), Some("plan"));
        assert_eq!(
            parse_gemini_chat_response(r#"{"candidates":[]}"#).unwrap_err(),
            AiModelError::MissingChatContent
        );
        let mut agent_request = request.clone();
        agent_request.mode = AiMode::Agent;
        let agent_body = build_gemini_chat_request_body(&agent_request, &settings, &history);
        assert_eq!(
            agent_body["toolConfig"]["functionCallingConfig"]["mode"],
            "ANY"
        );
        assert_eq!(
            agent_body["tools"][0]["functionDeclarations"][0]["name"],
            "execute_command"
        );
        let tool_completion = parse_gemini_chat_response(
            r#"{"candidates":[{"content":{"parts":[{"functionCall":{"name":"execute_command","args":{"thought":"inspect","command":"pwd","riskLevel":"low","riskReason":"read only"}}}]}}]}"#,
        )
        .expect("parse gemini function call");
        let parsed_tool =
            parse_agent_tool_call(&tool_completion.tool_calls).expect("parse agent tool");
        assert_eq!(agent_response_action(&parsed_tool), "execute_command");
        assert_eq!(parsed_tool.command.as_deref(), Some("pwd"));

        let deltas = parse_gemini_stream_chunk(concat!(
            "data: {\"candidates\":[{\"content\":{\"parts\":[{\"thought\":true,\"text\":\"plan\"}]}}]}\r\n\r\n",
            "data: {\"candidates\":[{\"content\":{\"parts\":[{\"functionCall\":{\"name\":\"execute_command\",\"args\":{\"thought\":\"inspect\",\"command\":\"pwd\",\"riskLevel\":\"low\",\"riskReason\":\"read only\"}}}]}}]}\r\n\r\n",
            "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"hello\"},{\"text\":\" world\"}]}}]}\r\n\r\n",
        ))
        .expect("parse gemini stream");
        assert_eq!(deltas.len(), 3);
        assert_eq!(deltas[0].reasoning_delta.as_deref(), Some("plan"));
        assert_eq!(
            deltas[1].tool_call_deltas[0].name_delta.as_deref(),
            Some("execute_command")
        );
        assert!(
            deltas[1].tool_call_deltas[0]
                .arguments_delta
                .contains("\"command\":\"pwd\"")
        );
        assert_eq!(deltas[2].text_delta, "hello world");
    }

    #[test]
    fn user_agent_and_deepseek_mapping_match_legacy() {
        let mut settings = AiSettings::default();
        settings.request_user_agent = "   ".to_string();
        assert_eq!(
            effective_ai_request_user_agent(&settings),
            AI_REQUEST_USER_AGENT_DEFAULT
        );
        settings.request_user_agent = "nyaterm-test/1.0".to_string();
        assert_eq!(
            effective_ai_request_user_agent(&settings),
            "nyaterm-test/1.0"
        );

        assert_eq!(
            genai_model_name(&AiProviderKind::Deepseek, "deepseek-v4-flash-none"),
            "deepseek-v4-flash"
        );
        assert_eq!(
            genai_model_name(&AiProviderKind::Openai, "gpt-test-none"),
            "gpt-test-none"
        );
    }

    fn sample_ai_request(language: &str) -> AiChatRequest {
        AiChatRequest {
            stream_id: None,
            session_id: Some("session-1".to_string()),
            connection_id: Some("connection-1".to_string()),
            terminal_session_id: Some("terminal-1".to_string()),
            mode: AiMode::Ask,
            model_id: None,
            model_name: None,
            action: AiAction::GenerateCommand,
            user_input: "show disk usage".to_string(),
            context: AiContext {
                connection_name: Some("prod".to_string()),
                host: Some("db-01".to_string()),
                port: Some(22),
                username: Some("root".to_string()),
                cwd: Some("/srv".to_string()),
                os: Some("linux".to_string()),
                arch: Some("x86_64".to_string()),
                recent_output: "df -h".to_string(),
                selected_text: "/srv/data".to_string(),
                input_buffer: String::new(),
            },
            options: AiRequestOptions {
                language: language.to_string(),
                ..AiRequestOptions::default()
            },
        }
    }

    fn sample_ai_history() -> Vec<AiMessage> {
        vec![
            AiMessage {
                id: "m1".to_string(),
                session_id: "session-1".to_string(),
                role: AiMessageRole::User,
                content: "previous question".to_string(),
                created_at: "2026-04-28T00:00:00Z".to_string(),
                reasoning_content: None,
                command_cards: vec![],
            },
            AiMessage {
                id: "m2".to_string(),
                session_id: "session-1".to_string(),
                role: AiMessageRole::Assistant,
                content: r#"{"text":"previous answer","commandCards":[]}"#.to_string(),
                created_at: "2026-04-28T00:00:01Z".to_string(),
                reasoning_content: None,
                command_cards: vec![],
            },
        ]
    }
}
