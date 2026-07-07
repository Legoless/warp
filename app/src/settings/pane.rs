use settings::macros::define_settings_group;
use settings::{RespectUserSyncSetting, SupportedPlatforms, SyncToCloud};
use warp_core::ui::theme::AnsiColorIdentifier;

#[derive(
    Default,
    Debug,
    serde::Serialize,
    serde::Deserialize,
    PartialEq,
    Copy,
    Clone,
    schemars::JsonSchema,
    settings_value::SettingsValue,
)]
#[schemars(description = "How panes are colored.", rename_all = "snake_case")]
pub enum PaneColorMode {
    #[default]
    Off,
    Activity,
}

impl PaneColorMode {
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Activity => "Activity",
        }
    }
}

settings::macros::implement_setting_for_enum!(
    PaneColorMode,
    PaneSettings,
    SupportedPlatforms::ALL,
    SyncToCloud::Globally(RespectUserSyncSetting::Yes),
    surface: settings::SettingSurfaces::GUI,
    private: false,
    toml_path: "appearance.panes.color_mode",
    description: "How panes are colored.",
);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneActivityState {
    Working,
    NeedsHelp,
    RequiresAttention,
    NotWorking,
}

impl PaneActivityState {
    pub const ALL: [Self; 4] = [
        Self::Working,
        Self::NeedsHelp,
        Self::RequiresAttention,
        Self::NotWorking,
    ];
    pub const COUNT: usize = Self::ALL.len();

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Working => "Working",
            Self::NeedsHelp => "Needs help",
            Self::RequiresAttention => "Requires attention",
            Self::NotWorking => "Not working",
        }
    }

    pub fn default_color(self) -> AnsiColorIdentifier {
        match self {
            Self::Working => AnsiColorIdentifier::Red,
            Self::NeedsHelp => AnsiColorIdentifier::Yellow,
            Self::RequiresAttention => AnsiColorIdentifier::Yellow,
            Self::NotWorking => AnsiColorIdentifier::Black,
        }
    }

    pub fn index(self) -> usize {
        match self {
            Self::Working => 0,
            Self::NeedsHelp => 1,
            Self::RequiresAttention => 2,
            Self::NotWorking => 3,
        }
    }
}

#[derive(
    Debug,
    Clone,
    serde::Serialize,
    serde::Deserialize,
    PartialEq,
    Eq,
    schemars::JsonSchema,
    settings_value::SettingsValue,
)]
#[serde(default)]
#[schemars(description = "Pane colors for agent activity states.")]
pub struct PaneActivityColors {
    #[schemars(description = "Pane color while the agent is working.")]
    pub working: AnsiColorIdentifier,
    #[schemars(description = "Pane color while the agent needs help.")]
    pub needs_help: AnsiColorIdentifier,
    #[schemars(description = "Pane color while the agent requires attention.")]
    pub requires_attention: AnsiColorIdentifier,
    #[schemars(description = "Pane color while the agent is not working.")]
    pub not_working: AnsiColorIdentifier,
}

impl Default for PaneActivityColors {
    fn default() -> Self {
        Self {
            working: PaneActivityState::Working.default_color(),
            needs_help: PaneActivityState::NeedsHelp.default_color(),
            requires_attention: PaneActivityState::RequiresAttention.default_color(),
            not_working: PaneActivityState::NotWorking.default_color(),
        }
    }
}

impl PaneActivityColors {
    pub fn color_for(&self, state: PaneActivityState) -> AnsiColorIdentifier {
        match state {
            PaneActivityState::Working => self.working,
            PaneActivityState::NeedsHelp => self.needs_help,
            PaneActivityState::RequiresAttention => self.requires_attention,
            PaneActivityState::NotWorking => self.not_working,
        }
    }

    pub fn with_color(&self, state: PaneActivityState, color: AnsiColorIdentifier) -> Self {
        let mut colors = self.clone();
        match state {
            PaneActivityState::Working => colors.working = color,
            PaneActivityState::NeedsHelp => colors.needs_help = color,
            PaneActivityState::RequiresAttention => colors.requires_attention = color,
            PaneActivityState::NotWorking => colors.not_working = color,
        }
        colors
    }
}

settings::macros::implement_setting_for_enum!(
    PaneActivityColors,
    PaneSettings,
    SupportedPlatforms::ALL,
    SyncToCloud::Globally(RespectUserSyncSetting::Yes),
    surface: settings::SettingSurfaces::GUI,
    private: false,
    toml_path: "appearance.panes.activity_colors",
    description: "Pane colors for agent activity states.",
);

define_settings_group!(PaneSettings, settings: [
    should_dim_inactive_panes: ShouldDimInactivePanes {
        type: bool,
        default: false,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
        surface: settings::SettingSurfaces::GUI,
        private: false,
        toml_path: "appearance.panes.should_dim_inactive_panes",
        description: "Whether inactive panes are visually dimmed.",
    },
    focus_panes_on_hover: FocusPaneOnHover {
        type: bool,
        default: false,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
        surface: settings::SettingSurfaces::GUI,
        private: false,
        toml_path: "appearance.panes.focus_pane_on_hover",
        description: "Whether panes are focused when hovered over.",
    },
    pane_color_mode: PaneColorMode,
    pane_activity_background: PaneActivityBackground {
        type: bool,
        default: false,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
        surface: settings::SettingSurfaces::GUI,
        private: false,
        toml_path: "appearance.panes.activity_background",
        description: "Whether activity colors tint pane backgrounds.",
    },
    pane_activity_colors: PaneActivityColors,
]);

#[cfg(test)]
#[path = "pane_tests.rs"]
mod tests;
