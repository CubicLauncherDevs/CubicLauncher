use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum NotificationPosition {
    TopLeft,
    TopCenter,
    BottomLeft,
    BottomCenter,
    BottomRight,
    #[default]
    #[serde(other)]
    TopRight,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NotificationSize {
    Normal,
    Wide,
    #[default]
    #[serde(other)]
    Compact,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(default)]
pub struct NotificationPreferences {
    pub enabled: bool,
    pub position: NotificationPosition,
    pub size: NotificationSize,
    pub title_size: u32,
    pub message_size: u32,
    pub uppercase_title: bool,
    pub bold_title: bool,
    pub duration_seconds: u32,
}

impl Default for NotificationPreferences {
    fn default() -> Self {
        Self {
            enabled: false,
            position: NotificationPosition::TopRight,
            size: NotificationSize::Compact,
            title_size: 13,
            message_size: 11,
            uppercase_title: false,
            bold_title: false,
            duration_seconds: 5,
        }
    }
}

impl NotificationPreferences {
    pub fn from_legacy(prominent: bool) -> Self {
        if prominent {
            Self {
                enabled: false,
                position: NotificationPosition::TopCenter,
                size: NotificationSize::Wide,
                title_size: 18,
                message_size: 16,
                uppercase_title: true,
                bold_title: true,
                duration_seconds: 8,
            }
        } else {
            Self::default()
        }
    }

    pub fn normalize(&mut self) {
        self.title_size = self.title_size.clamp(12, 24);
        self.message_size = self.message_size.clamp(11, 22);
        self.duration_seconds = self.duration_seconds.clamp(3, 30);
    }
}
