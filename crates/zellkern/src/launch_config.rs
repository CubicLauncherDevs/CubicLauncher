use std::collections::HashMap;
use std::path::PathBuf;

/// All parameters needed to launch a Minecraft instance.
#[derive(Debug, Clone)]
pub struct LaunchConfig {
    pub java_path: PathBuf,
    pub username: String,
    pub min_ram: String,
    pub max_ram: String,
    pub width: u32,
    pub height: u32,
    pub cracked: bool,
    pub demo_mode: bool,
    pub env: HashMap<String, String>,
    pub quick_play: Option<QuickPlay>,
    pub access_token: Option<String>,
    pub auth_uuid: Option<String>,
    pub user_type: Option<String>,
    pub extra_jvm_args: Vec<String>,
    pub authlib_injector_path: Option<PathBuf>,
    pub yggdrasil_metadata_b64: Option<String>,
}

#[derive(Debug, Clone)]
pub enum QuickPlay {
    Singleplayer(String),
    Multiplayer(String),
    Realms(String),
}

impl Default for LaunchConfig {
    fn default() -> Self {
        Self {
            java_path: PathBuf::from("java"),
            username: "Player".to_string(),
            min_ram: "512M".to_string(),
            max_ram: "2G".to_string(),
            width: 854,
            height: 480,
            cracked: false,
            demo_mode: false,
            env: HashMap::new(),
            quick_play: None,
            access_token: None,
            auth_uuid: None,
            user_type: None,
            extra_jvm_args: Vec::new(),
            authlib_injector_path: None,
            yggdrasil_metadata_b64: None,
        }
    }
}

impl LaunchConfig {
    pub fn builder() -> LaunchConfigBuilder {
        LaunchConfigBuilder::new()
    }
}

#[derive(Default)]
pub struct LaunchConfigBuilder(LaunchConfig);

impl LaunchConfigBuilder {
    pub fn new() -> Self {
        Self(LaunchConfig::default())
    }

    pub fn java_path(mut self, p: impl Into<PathBuf>) -> Self {
        self.0.java_path = p.into();
        self
    }
    pub fn username(mut self, u: impl Into<String>) -> Self {
        self.0.username = u.into();
        self
    }
    pub fn ram(mut self, min: impl Into<String>, max: impl Into<String>) -> Self {
        self.0.min_ram = min.into();
        self.0.max_ram = max.into();
        self
    }
    pub fn resolution(mut self, w: u32, h: u32) -> Self {
        self.0.width = w;
        self.0.height = h;
        self
    }
    pub fn cracked(mut self, c: bool) -> Self {
        self.0.cracked = c;
        self
    }
    pub fn demo(mut self, d: bool) -> Self {
        self.0.demo_mode = d;
        self
    }
    pub fn env(mut self, key: impl Into<String>, val: impl Into<String>) -> Self {
        self.0.env.insert(key.into(), val.into());
        self
    }
    pub fn quick_play(mut self, qp: QuickPlay) -> Self {
        self.0.quick_play = Some(qp);
        self
    }
    pub fn access_token(mut self, t: impl Into<String>) -> Self {
        self.0.access_token = Some(t.into());
        self
    }
    pub fn auth_uuid(mut self, u: impl Into<String>) -> Self {
        self.0.auth_uuid = Some(u.into());
        self
    }
    pub fn user_type(mut self, t: impl Into<String>) -> Self {
        self.0.user_type = Some(t.into());
        self
    }
    pub fn extra_jvm_args(mut self, args: Vec<String>) -> Self {
        self.0.extra_jvm_args = args;
        self
    }
    pub fn authlib_injector_path(mut self, p: impl Into<PathBuf>) -> Self {
        self.0.authlib_injector_path = Some(p.into());
        self
    }
    pub fn yggdrasil_metadata_b64(mut self, m: impl Into<String>) -> Self {
        self.0.yggdrasil_metadata_b64 = Some(m.into());
        self
    }
    pub fn build(self) -> LaunchConfig {
        self.0
    }
}
