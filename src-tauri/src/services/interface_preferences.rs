use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InterfaceDensity {
    Compact,
    Comfortable,
    #[default]
    #[serde(other)]
    Theme,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(default)]
pub struct InterfacePreferences {
    pub scale: u32,
    pub density: InterfaceDensity,
}

impl Default for InterfacePreferences {
    fn default() -> Self {
        Self {
            scale: 100,
            density: InterfaceDensity::Theme,
        }
    }
}

impl InterfacePreferences {
    pub fn normalize(&mut self) {
        if ![90, 100, 110, 125].contains(&self.scale) {
            self.scale = 100;
        }
    }
}
