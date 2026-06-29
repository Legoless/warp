//! Permission checks for local control.
use ::local_control::{ActionKind, ControlError, ErrorCode, PROTOCOL_VERSION};
use warpui::{ModelContext, SingletonEntity};

use crate::features::FeatureFlag;
use crate::local_control::LocalControlBridge;
use crate::settings::{AISettings, LocalControlSettings};

pub(super) fn warp_control_cli_enabled() -> bool {
    FeatureFlag::WarpControlCli.is_enabled()
}

pub(super) fn ensure_protocol_version(protocol_version: u32) -> Result<(), ControlError> {
    if protocol_version == PROTOCOL_VERSION {
        return Ok(());
    }
    Err(ControlError::new(
        ErrorCode::ProtocolVersionUnsupported,
        format!("unsupported protocol version {protocol_version}"),
    ))
}

#[cfg(test)]
pub(crate) fn ensure_feature_enabled() -> Result<(), ControlError> {
    if warp_control_cli_enabled() {
        return Ok(());
    }
    Err(ControlError::new(
        ErrorCode::LocalControlDisabled,
        "Warp control CLI is disabled by feature flag",
    ))
}

pub(super) fn ensure_control_runtime_enabled(
    ctx: &mut ModelContext<LocalControlBridge>,
) -> Result<(), ControlError> {
    if warp_control_cli_enabled() || AISettings::as_ref(ctx).is_warp_control_mcp_server_enabled(ctx)
    {
        return Ok(());
    }
    Err(ControlError::new(
        ErrorCode::LocalControlDisabled,
        "Warp control is disabled",
    ))
}

#[cfg(test)]
pub(crate) fn capabilities() -> Vec<ActionKind> {
    ActionKind::implemented_metadata()
        .into_iter()
        .map(|metadata| metadata.kind)
        .collect()
}

pub(crate) fn ensure_action_allowed(
    action: ActionKind,
    ctx: &mut ModelContext<LocalControlBridge>,
) -> Result<(), ControlError> {
    let settings = LocalControlSettings::as_ref(ctx);
    ensure_control_access_allowed(
        settings.is_enabled(),
        AISettings::as_ref(ctx).is_warp_control_mcp_server_enabled(ctx),
        action,
    )
}

#[cfg(test)]
pub(crate) fn ensure_settings_allow_action(
    settings: &LocalControlSettings,
    action: ActionKind,
) -> Result<(), ControlError> {
    ensure_control_access_allowed(settings.is_enabled(), false, action)
}

fn ensure_control_access_allowed(
    local_control_enabled: bool,
    warp_control_mcp_enabled: bool,
    action: ActionKind,
) -> Result<(), ControlError> {
    if !local_control_enabled && !warp_control_mcp_enabled {
        return Err(ControlError::new(
            ErrorCode::LocalControlDisabled,
            format!(
                "{} is disabled for local control and the built-in Warp MCP server",
                action.as_str()
            ),
        ));
    }
    Ok(())
}
