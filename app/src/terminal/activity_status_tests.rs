use super::*;
use crate::terminal::cli_agent_sessions::{CLIAgentInputState, CLIAgentSessionContext};

#[test]
fn pane_activity_state_treats_missing_status_as_not_working() {
    assert_eq!(
        pane_activity_state_for_status(None),
        PaneActivityState::NotWorking
    );
}

#[test]
fn pane_activity_state_maps_in_progress_to_working() {
    assert_eq!(
        pane_activity_state_for_status(Some(&ConversationStatus::InProgress)),
        PaneActivityState::Working
    );
}

#[test]
fn pane_activity_state_maps_blocked_to_requires_attention() {
    assert_eq!(
        pane_activity_state_for_status(Some(&ConversationStatus::Blocked {
            blocked_action: "confirm command".to_owned(),
        })),
        PaneActivityState::RequiresAttention
    );
}

#[test]
fn pane_activity_state_maps_errors_to_needs_help() {
    assert_eq!(
        pane_activity_state_for_status(Some(&ConversationStatus::Error)),
        PaneActivityState::NeedsHelp
    );
    assert_eq!(
        pane_activity_state_for_status(Some(&ConversationStatus::TransientError)),
        PaneActivityState::NeedsHelp
    );
}

#[test]
fn pane_activity_state_maps_finished_and_waiting_to_not_working() {
    for status in [
        ConversationStatus::Success,
        ConversationStatus::Cancelled,
        ConversationStatus::WaitingForEvents,
    ] {
        assert_eq!(
            pane_activity_state_for_status(Some(&status)),
            PaneActivityState::NotWorking
        );
    }
}

#[test]
fn terminal_activity_treats_absent_agent_icon_status_as_idle() {
    let status = terminal_activity_status_from_inputs(TerminalActivityStatusInputs {
        cli_session: None,
        has_terminal_conversation: false,
        is_ambient: false,
        selected_conversation_status: None,
        has_active_cli_agent_command: false,
        cli_agent_output_active: false,
        has_active_conversation: false,
        is_long_running: false,
        agent_icon_status: None,
    });

    assert_eq!(
        pane_activity_state_for_status(status.as_ref()),
        PaneActivityState::NotWorking
    );
}

#[test]
fn terminal_activity_uses_rich_agent_icon_status_as_last_fallback() {
    let status = terminal_activity_status_from_inputs(TerminalActivityStatusInputs {
        cli_session: None,
        has_terminal_conversation: false,
        is_ambient: false,
        selected_conversation_status: None,
        has_active_cli_agent_command: false,
        cli_agent_output_active: false,
        has_active_conversation: false,
        is_long_running: false,
        agent_icon_status: Some(ConversationStatus::InProgress),
    });

    assert_eq!(
        pane_activity_state_for_status(status.as_ref()),
        PaneActivityState::Working
    );
}

#[test]
fn terminal_activity_uses_tab_progress_for_active_conversation() {
    let status = activity_status_from_tab_progress(true, true);

    assert_eq!(
        pane_activity_state_for_status(status.as_ref()),
        PaneActivityState::Working
    );
}

#[test]
fn terminal_activity_uses_terminal_conversation_status() {
    let status = activity_status_from_terminal_conversation_status(
        true,
        false,
        Some(ConversationStatus::InProgress),
    );

    assert_eq!(
        pane_activity_state_for_status(status.as_ref()),
        PaneActivityState::Working
    );
}

#[test]
fn terminal_activity_ignores_terminal_conversation_status_without_agent_context() {
    let status = activity_status_from_terminal_conversation_status(
        false,
        false,
        Some(ConversationStatus::InProgress),
    );

    assert_eq!(
        pane_activity_state_for_status(status.as_ref()),
        PaneActivityState::NotWorking
    );
}

#[test]
fn terminal_activity_ignores_tab_progress_without_active_conversation() {
    let status = activity_status_from_tab_progress(false, true);

    assert_eq!(
        pane_activity_state_for_status(status.as_ref()),
        PaneActivityState::NotWorking
    );
}

#[test]
fn terminal_activity_treats_rich_cli_session_without_prompt_as_not_working() {
    let session = cli_agent_session(CLIAgentSessionStatus::InProgress, None);

    assert_eq!(activity_status_for_cli_agent_session(&session), None);
}

