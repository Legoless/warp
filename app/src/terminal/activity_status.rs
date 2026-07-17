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

/// Client-owned idle backstop for a *rich* CLI agent (e.g. Claude Code) whose
/// tracked status is still `InProgress`. A rich session leaves `InProgress` only
/// on a `stop`/`idle_prompt` event, but those notifications are best-effort — if
/// they are lost the status latch would pin the pane at "working" forever with
/// no recovery. This is the fallback that needs no plugin event: Claude's TUI
/// animates (spinner, elapsed timer, streamed tokens) at least every second
/// while a turn is active, so once the PTY has been quiet this long the agent
/// has returned to an idle prompt and the pane goes idle. It is far longer than
/// `CLI_AGENT_OUTPUT_QUIET_MS` (the non-rich window) so a genuinely-working
/// agent is never flipped idle during a brief render gap.
pub(crate) const CLI_AGENT_RICH_IDLE_QUIET_MS: u128 = 5_000;

pub(crate) struct TerminalActivityStatusInputs<'a> {
    pub(crate) cli_session: Option<&'a CLIAgentSession>,
    pub(crate) has_terminal_conversation: bool,
    pub(crate) is_ambient: bool,
    pub(crate) selected_conversation_status: Option<ConversationStatus>,
    pub(crate) has_active_cli_agent_command: bool,
    /// Milliseconds since the pane's PTY last produced output, or `None` if it
    /// has produced none. Drives two idle checks: `CLI_AGENT_OUTPUT_QUIET_MS`
    /// for a non-rich CLI agent (e.g. Codex, which has no reliable
    /// "started/stopped working" event), and `CLI_AGENT_RICH_IDLE_QUIET_MS` as
    /// the backstop that releases a rich agent's stuck `InProgress` latch when
    /// its `stop`/`idle_prompt` event was lost.
    pub(crate) millis_since_last_output: Option<u128>,
    pub(crate) has_active_conversation: bool,
    pub(crate) is_long_running: bool,
    pub(crate) agent_icon_status: Option<ConversationStatus>,
}

pub(crate) fn terminal_activity_status_from_inputs(
    inputs: TerminalActivityStatusInputs<'_>,
) -> Option<ConversationStatus> {
    let cli_agent_output_active = inputs
        .millis_since_last_output
        .is_some_and(|ms| ms < CLI_AGENT_OUTPUT_QUIET_MS);

    // A non-rich CLI agent (e.g. Codex) can drive command-presence activity, but
    // only when gated on real output activity below. Its session status is an
    // unreliable one-way latch (it sticks at `InProgress` whenever the OSC9
    // `Stop` notification is missing), so it must NOT gate coloring on its own —
    // doing so paints an idle Codex red. Output activity is the reliable signal.
    let command_detected_cli_session_can_drive_activity = inputs
        .cli_session
        .filter(|session| !matches!(session.agent, CLIAgent::Unknown))
        .is_none_or(|session| {
            matches!(session.status, CLIAgentSessionStatus::InProgress)
                && (!session.supports_rich_status() || matches!(session.agent, CLIAgent::Codex))
        });

    if let Some(session) = inputs
        .cli_session
        .filter(|session| !matches!(session.agent, CLIAgent::Unknown))
        .filter(|session| cli_agent_session_drives_activity_status(session))
    {
        if let Some(status) = activity_status_for_cli_agent_session(session) {
            // Rich-agent idle backstop: a rich session's `InProgress` is a
            // one-way latch cleared only by a best-effort `stop`/`idle_prompt`
            // event. If those are lost the pane would stay "working" forever, so
            // fall back to PTY quiet — a gap longer than the rich window means
            // the agent has returned to an idle prompt. Only `InProgress` is
            // gated this way; `Blocked`/`Success` are authoritative and pass
            // through unchanged.
            let idle_despite_latch = matches!(status, ConversationStatus::InProgress)
                && inputs
                    .millis_since_last_output
                    .is_some_and(|ms| ms >= CLI_AGENT_RICH_IDLE_QUIET_MS);
            if !idle_despite_latch {
                return Some(status);
            }
        }
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
        && cli_agent_output_active
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
