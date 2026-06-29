//! Built-in stdio MCP server that controls the local Warp app through local-control.
use local_control::discovery::{self, InstanceId};
use local_control::protocol::{
    Action, ActionKind, BlockQueryParams, BlockReadParams, ControlError, ControlResponse,
    Direction, DirectionParams, EmptyParams, ErrorCode, KeySequenceParams, RequestEnvelope,
    TabCreateParams, TabType, TargetSelector,
};
use local_control::selection::{InstanceSelector, select_instance};
use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use warp_core::channel::ChannelState;

pub const MCP_SERVER_MODE_FLAG: &str = "--warp-mcp-server";
pub const DEFAULT_INSTANCE_ID_ENV: &str = "WARP_MCP_DEFAULT_INSTANCE_ID";
pub const DEFAULT_PID_ENV: &str = "WARP_MCP_DEFAULT_PID";

#[derive(Debug, Clone)]
pub struct WarpMcpServer {
    tool_router: ToolRouter<Self>,
}

impl WarpMcpServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

impl Default for WarpMcpServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router(router = tool_router)]
impl WarpMcpServer {
    #[tool(
        name = "warp_control",
        description = "Run any implemented Warp local-control action by canonical name, such as window.list, tab.create, pane.split, block.read, or input.send_keys."
    )]
    pub async fn warp_control(
        &self,
        Parameters(params): Parameters<WarpControlParams>,
    ) -> CallToolResult {
        let result = async {
            let action = ActionKind::from_name(&params.action).ok_or_else(|| {
                ControlError::new(
                    ErrorCode::UnsupportedAction,
                    format!("unknown Warp control action `{}`", params.action),
                )
            })?;
            if !action.is_implemented() {
                return Err(ControlError::new(
                    ErrorCode::UnsupportedAction,
                    format!("{} is not implemented", action.as_str()),
                ));
            }
            call_action(
                action,
                params.params.unwrap_or_else(|| json!({})),
                params.target,
                params.instance_id,
                params.pid,
            )
            .await
        }
        .await;
        tool_result(result)
    }

    #[tool(
        name = "list_actions",
        description = "List implemented Warp control actions."
    )]
    pub async fn list_actions(
        &self,
        Parameters(params): Parameters<InstanceParams>,
    ) -> CallToolResult {
        tool_result(
            call_action_with_params(
                ActionKind::ActionList,
                EmptyParams {},
                None,
                params.instance_id,
                params.pid,
            )
            .await,
        )
    }

    #[tool(
        name = "app_active",
        description = "Return the active Warp window/tab/pane/session chain."
    )]
    pub async fn app_active(
        &self,
        Parameters(params): Parameters<TargetOnlyParams>,
    ) -> CallToolResult {
        tool_result(
            call_action_with_params(
                ActionKind::AppActive,
                EmptyParams {},
                params.target,
                params.instance_id,
                params.pid,
            )
            .await,
        )
    }

    #[tool(
        name = "active_context",
        description = "Return the active Warp context plus windows, tabs, panes, and sessions for resolving targets before acting."
    )]
    pub async fn active_context(
        &self,
        Parameters(params): Parameters<TargetOnlyParams>,
    ) -> CallToolResult {
        let result = async {
            let active = call_action_with_params(
                ActionKind::AppActive,
                EmptyParams {},
                params.target.clone(),
                params.instance_id.clone(),
                params.pid,
            )
            .await?;
            let windows = call_action_with_params(
                ActionKind::WindowList,
                EmptyParams {},
                params.target.clone(),
                params.instance_id.clone(),
                params.pid,
            )
            .await?;
            let tabs = call_action_with_params(
                ActionKind::TabList,
                EmptyParams {},
                params.target.clone(),
                params.instance_id.clone(),
                params.pid,
            )
            .await?;
            let panes = call_action_with_params(
                ActionKind::PaneList,
                EmptyParams {},
                params.target.clone(),
                params.instance_id.clone(),
                params.pid,
            )
            .await?;
            let sessions = call_action_with_params(
                ActionKind::SessionList,
                EmptyParams {},
                params.target,
                params.instance_id,
                params.pid,
            )
            .await?;
            Ok(json!({
                "active": active,
                "windows": windows,
                "tabs": tabs,
                "panes": panes,
                "sessions": sessions,
            }))
        }
        .await;
        tool_result(result)
    }

    #[tool(name = "list_windows", description = "List Warp windows.")]
    pub async fn list_windows(
        &self,
        Parameters(params): Parameters<TargetOnlyParams>,
    ) -> CallToolResult {
        tool_result(
            call_action_with_params(
                ActionKind::WindowList,
                EmptyParams {},
                params.target,
                params.instance_id,
                params.pid,
            )
            .await,
        )
    }

    #[tool(name = "list_tabs", description = "List tabs in a Warp window.")]
    pub async fn list_tabs(
        &self,
        Parameters(params): Parameters<TargetOnlyParams>,
    ) -> CallToolResult {
        tool_result(
            call_action_with_params(
                ActionKind::TabList,
                EmptyParams {},
                params.target,
                params.instance_id,
                params.pid,
            )
            .await,
        )
    }

    #[tool(name = "list_panes", description = "List panes in a Warp tab.")]
    pub async fn list_panes(
        &self,
        Parameters(params): Parameters<TargetOnlyParams>,
    ) -> CallToolResult {
        tool_result(
            call_action_with_params(
                ActionKind::PaneList,
                EmptyParams {},
                params.target,
                params.instance_id,
                params.pid,
            )
            .await,
        )
    }

    #[tool(
        name = "list_sessions",
        description = "List terminal sessions in a Warp tab."
    )]
    pub async fn list_sessions(
        &self,
        Parameters(params): Parameters<TargetOnlyParams>,
    ) -> CallToolResult {
        tool_result(
            call_action_with_params(
                ActionKind::SessionList,
                EmptyParams {},
                params.target,
                params.instance_id,
                params.pid,
            )
            .await,
        )
    }

    #[tool(name = "create_window", description = "Create a new Warp window.")]
    pub async fn create_window(
        &self,
        Parameters(params): Parameters<CreateTabParams>,
    ) -> CallToolResult {
        let result = async {
            let tab_params = tab_create_params(&params)?;
            let params_value = serde_json::to_value(tab_params).map_err(serialize_error)?;
            call_action(
                ActionKind::WindowCreate,
                params_value,
                params.target,
                params.instance_id,
                params.pid,
            )
            .await
        }
        .await;
        tool_result(result)
    }

    #[tool(name = "create_tab", description = "Create a new Warp tab.")]
    pub async fn create_tab(
        &self,
        Parameters(params): Parameters<CreateTabParams>,
    ) -> CallToolResult {
        let result = async {
            let tab_params = tab_create_params(&params)?;
            let params_value = serde_json::to_value(tab_params).map_err(serialize_error)?;
            call_action(
                ActionKind::TabCreate,
                params_value,
                params.target,
                params.instance_id,
                params.pid,
            )
            .await
        }
        .await;
        tool_result(result)
    }

    #[tool(
        name = "split_pane",
        description = "Split the active or selected Warp pane."
    )]
    pub async fn split_pane(
        &self,
        Parameters(params): Parameters<DirectionToolParams>,
    ) -> CallToolResult {
        let result = async {
            let direction_params = direction_params(&params)?;
            let params_value = serde_json::to_value(direction_params).map_err(serialize_error)?;
            call_action(
                ActionKind::PaneSplit,
                params_value,
                params.target,
                params.instance_id,
                params.pid,
            )
            .await
        }
        .await;
        tool_result(result)
    }

    #[tool(name = "focus_pane", description = "Focus the selected Warp pane.")]
    pub async fn focus_pane(
        &self,
        Parameters(params): Parameters<TargetOnlyParams>,
    ) -> CallToolResult {
        tool_result(
            call_action_with_params(
                ActionKind::PaneFocus,
                EmptyParams {},
                params.target,
                params.instance_id,
                params.pid,
            )
            .await,
        )
    }

    #[tool(
        name = "send_keys",
        description = "Send text and terminal key sequence entries to a Warp terminal session."
    )]
    pub async fn send_keys(
        &self,
        Parameters(params): Parameters<SendKeysParams>,
    ) -> CallToolResult {
        tool_result(
            call_action_with_params(
                ActionKind::InputSendKeys,
                KeySequenceParams {
                    keys: params.keys,
                    text: params.text,
                },
                params.target,
                params.instance_id,
                params.pid,
            )
            .await,
        )
    }

    #[tool(
        name = "list_blocks",
        description = "List recent terminal blocks in a Warp terminal session."
    )]
    pub async fn list_blocks(
        &self,
        Parameters(params): Parameters<ListBlocksParams>,
    ) -> CallToolResult {
        tool_result(
            call_action_with_params(
                ActionKind::BlockList,
                BlockQueryParams {
                    limit: params.limit,
                    include_hidden: params.include_hidden.unwrap_or(false),
                    include_output: params.include_output.unwrap_or(false),
                    max_output_chars: params.max_output_chars,
                },
                params.target,
                params.instance_id,
                params.pid,
            )
            .await,
        )
    }

    #[tool(
        name = "read_block",
        description = "Read a terminal block, defaulting to the latest visible block."
    )]
    pub async fn read_block(
        &self,
        Parameters(params): Parameters<ReadBlockParams>,
    ) -> CallToolResult {
        tool_result(
            call_action_with_params(
                ActionKind::BlockRead,
                BlockReadParams {
                    block_id: params.block_id,
                    index: params.index,
                    include_output: params.include_output.unwrap_or(true),
                    max_output_chars: params.max_output_chars,
                },
                params.target,
                params.instance_id,
                params.pid,
            )
            .await,
        )
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for WarpMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions("Control the running local Warp app through Warp local-control. Start pane-targeted tasks with active_context so the active tab, panes, and sessions are visible before acting. Use returned ids/selectors with the focused helper tools or warp_control. Do not fall back to shell process discovery, osascript, or GUI automation; if Warp MCP returns an error, report that error.")
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct InstanceParams {
    #[serde(default)]
    pub instance_id: Option<String>,
    #[serde(default)]
    pub pid: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TargetOnlyParams {
    #[serde(default)]
    pub instance_id: Option<String>,
    #[serde(default)]
    pub pid: Option<u32>,
    #[serde(default)]
    pub target: Option<Value>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WarpControlParams {
    pub action: String,
    #[serde(default)]
    pub params: Option<Value>,
    #[serde(default)]
    pub target: Option<Value>,
    #[serde(default)]
    pub instance_id: Option<String>,
    #[serde(default)]
    pub pid: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateTabParams {
    #[serde(default)]
    pub tab_type: Option<McpTabType>,
    #[serde(default)]
    pub shell: Option<String>,
    #[serde(default)]
    pub target: Option<Value>,
    #[serde(default)]
    pub instance_id: Option<String>,
    #[serde(default)]
    pub pid: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum McpTabType {
    Terminal,
    Agent,
    CloudAgent,
    Default,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DirectionToolParams {
    pub direction: String,
    #[serde(default)]
    pub target: Option<Value>,
    #[serde(default)]
    pub instance_id: Option<String>,
    #[serde(default)]
    pub pid: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SendKeysParams {
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub keys: Vec<String>,
    #[serde(default)]
    pub target: Option<Value>,
    #[serde(default)]
    pub instance_id: Option<String>,
    #[serde(default)]
    pub pid: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListBlocksParams {
    #[serde(default)]
    pub limit: Option<u32>,
    #[serde(default)]
    pub include_hidden: Option<bool>,
    #[serde(default)]
    pub include_output: Option<bool>,
    #[serde(default)]
    pub max_output_chars: Option<usize>,
    #[serde(default)]
    pub target: Option<Value>,
    #[serde(default)]
    pub instance_id: Option<String>,
    #[serde(default)]
    pub pid: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReadBlockParams {
    #[serde(default)]
    pub block_id: Option<String>,
    #[serde(default)]
    pub index: Option<u32>,
    #[serde(default)]
    pub include_output: Option<bool>,
    #[serde(default)]
    pub max_output_chars: Option<usize>,
    #[serde(default)]
    pub target: Option<Value>,
    #[serde(default)]
    pub instance_id: Option<String>,
    #[serde(default)]
    pub pid: Option<u32>,
}

fn tool_result(result: Result<Value, ControlError>) -> CallToolResult {
    match result {
        Ok(data) => CallToolResult::structured(json!({
            "ok": true,
            "data": data,
        })),
        Err(error) => CallToolResult::structured_error(json!({
            "ok": false,
            "error": error,
        })),
    }
}

async fn call_action_with_params<T: serde::Serialize + Send + 'static>(
    action: ActionKind,
    params: T,
    target: Option<Value>,
    instance_id: Option<String>,
    pid: Option<u32>,
) -> Result<Value, ControlError> {
    let params = serde_json::to_value(params).map_err(serialize_error)?;
    call_action(action, params, target, instance_id, pid).await
}

async fn call_action(
    action: ActionKind,
    params: Value,
    target: Option<Value>,
    instance_id: Option<String>,
    pid: Option<u32>,
) -> Result<Value, ControlError> {
    let target = parse_target(target)?;
    tokio::task::spawn_blocking(move || {
        call_action_blocking(action, params, target, instance_id, pid)
    })
    .await
    .map_err(|err| {
        ControlError::with_details(
            ErrorCode::Internal,
            "Warp MCP local-control worker failed",
            err.to_string(),
        )
    })?
}

fn call_action_blocking(
    action: ActionKind,
    params: Value,
    target: TargetSelector,
    instance_id: Option<String>,
    pid: Option<u32>,
) -> Result<Value, ControlError> {
    let selector = instance_selector(instance_id, pid)?;
    let records = discovery::list_instances(&ChannelState::channel().to_string());
    let instance = select_instance(&records, &selector)?;
    let mut request = RequestEnvelope::new(Action {
        kind: action,
        params,
    });
    request.target = target;
    let response = local_control::client::send_request(&instance, &request)?;
    match response.response {
        ControlResponse::Ok { data } => Ok(data),
        ControlResponse::Error { error } => Err(error),
    }
}

fn instance_selector(
    instance_id: Option<String>,
    pid: Option<u32>,
) -> Result<InstanceSelector, ControlError> {
    instance_selector_with_defaults(
        instance_id,
        pid,
        default_instance_id_from_env(),
        default_pid_from_env()?,
    )
}

fn instance_selector_with_defaults(
    instance_id: Option<String>,
    pid: Option<u32>,
    default_instance_id: Option<String>,
    default_pid: Option<u32>,
) -> Result<InstanceSelector, ControlError> {
    match (instance_id, pid) {
        (Some(_), Some(_)) => Err(ControlError::new(
            ErrorCode::InvalidSelector,
            "instance_id and pid cannot both be set",
        )),
        (Some(instance_id), None) => Ok(InstanceSelector::Id(InstanceId(instance_id))),
        (None, Some(pid)) => Ok(InstanceSelector::Pid(pid)),
        (None, None) => match (default_instance_id, default_pid) {
            (Some(_), Some(_)) => Err(ControlError::new(
                ErrorCode::InvalidSelector,
                format!("{DEFAULT_INSTANCE_ID_ENV} and {DEFAULT_PID_ENV} cannot both be set"),
            )),
            (Some(instance_id), None) => Ok(InstanceSelector::Id(InstanceId(instance_id))),
            (None, Some(pid)) => Ok(InstanceSelector::Pid(pid)),
            (None, None) => Ok(InstanceSelector::Active),
        },
    }
}

fn default_instance_id_from_env() -> Option<String> {
    std::env::var(DEFAULT_INSTANCE_ID_ENV)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn default_pid_from_env() -> Result<Option<u32>, ControlError> {
    let Some(value) = std::env::var(DEFAULT_PID_ENV)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };
    value.parse::<u32>().map(Some).map_err(|err| {
        ControlError::with_details(
            ErrorCode::InvalidSelector,
            format!("{DEFAULT_PID_ENV} must be a process id"),
            err.to_string(),
        )
    })
}

fn parse_target(target: Option<Value>) -> Result<TargetSelector, ControlError> {
    match target {
        None | Some(Value::Null) => Ok(TargetSelector::default()),
        Some(target) => serde_json::from_value(target).map_err(|err| {
            ControlError::with_details(
                ErrorCode::InvalidSelector,
                "failed to decode Warp target selector",
                err.to_string(),
            )
        }),
    }
}

fn tab_create_params(params: &CreateTabParams) -> Result<TabCreateParams, ControlError> {
    Ok(TabCreateParams {
        tab_type: params.tab_type.as_ref().map(|tab_type| match tab_type {
            McpTabType::Terminal => TabType::Terminal,
            McpTabType::Agent => TabType::Agent,
            McpTabType::CloudAgent => TabType::CloudAgent,
            McpTabType::Default => TabType::Default,
        }),
        shell: params.shell.clone(),
    })
}

fn direction_params(params: &DirectionToolParams) -> Result<DirectionParams, ControlError> {
    let direction = match params.direction.as_str() {
        "left" => Direction::Left,
        "right" => Direction::Right,
        "up" => Direction::Up,
        "down" => Direction::Down,
        "previous" => Direction::Previous,
        "next" => Direction::Next,
        _ => {
            return Err(ControlError::new(
                ErrorCode::InvalidParams,
                "direction must be left, right, up, down, previous, or next",
            ));
        }
    };
    Ok(DirectionParams { direction })
}

fn serialize_error(err: serde_json::Error) -> ControlError {
    ControlError::with_details(
        ErrorCode::InvalidParams,
        "failed to serialize Warp control parameters",
        err.to_string(),
    )
}

pub fn run_stdio_blocking() -> anyhow::Result<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async {
        let service = WarpMcpServer::new()
            .serve(rmcp::transport::stdio())
            .await
            .map_err(|err| anyhow::anyhow!("failed to start Warp MCP server: {err}"))?;
        service
            .waiting()
            .await
            .map_err(|err| anyhow::anyhow!("Warp MCP server task failed: {err}"))?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_instance_selector_rejects_conflicting_fields() {
        let err = instance_selector_with_defaults(Some("inst_1".to_owned()), Some(123), None, None)
            .expect_err("explicit instance_id and pid should conflict");

        assert_eq!(err.code, ErrorCode::InvalidSelector);
    }

    #[test]
    fn instance_selector_uses_default_pid_when_no_explicit_selector() {
        let selector = instance_selector_with_defaults(None, None, None, Some(123))
            .expect("default pid should select pid");

        assert_eq!(selector, InstanceSelector::Pid(123));
    }

    #[test]
    fn explicit_selector_overrides_default_pid() {
        let selector =
            instance_selector_with_defaults(Some("inst_1".to_owned()), None, None, Some(123))
                .expect("explicit instance id should win");

        assert_eq!(
            selector,
            InstanceSelector::Id(InstanceId("inst_1".to_owned()))
        );
    }

    #[test]
    fn default_selector_rejects_conflicting_env_defaults() {
        let err = instance_selector_with_defaults(None, None, Some("inst_1".to_owned()), Some(123))
            .expect_err("default instance id and pid should conflict");

        assert_eq!(err.code, ErrorCode::InvalidSelector);
    }
}
