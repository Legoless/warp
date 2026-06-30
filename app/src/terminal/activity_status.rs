use crate::ai::agent::conversation::ConversationStatus;
use crate::settings::PaneActivityState;
use crate::terminal::cli_agent_sessions::{CLIAgentSession, CLIAgentSessionStatus};
use crate::terminal::CLIAgent;

/// How long after the most recent PTY output a non-rich CLI agent (e.g. Codex)
/// is still considered "working" for pane-activity coloring. Output gaps shorter
/// than this — spinner frames, streamed tokens — keep the pane red; once output
/// stays quiet longer than this the agent is treated as idle and the pane goes
/// black.
pub(crate) const CLI_AGENT_OUTPUT_QUIET_MS: u128 = 800;

pub(crate) struct TerminalActivityStatusInputs<'a> {
    pub(crate) cli_session: Option<&'a CLIAgentSession>,
    pub(crate) has_terminal_conversation: bool,
    pub(crate) is_ambient: bool,
    pub(crate) selected_conversation_status: Option<ConversationStatus>,
    pub(crate) has_active_cli_agent_command: bool,
    /// Whether the pane's PTY produced output recently (within
    /// `CLI_AGENT_OUTPUT_QUIET_MS`). For a non-rich CLI agent (e.g. Codex) this
    /// is the signal that it is actively working rather than idling at its
    /// prompt — Codex has no reliable "started/stopped working" status event.
    pub(crate) cli_agent_output_active: bool,
    pub(crate) has_active_conversation: bool,
    pub(crate) is_long_running: bool,
    pub(crate) agent_icon_status: Option<ConversationStatus>,
}

pub(crate) fn terminal_activity_status_from_inputs(
    inputs: TerminalActivityStatusInputs<'_>,
) -> Option<ConversationStatus> {
    // A non-rich CLI agent (e.g. Codex) can drive command-presence activity, but
    // only when gated on real output activity below. Its session status is an
    // unreliable one-way latch (it sticks at `InProgress` whenever the OSC9
    // `Stop` notification is missing), so it must NOT gate coloring on its own —
    // doing so paints an idle Codex red. Output activity is the reliable signal.
    let command_detected_cli_session_can_drive_activity = inputs
        .cli_session
        .filter(|session| !matches!(session.agent, CLIAgent::Unknown))
        .map_or(true, |session| {
            matches!(session.status, CLIAgentSessionStatus::InProgress)
                && (!session.supports_rich_status() || matches!(session.agent, CLIAgent::Codex))
        });

    if let Some(status) = inputs
        .cli_session
        .filter(|session| !matches!(session.agent, CLIAgent::Unknown))
        .filter(|session| cli_agent_session_drives_activity_status(session))
        .and_then(activity_status_for_cli_agent_session)
    {
        return Some(status);
    }

    if let Some(status) = activity_status_from_terminal_conversation_status(
        inputs.has_terminal_conversation,
        inputs.is_ambient,
        inputs.selected_conversation_status,
    ) {
        return Some(status);
    }

    if command_detected_cli_session_can_drive_activity
        && inputs.has_active_cli_agent_command
        && inputs.cli_agent_output_active
    {
        return Some(ConversationStatus::InProgress);
    }

    if let Some(status) =
        activity_status_from_tab_progress(inputs.has_active_conversation, inputs.is_long_running)
    {
        return Some(status);
    }

    if inputs
        .cli_session
        .is_some_and(|session| !matches!(session.agent, CLIAgent::Unknown))
    {
        return None;
    }

    inputs.agent_icon_status
}

pub(crate) fn pane_activity_state_for_status(
    status: Option<&ConversationStatus>,
) -> PaneActivityState {
    match status {
        Some(ConversationStatus::InProgress) => PaneActivityState::Working,
        Some(ConversationStatus::Blocked { .. }) => PaneActivityState::RequiresAttention,
        Some(ConversationStatus::Error | ConversationStatus::TransientError) => {
            PaneActivityState::NeedsHelp
        }
        Some(
            ConversationStatus::Success
            | ConversationStatus::Cancelled
            | ConversationStatus::WaitingForEvents,
        )
        | None => PaneActivityState::NotWorking,
    }
}

pub(crate) fn activity_status_from_tab_progress(
    has_active_conversation: bool,
    is_long_running: bool,
) -> Option<ConversationStatus> {
    if has_active_conversation && is_long_running {
        Some(ConversationStatus::InProgress)
    } else {
        None
    }
}

pub(crate) fn activity_status_from_terminal_conversation_status(
    has_conversation: bool,
    is_ambient: bool,
    selected_conversation_status: Option<ConversationStatus>,
) -> Option<ConversationStatus> {
    if has_conversation || is_ambient {
        selected_conversation_status
    } else {
        None
    }
}

pub(crate) fn activity_status_for_cli_agent_session(
    session: &CLIAgentSession,
) -> Option<ConversationStatus> {
    if matches!(session.agent, CLIAgent::Claude | CLIAgent::Codex)
        && matches!(session.status, CLIAgentSessionStatus::InProgress)
        && session.session_context.latest_user_prompt().is_none()
    {
        return None;
    }

    Some(session.status.to_conversation_status())
}

pub(crate) fn cli_agent_session_drives_activity_status(session: &CLIAgentSession) -> bool {
    session.supports_rich_status()
}

#[cfg(test)]
#[path = "activity_status_tests.rs"]
mod tests;