#[test]
fn terminal_activity_treats_rich_cli_session_with_prompt_as_working() {
    let session = cli_agent_session(CLIAgentSessionStatus::InProgress, Some("fix this"));

    assert_eq!(
        activity_status_for_cli_agent_session(&session),
        Some(ConversationStatus::InProgress),
    );
}

#[test]
fn terminal_activity_treats_codex_rich_session_without_prompt_as_not_working() {
    let mut session = cli_agent_session(CLIAgentSessionStatus::InProgress, None);
    session.agent = CLIAgent::Codex;

    assert_eq!(activity_status_for_cli_agent_session(&session), None,);
}

#[test]
fn terminal_activity_does_not_restore_suppressed_cli_session_status_from_icon_status() {
    let mut session = cli_agent_session(CLIAgentSessionStatus::InProgress, None);
    session.agent = CLIAgent::Codex;
    session.received_rich_notification = true;

    let status = terminal_activity_status_from_inputs(TerminalActivityStatusInputs {
        cli_session: Some(&session),
        has_terminal_conversation: false,
        is_ambient: false,
        selected_conversation_status: None,
        has_active_cli_agent_command: false,
        cli_agent_output_active: false,
        has_active_conversation: false,
        is_long_running: false,
        agent_icon_status: Some(ConversationStatus::InProgress),
    });

    assert_eq!(status, None);
}

#[test]
fn terminal_activity_converts_cli_session_status_when_status_source_allows_it() {
    let mut session = cli_agent_session(CLIAgentSessionStatus::Success, None);
    session.agent = CLIAgent::Codex;
    session.received_rich_notification = true;

    assert_eq!(
        activity_status_for_cli_agent_session(&session),
        Some(ConversationStatus::Success),
    );
}

#[test]
fn terminal_activity_status_source_waits_for_claude_and_opencode_rich_status() {
    for agent in [CLIAgent::Claude, CLIAgent::OpenCode] {
        let mut session = cli_agent_session(CLIAgentSessionStatus::InProgress, Some("fix this"));
        session.agent = agent;
        session.received_rich_notification = false;

        assert!(
            !cli_agent_session_drives_activity_status(&session),
            "{agent:?} should not drive Activity color before rich status arrives"
        );

        session.received_rich_notification = true;
        assert!(
            cli_agent_session_drives_activity_status(&session),
            "{agent:?} should drive Activity color once rich status arrives"
        );
    }
}

#[test]
fn terminal_activity_status_source_waits_for_codex_rich_status() {
    let mut session = cli_agent_session(CLIAgentSessionStatus::Success, None);
    session.agent = CLIAgent::Codex;
    session.received_rich_notification = false;

    assert!(
        !cli_agent_session_drives_activity_status(&session),
        "non-rich Codex sessions should not pin Activity to their stale status"
    );

    session.received_rich_notification = true;
    assert!(
        cli_agent_session_drives_activity_status(&session),
        "Codex can drive Activity once it reports rich status"
    );
}

#[test]
fn terminal_activity_prefers_conversation_status_over_command_detected_status() {
    let mut session = cli_agent_session(CLIAgentSessionStatus::InProgress, Some("fix this"));
    session.agent = CLIAgent::Codex;
    session.received_rich_notification = false;

    let status = terminal_activity_status_from_inputs(TerminalActivityStatusInputs {
        cli_session: Some(&session),
        has_terminal_conversation: true,
        is_ambient: false,
        selected_conversation_status: Some(ConversationStatus::Success),
        has_active_cli_agent_command: true,
        cli_agent_output_active: true,
        has_active_conversation: true,
        is_long_running: true,
        agent_icon_status: None,
    });

    assert_eq!(status, Some(ConversationStatus::Success));
    assert_eq!(
        pane_activity_state_for_status(status.as_ref()),
        PaneActivityState::NotWorking
    );
}

#[test]
fn terminal_activity_uses_command_detected_running_agent_when_output_is_active() {
    let status = terminal_activity_status_from_inputs(TerminalActivityStatusInputs {
        cli_session: None,
        has_terminal_conversation: false,
        is_ambient: false,
        selected_conversation_status: None,
        has_active_cli_agent_command: true,
        cli_agent_output_active: true,
        has_active_conversation: false,
        is_long_running: true,
        agent_icon_status: None,
    });

    assert_eq!(status, Some(ConversationStatus::InProgress));
}

