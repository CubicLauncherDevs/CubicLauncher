use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Activity {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamps: Option<Timestamps>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assets: Option<Assets>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub party: Option<Party>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secrets: Option<Secrets>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buttons: Option<Vec<Button>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Button {
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Timestamps {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Assets {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub large_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub large_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub small_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub small_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Party {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<[u32; 2]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Secrets {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub join: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spectate: Option<String>,
    #[serde(rename = "match", skip_serializing_if = "Option::is_none")]
    pub match_secret: Option<String>,
}

// ── Builder ──────────────────────────────────────────────────────────────────

/// Fluent builder for [`Activity`].
///
/// # Example
/// ```rust
/// let activity = Activity::builder()
///     .details("Playing a game")
///     .state("In a match")
///     .start_timestamp(now_secs)
///     .large_image("game_logo", "My Game")
///     .small_image("rank_icon", "Gold")
///     .button("Watch stream", "https://twitch.tv/me")
///     .build();
/// ```
#[derive(Default)]
pub struct ActivityBuilder {
    inner: Activity,
}

impl Activity {
    pub fn builder() -> ActivityBuilder {
        ActivityBuilder::default()
    }
}

impl ActivityBuilder {
    /// What the user is doing (top line).
    pub fn details(mut self, details: impl Into<String>) -> Self {
        self.inner.details = Some(details.into());
        self
    }

    /// Sub-state (bottom line).
    pub fn state(mut self, state: impl Into<String>) -> Self {
        self.inner.state = Some(state.into());
        self
    }

    // ── Timestamps ───────────────────────────────────────────────────────────

    pub fn start_timestamp(mut self, secs: u64) -> Self {
        let ts = self.inner.timestamps.get_or_insert_with(Timestamps::default);
        ts.start = Some(secs);
        self
    }

    pub fn end_timestamp(mut self, secs: u64) -> Self {
        let ts = self.inner.timestamps.get_or_insert_with(Timestamps::default);
        ts.end = Some(secs);
        self
    }

    // ── Assets ───────────────────────────────────────────────────────────────

    pub fn large_image(mut self, key: impl Into<String>, tooltip: impl Into<String>) -> Self {
        let assets = self.inner.assets.get_or_insert_with(Assets::default);
        assets.large_image = Some(key.into());
        assets.large_text = Some(tooltip.into());
        self
    }

    pub fn small_image(mut self, key: impl Into<String>, tooltip: impl Into<String>) -> Self {
        let assets = self.inner.assets.get_or_insert_with(Assets::default);
        assets.small_image = Some(key.into());
        assets.small_text = Some(tooltip.into());
        self
    }


    pub fn button(mut self, label: impl Into<String>, url: impl Into<String>) -> Self {
        let buttons = self.inner.buttons.get_or_insert_with(Vec::new);
        if buttons.len() < 2 {
            buttons.push(Button {
                label: label.into(),
                url: url.into(),
            });
        }
        self
    }


    pub fn party(mut self, id: impl Into<String>, current: u32, max: u32) -> Self {
        self.inner.party = Some(Party {
            id: Some(id.into()),
            size: Some([current, max]),
        });
        self
    }

    pub fn instance(mut self, instance: bool) -> Self {
        self.inner.instance = Some(instance);
        self
    }

    pub fn build(self) -> Activity {
        self.inner
    }
}
