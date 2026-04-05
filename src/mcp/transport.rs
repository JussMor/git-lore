use std::future::Future;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use rmcp::{
    model::{CallToolRequestParams, CallToolResult, Content, Implementation, ListToolsResult, ServerCapabilities, ServerInfo, Tool},
    service::{RequestContext, RoleServer},
    transport::stdio,
    ServerHandler, ServiceExt,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use super::{McpService, ProposalRequest};
use crate::lore::{AtomState, LoreKind};

#[derive(Clone, Debug)]
pub struct McpServer {
    service: McpService,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
struct ContextToolArgs {
    #[serde(alias = "filePath")]
    file_path: String,
    #[serde(default, alias = "cursorLine")]
    cursor_line: Option<usize>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
struct ProposeToolArgs {
    #[serde(alias = "filePath")]
    file_path: String,
    #[serde(default, alias = "cursorLine")]
    cursor_line: Option<usize>,
    #[serde(default = "default_kind")]
    kind: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    scope: Option<String>,
    #[serde(default, alias = "validationScript")]
    validation_script: Option<String>,
    #[serde(alias = "stateChecksum")]
    state_checksum: String,
    #[serde(alias = "snapshotGeneratedUnixSeconds", alias = "stateGeneratedUnixSeconds")]
    snapshot_generated_unix_seconds: u64,
    #[serde(default = "default_autofill")]
    autofill: bool,
}

#[derive(Debug, Default, Deserialize, Serialize, schemars::JsonSchema)]
struct StateSnapshotToolArgs {}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
struct MemoryPreflightToolArgs {
    #[serde(default = "default_operation", alias = "writeOperation")]
    operation: String,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
struct MemorySearchToolArgs {
    query: String,
    #[serde(default, alias = "filePath")]
    file_path: Option<String>,
    #[serde(default, alias = "cursorLine")]
    cursor_line: Option<usize>,
    #[serde(default = "default_search_limit")]
    limit: usize,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
struct StateTransitionPreviewToolArgs {
    #[serde(alias = "atomId")]
    atom_id: String,
    #[serde(alias = "targetState")]
    target_state: String,
}

#[derive(Debug, Serialize)]
struct ToolErrorPayload {
    code: String,
    message: String,
    retryable: bool,
    recommended_action: Option<String>,
}

fn default_kind() -> String {
    "decision".to_string()
}

fn default_operation() -> String {
    "edit".to_string()
}

fn default_autofill() -> bool {
    true
}

fn default_search_limit() -> usize {
    10
}

const STATE_GUARD_TTL_SECONDS: u64 = 120;
const STATE_GUARD_CLOCK_SKEW_SECONDS: u64 = 5;

impl McpServer {
    pub fn new(workspace_hint: impl AsRef<Path>) -> Self {
        Self {
            service: McpService::new(workspace_hint),
        }
    }

    pub fn run_stdio(&self) -> Result<()> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .context("failed to initialize tokio runtime for MCP server")?;

        runtime.block_on(async {
            let service = self
                .clone()
                .serve(stdio())
                .await
                .context("failed to start rmcp stdio server")?;
            service
                .waiting()
                .await
                .context("rmcp stdio server exited with an error")
                .map(|_| ())
        })
    }

    fn call_tool_impl(&self, request: CallToolRequestParams) -> CallToolResult {
        match request.name.as_ref() {
            "git_lore_context" => {
                let params = match parse_arguments::<ContextToolArgs>(request.arguments) {
                    Ok(value) => value,
                    Err(error) => {
                        return tool_error_with_code(
                            "invalid_params",
                            error,
                            false,
                            Some("Provide a valid JSON payload for git_lore_context"),
                        )
                    }
                };

                let snapshot = match self
                    .service
                    .context(Path::new(&params.file_path), params.cursor_line)
                {
                    Ok(value) => value,
                    Err(error) => {
                        return tool_error_with_code(
                            "context_lookup_failed",
                            error.to_string(),
                            true,
                            Some("Retry with an existing file path inside the workspace"),
                        )
                    }
                };

                tool_json_response(&snapshot)
            }
            "git_lore_propose" => {
                let params = match parse_arguments::<ProposeToolArgs>(request.arguments) {
                    Ok(value) => value,
                    Err(error) => {
                        return tool_error_with_code(
                            "invalid_params",
                            error,
                            false,
                            Some("Provide required fields including state_checksum and snapshot_generated_unix_seconds"),
                        )
                    }
                };

                let kind = match parse_kind(&params.kind) {
                    Ok(value) => value,
                    Err(error) => {
                        return tool_error_with_code(
                            "invalid_kind",
                            error,
                            false,
                            Some("Use one of: decision, assumption, open_question, signal"),
                        )
                    }
                };

                let autofilled = if params.autofill {
                    match self.service.autofill_proposal(
                        Path::new(&params.file_path),
                        params.cursor_line,
                        kind.clone(),
                        params.title.clone(),
                        params.body.clone(),
                        params.scope.clone(),
                    ) {
                        Ok(value) => value,
                        Err(error) => {
                            return tool_error_with_code(
                                "autofill_failed",
                                error.to_string(),
                                true,
                                Some("Retry with autofill disabled and explicit title/body"),
                            )
                        }
                    }
                } else {
                    let title = params
                        .title
                        .as_deref()
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(str::to_string)
                        .ok_or_else(|| {
                            tool_error_with_code(
                                "missing_title",
                                "title is required when autofill=false".to_string(),
                                false,
                                Some("Provide title or enable autofill"),
                            )
                        });

                    let title = match title {
                        Ok(value) => value,
                        Err(error) => return error,
                    };

                    super::ProposalAutofill {
                        title,
                        body: params.body.clone(),
                        scope: params.scope.clone(),
                        filled_fields: Vec::new(),
                    }
                };

                if let Err(error) = self.enforce_state_first_guard(
                    &params.state_checksum,
                    params.snapshot_generated_unix_seconds,
                ) {
                    return tool_error_with_code(
                        "state_guard_failed",
                        format!(
                            "state-first guard failed: {error}. Refresh with git_lore_state_snapshot and retry"
                        ),
                        true,
                        Some("Call git_lore_state_snapshot and retry with fresh guard values"),
                    );
                }

                let proposal = ProposalRequest {
                    file_path: PathBuf::from(params.file_path),
                    cursor_line: params.cursor_line,
                    kind,
                    title: autofilled.title,
                    body: autofilled.body,
                    scope: autofilled.scope,
                    validation_script: params.validation_script,
                };

                let result = match self.service.propose(proposal) {
                    Ok(value) => value,
                    Err(error) => {
                        return tool_error_with_code(
                            "propose_failed",
                            error.to_string(),
                            true,
                            Some("Fix preflight issues and retry"),
                        )
                    }
                };

                tool_json_response(&result)
            }
            "git_lore_state_snapshot" => {
                if let Err(error) = parse_arguments::<StateSnapshotToolArgs>(request.arguments) {
                    return tool_error_with_code(
                        "invalid_params",
                        error,
                        false,
                        Some("git_lore_state_snapshot does not require parameters"),
                    );
                }

                let snapshot = match self.service.state_snapshot() {
                    Ok(value) => value,
                    Err(error) => {
                        return tool_error_with_code(
                            "state_snapshot_failed",
                            error.to_string(),
                            true,
                            Some("Ensure .lore workspace is initialized"),
                        )
                    }
                };

                tool_json_response(&snapshot)
            }
            "git_lore_memory_preflight" => {
                let params = match parse_arguments::<MemoryPreflightToolArgs>(request.arguments) {
                    Ok(value) => value,
                    Err(error) => {
                        return tool_error_with_code(
                            "invalid_params",
                            error,
                            false,
                            Some("Provide operation as edit, commit, or sync"),
                        )
                    }
                };

                let operation = match parse_operation(&params.operation) {
                    Ok(value) => value,
                    Err(error) => {
                        return tool_error_with_code(
                            "invalid_operation",
                            error,
                            false,
                            Some("Use operation: edit, commit, or sync"),
                        )
                    }
                };

                let report = match self.service.memory_preflight(operation) {
                    Ok(value) => value,
                    Err(error) => {
                        return tool_error_with_code(
                            "preflight_failed",
                            error.to_string(),
                            true,
                            Some("Retry after resolving workspace or git discovery issues"),
                        )
                    }
                };

                tool_json_response(&report)
            }
            "git_lore_memory_search" => {
                let params = match parse_arguments::<MemorySearchToolArgs>(request.arguments) {
                    Ok(value) => value,
                    Err(error) => {
                        return tool_error_with_code(
                            "invalid_params",
                            error,
                            false,
                            Some("Provide query and optional file_path/cursor_line/limit"),
                        )
                    }
                };

                let report = match self.service.memory_search(
                    &params.query,
                    params.file_path.map(PathBuf::from),
                    params.cursor_line,
                    params.limit,
                ) {
                    Ok(value) => value,
                    Err(error) => {
                        return tool_error_with_code(
                            "memory_search_failed",
                            error.to_string(),
                            true,
                            Some("Retry with a non-empty query and valid workspace path"),
                        )
                    }
                };

                tool_json_response(&report)
            }
            "git_lore_state_transition_preview" => {
                let params = match parse_arguments::<StateTransitionPreviewToolArgs>(request.arguments) {
                    Ok(value) => value,
                    Err(error) => {
                        return tool_error_with_code(
                            "invalid_params",
                            error,
                            false,
                            Some("Provide atom_id and target_state"),
                        )
                    }
                };

                let target_state = match parse_atom_state(&params.target_state) {
                    Ok(value) => value,
                    Err(error) => {
                        return tool_error_with_code(
                            "invalid_target_state",
                            error,
                            false,
                            Some("Use one of: draft, proposed, accepted, deprecated"),
                        )
                    }
                };

                let preview = match self
                    .service
                    .state_transition_preview(&params.atom_id, target_state)
                {
                    Ok(value) => value,
                    Err(error) => {
                        return tool_error_with_code(
                            "state_transition_preview_failed",
                            error.to_string(),
                            true,
                            Some("Retry after refreshing workspace state"),
                        )
                    }
                };

                tool_json_response(&preview)
            }
            other => tool_error_with_code(
                "unknown_tool",
                format!("unknown tool: {other}"),
                false,
                Some("Call list_tools and use one of the advertised tool names"),
            ),
        }
    }

    fn enforce_state_first_guard(
        &self,
        provided_checksum: &str,
        snapshot_generated_unix_seconds: u64,
    ) -> std::result::Result<(), String> {
        let provided_checksum = provided_checksum.trim();
        if provided_checksum.is_empty() {
            return Err("missing state checksum".to_string());
        }

        validate_snapshot_freshness(snapshot_generated_unix_seconds, now_unix_seconds())?;

        let snapshot = self
            .service
            .state_snapshot()
            .map_err(|error| format!("failed to load state snapshot: {error}"))?;

        if snapshot.state_checksum != provided_checksum {
            return Err(format!(
                "state checksum mismatch (provided {}, current {})",
                provided_checksum, snapshot.state_checksum
            ));
        }

        Ok(())
    }
}

impl ServerHandler for McpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            server_info: Implementation {
                name: "git-lore".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                ..Default::default()
            },
            instructions: Some(
                "Git-Lore MCP server exposing context, proposal, state snapshot, preflight, memory search, and state transition preview tools."
                    .to_string(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }

    fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = std::result::Result<CallToolResult, rmcp::ErrorData>> + Send + '_ {
        async move { Ok(self.call_tool_impl(request)) }
    }

    fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = std::result::Result<ListToolsResult, rmcp::ErrorData>> + Send + '_ {
        async move { Ok(ListToolsResult::with_all_items(tool_specs())) }
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        tool_specs().into_iter().find(|tool| tool.name.as_ref() == name)
    }
}

fn parse_arguments<T: DeserializeOwned>(
    arguments: Option<rmcp::model::JsonObject>,
) -> std::result::Result<T, String> {
    serde_json::from_value(serde_json::Value::Object(arguments.unwrap_or_default()))
        .map_err(|error| format!("failed to deserialize parameters: {error}"))
}

fn tool_specs() -> Vec<Tool> {
    vec![
        Tool::new(
            "git_lore_context",
            "Return scope-aware lore context for a file and optional cursor line.",
            rmcp::handler::server::tool::schema_for_type::<ContextToolArgs>(),
        ),
        Tool::new(
            "git_lore_propose",
            "Record a proposed lore atom for a file and optional cursor line. Requires state_checksum and snapshot_generated_unix_seconds from git_lore_state_snapshot.",
            rmcp::handler::server::tool::schema_for_type::<ProposeToolArgs>(),
        ),
        Tool::new(
            "git_lore_state_snapshot",
            "Return current workspace lore state metadata for state-first workflows.",
            rmcp::handler::server::tool::schema_for_type::<StateSnapshotToolArgs>(),
        ),
        Tool::new(
            "git_lore_memory_preflight",
            "Run memory safety checks before write operations. operation supports: edit, commit, sync.",
            rmcp::handler::server::tool::schema_for_type::<MemoryPreflightToolArgs>(),
        ),
        Tool::new(
            "git_lore_memory_search",
            "Search local lore memory with hybrid ranking (lexical, recency, state, and path/scope proximity).",
            rmcp::handler::server::tool::schema_for_type::<MemorySearchToolArgs>(),
        ),
        Tool::new(
            "git_lore_state_transition_preview",
            "Preview whether an atom state transition is allowed before applying it.",
            rmcp::handler::server::tool::schema_for_type::<StateTransitionPreviewToolArgs>(),
        ),
    ]
}

fn tool_json_response<T: Serialize>(value: &T) -> CallToolResult {
    match serde_json::to_string_pretty(value) {
        Ok(text) => CallToolResult::success(vec![Content::text(text)]),
        Err(error) => tool_error_with_code(
            "serialization_failed",
            format!("failed to serialize tool response: {error}"),
            false,
            None,
        ),
    }
}

fn tool_error_with_code(
    code: impl AsRef<str>,
    message: impl Into<String>,
    retryable: bool,
    recommended_action: Option<&str>,
) -> CallToolResult {
    let payload = ToolErrorPayload {
        code: code.as_ref().to_string(),
        message: message.into(),
        retryable,
        recommended_action: recommended_action.map(str::to_string),
    };

    let text = serde_json::to_string_pretty(&payload)
        .unwrap_or_else(|_| format!("{{\"code\":\"{}\",\"message\":\"tool error\"}}", payload.code));
    CallToolResult::error(vec![Content::text(text)])
}

fn parse_kind(value: &str) -> std::result::Result<LoreKind, String> {
    match value {
        "decision" => Ok(LoreKind::Decision),
        "assumption" => Ok(LoreKind::Assumption),
        "open_question" => Ok(LoreKind::OpenQuestion),
        "signal" => Ok(LoreKind::Signal),
        other => Err(format!("unsupported lore kind: {other}")),
    }
}

fn parse_operation(value: &str) -> std::result::Result<&str, String> {
    match value {
        "edit" | "commit" | "sync" => Ok(value),
        other => Err(format!(
            "unsupported preflight operation: {other} (expected edit, commit, or sync)"
        )),
    }
}

fn parse_atom_state(value: &str) -> std::result::Result<AtomState, String> {
    match value {
        "draft" => Ok(AtomState::Draft),
        "proposed" => Ok(AtomState::Proposed),
        "accepted" => Ok(AtomState::Accepted),
        "deprecated" => Ok(AtomState::Deprecated),
        other => Err(format!("unsupported atom state: {other}")),
    }
}

fn validate_snapshot_freshness(
    snapshot_generated_unix_seconds: u64,
    now_unix_seconds: u64,
) -> std::result::Result<(), String> {
    if snapshot_generated_unix_seconds > now_unix_seconds + STATE_GUARD_CLOCK_SKEW_SECONDS {
        return Err("snapshot timestamp is in the future".to_string());
    }

    let age = now_unix_seconds.saturating_sub(snapshot_generated_unix_seconds);
    if age > STATE_GUARD_TTL_SECONDS {
        return Err(format!(
            "snapshot is stale (age {}s, ttl {}s)",
            age, STATE_GUARD_TTL_SECONDS
        ));
    }

    Ok(())
}

fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::{
        parse_atom_state, parse_kind, parse_operation, validate_snapshot_freshness,
        ProposeToolArgs,
        STATE_GUARD_CLOCK_SKEW_SECONDS, STATE_GUARD_TTL_SECONDS,
    };
    use crate::lore::{AtomState, LoreKind};

    #[test]
    fn parse_kind_accepts_supported_values() {
        assert!(matches!(parse_kind("decision"), Ok(LoreKind::Decision)));
        assert!(matches!(parse_kind("assumption"), Ok(LoreKind::Assumption)));
        assert!(matches!(parse_kind("open_question"), Ok(LoreKind::OpenQuestion)));
        assert!(matches!(parse_kind("signal"), Ok(LoreKind::Signal)));
    }

    #[test]
    fn parse_kind_rejects_unknown_values() {
        assert!(parse_kind("unsupported").is_err());
    }

    #[test]
    fn parse_operation_accepts_supported_values() {
        assert!(matches!(parse_operation("edit"), Ok("edit")));
        assert!(matches!(parse_operation("commit"), Ok("commit")));
        assert!(matches!(parse_operation("sync"), Ok("sync")));
    }

    #[test]
    fn parse_operation_rejects_unknown_values() {
        assert!(parse_operation("deploy").is_err());
    }

    #[test]
    fn propose_args_require_state_guard_fields() {
        let value = serde_json::json!({
            "file_path": "src/lib.rs",
            "title": "test"
        });

        let parsed = serde_json::from_value::<ProposeToolArgs>(value);
        assert!(parsed.is_err());
    }

    #[test]
    fn parse_atom_state_accepts_supported_values() {
        assert!(matches!(parse_atom_state("draft"), Ok(AtomState::Draft)));
        assert!(matches!(parse_atom_state("proposed"), Ok(AtomState::Proposed)));
        assert!(matches!(parse_atom_state("accepted"), Ok(AtomState::Accepted)));
        assert!(matches!(parse_atom_state("deprecated"), Ok(AtomState::Deprecated)));
    }

    #[test]
    fn parse_atom_state_rejects_unknown_values() {
        assert!(parse_atom_state("archived").is_err());
    }

    #[test]
    fn snapshot_freshness_accepts_recent_timestamps() {
        let now = 10_000;
        assert!(validate_snapshot_freshness(now, now).is_ok());
        assert!(validate_snapshot_freshness(now - STATE_GUARD_TTL_SECONDS, now).is_ok());
    }

    #[test]
    fn snapshot_freshness_rejects_stale_timestamps() {
        let now = 10_000;
        assert!(validate_snapshot_freshness(now - STATE_GUARD_TTL_SECONDS - 1, now).is_err());
    }

    #[test]
    fn snapshot_freshness_rejects_far_future_timestamps() {
        let now = 10_000;
        assert!(validate_snapshot_freshness(now + STATE_GUARD_CLOCK_SKEW_SECONDS + 1, now).is_err());
    }
}
