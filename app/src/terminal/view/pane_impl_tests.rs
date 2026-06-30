use super::*;
use crate::terminal::CLIAgent;

#[test]
fn pane_activity_preserves_content_colors_for_alt_screen_tuis() {
    assert!(should_preserve_activity_content_colors(true, true, false));
}

#[test]
fn pane_activity_preserves_content_colors_for_cli_agent_tuis_without_alt_screen() {
    assert!(should_preserve_activity_content_colors(true, false, true));
}

#[test]
fn pane_activity_does_not_preserve_content_colors_for_block_list_content() {
    // Block-list panes are tinted on the pane background itself.
    assert!(!should_preserve_activity_content_colors(true, false, false));
}

#[test]
fn pane_activity_does_not_preserve_content_colors_outside_activity_mode() {
    assert!(!should_preserve_activity_content_colors(false, true, true));
}

#[test]
fn agent_icon_status_uses_activity_status_for_command_detected_codex() {
    let variant = IconWithStatusVariant::CLIAgent {
        agent: CLIAgent::Codex,
        status: None,
        is_ambient: false,
    };

    let variant =
        agent_icon_variant_with_activity_status(variant, Some(ConversationStatus::InProgress));

    match variant {
        IconWithStatusVariant::CLIAgent {
            agent,
            status,
            is_ambient,
        } => {
            assert_eq!(agent, CLIAgent::Codex);
            assert_eq!(status, Some(ConversationStatus::InProgress));
            assert!(!is_ambient);
        }
        _ => panic!("expected Codex CLI agent variant"),
    }
}

#[test]
fn agent_icon_status_clears_when_activity_status_is_idle() {
    let variant = IconWithStatusVariant::CLIAgent {
        agent: CLIAgent::Codex,
        status: Some(ConversationStatus::InProgress),
        is_ambient: false,
    };

    let variant = agent_icon_variant_with_activity_status(variant, None);

    match variant {
        IconWithStatusVariant::CLIAgent { status, .. } => {
            assert_eq!(status, None);
        }
        _ => panic!("expected CLI agent variant"),
    }
}
