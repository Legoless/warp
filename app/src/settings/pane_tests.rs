use settings::Setting;
use warp_core::ui::theme::AnsiColorIdentifier;
use warpui::{App, SingletonEntity};

use super::*;
use crate::test_util::settings::initialize_settings_for_tests;

#[test]
fn pane_color_mode_defaults_to_off() {
    App::test((), |mut app| async move {
        initialize_settings_for_tests(&mut app);

        PaneSettings::handle(&app).read(&app, |settings, _ctx| {
            assert_eq!(settings.pane_color_mode, PaneColorMode::Off);
            assert!(!*settings.pane_activity_background);
        });
    });
}

#[test]
fn pane_activity_colors_default_to_requested_mapping() {
    let colors = PaneActivityColors::default();

    assert_eq!(
        colors.color_for(PaneActivityState::Working),
        AnsiColorIdentifier::Red
    );
    assert_eq!(
        colors.color_for(PaneActivityState::NeedsHelp),
        AnsiColorIdentifier::Yellow
    );
    assert_eq!(
        colors.color_for(PaneActivityState::RequiresAttention),
        AnsiColorIdentifier::Yellow
    );
    assert_eq!(
        colors.color_for(PaneActivityState::NotWorking),
        AnsiColorIdentifier::Black
    );
}

#[test]
fn pane_color_mode_uses_panes_path() {
    assert_eq!(
        PaneColorMode::toml_path(),
        Some("appearance.panes.color_mode")
    );
    assert_eq!(PaneColorMode::hierarchy(), Some("appearance.panes"));
    assert_eq!(PaneColorMode::toml_key(), "color_mode");
}

#[test]
fn pane_activity_colors_uses_panes_path() {
    assert_eq!(
        PaneActivityColors::toml_path(),
        Some("appearance.panes.activity_colors")
    );
    assert_eq!(PaneActivityColors::hierarchy(), Some("appearance.panes"));
    assert_eq!(PaneActivityColors::toml_key(), "activity_colors");
}

#[test]
fn pane_activity_background_uses_panes_path() {
    assert_eq!(
        PaneActivityBackground::toml_path(),
        Some("appearance.panes.activity_background")
    );
    assert_eq!(
        PaneActivityBackground::hierarchy(),
        Some("appearance.panes")
    );
    assert_eq!(PaneActivityBackground::toml_key(), "activity_background");
}

#[test]
fn pane_activity_colors_can_update_one_activity_color() {
    let colors = PaneActivityColors::default()
        .with_color(PaneActivityState::NeedsHelp, AnsiColorIdentifier::Cyan);

    assert_eq!(
        colors.color_for(PaneActivityState::NeedsHelp),
        AnsiColorIdentifier::Cyan
    );
    assert_eq!(
        colors.color_for(PaneActivityState::Working),
        AnsiColorIdentifier::Red
    );
}