#[test]
fn terminal_activity_command_detected_agent_is_idle_without_output() {
    // A detected CLI agent (e.g. Codex) sitting quietly at its prompt must read
    // as idle (black), not Working (red). Command presence alone no longer
    // drives the color — real PTY output activity is required.
    let status = terminal_activity_status_from_inputs(TerminalActivityStatusInputs {
        cli_session: None,
        has_terminal_conversation: false,
        is_ambient: false,
        selected_conversation_status: None,
        has_active_cli_agent_command: true,
        cli_agent_output_active: false,
        has_active_conversation: false,
        is_long_running: true,
        agent_icon_status: None,
    });

    assert_eq!(status, None);
    assert_eq!(
        pane_activity_state_for_status(status.as_ref()),
        PaneActivityState::NotWorking
    );
}

#[test]
fn terminal_activity_non_rich_agent_working_only_while_output_is_active() {
    // Core regression test for the "idle Codex pane stays red" bug. A non-rich
    // Codex session's status is a one-way latch: it sticks at InProgress when no
    // OSC9 Stop arrives. The pane color must follow real output activity, not the
    // latched status — Working (red) while output streams, NotWorking (black)
    // once it goes quiet, even though session.status is still InProgress.
    let mut session = cli_agent_session(CLIAgentSessionStatus::InProgress, None);
    session.agent = CLIAgent::Codex;
    session.received_rich_notification = false;

    let working = terminal_activity_status_from_inputs(TerminalActivityStatusInputs {
        cli_session: Some(&session),
        has_terminal_conversation: false,
        is_ambient: false,
        selected_conversation_status: None,
        has_active_cli_agent_command: true,
        cli_agent_output_active: true,
        has_active_conversation: false,
        is_long_running: true,
        agent_icon_status: None,
    });
    assert_eq!(
        pane_activity_state_for_status(working.as_ref()),
        PaneActivityState::Working,
        "streaming Codex output should paint the pane Working (red)"
    );

    let idle = terminal_activity_status_from_inputs(TerminalActivityStatusInputs {
        cli_session: Some(&session),
        has_terminal_conversation: false,
        is_ambient: false,
        selected_conversation_status: None,
        has_active_cli_agent_command: true,
        cli_agent_output_active: false,
        has_active_conversation: false,
        is_long_running: true,
        agent_icon_status: None,
    });
    assert_eq!(
        idle, None,
        "idle Codex (output quiet) must not stay Working even though its session status is latched InProgress"
    );
    assert_eq!(
        pane_activity_state_for_status(idle.as_ref()),
        PaneActivityState::NotWorking
    );
}

#[test]
fn terminal_activity_does_not_use_command_detection_after_non_rich_session_finishes() {
    let mut session = cli_agent_session(CLIAgentSessionStatus::Success, None);
    session.agent = CLIAgent::Codex;
    session.received_rich_notification = false;

    let status = terminal_activity_status_from_inputs(TerminalActivityStatusInputs {
        cli_session: Some(&session),
        has_terminal_conversation: false,
        is_ambient: false,
        selected_conversation_status: None,
        has_active_cli_agent_command: true,
        cli_agent_output_active: true,
        has_active_conversation: false,
        is_long_running: true,
        agent_icon_status: None,
    });

    assert_eq!(status, None);
    assert_eq!(
        pane_activity_state_for_status(status.as_ref()),
        PaneActivityState::NotWorking
    );
}

fn cli_agent_session(status: CLIAgentSessionStatus, query: Option<&str>) -> CLIAgentSession {
    CLIAgentSession {
        agent: CLIAgent::Claude,
        status,
        session_context: CLIAgentSessionContext {
            query: query.map(str::to_owned),
            ..Default::default()
        },
        input_state: CLIAgentInputState::Closed,
        should_auto_toggle_input: false,
        listener: None,
        plugin_version: None,
        remote_host: None,
        draft_text: None,
        custom_command_prefix: None,
        received_rich_notification: true,
    }
}
