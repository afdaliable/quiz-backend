use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PomodoroPreferences {
    pub enabled: bool,
    pub focus_duration: i32,
    pub short_break_duration: i32,
    pub long_break_duration: i32,
    pub long_break_after: i32,
    pub sound_enabled: bool,
    pub browser_notification_enabled: bool,
}

impl Default for PomodoroPreferences {
    fn default() -> Self {
        PomodoroPreferences {
            enabled: false,
            focus_duration: 25,
            short_break_duration: 5,
            long_break_duration: 15,
            long_break_after: 4,
            sound_enabled: true,
            browser_notification_enabled: false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThemePreferences {
    pub dark_mode: bool,
}

impl Default for ThemePreferences {
    fn default() -> Self {
        ThemePreferences { dark_mode: false }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserPreferences {
    pub pomodoro: PomodoroPreferences,
    #[serde(default)]
    pub theme: ThemePreferences,
}

impl Default for UserPreferences {
    fn default() -> Self {
        UserPreferences {
            pomodoro: PomodoroPreferences::default(),
            theme: ThemePreferences::default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdatePreferencesRequest {
    pub pomodoro: Option<PomodoroPreferences>,
    pub theme: Option<ThemePreferences>,
}
