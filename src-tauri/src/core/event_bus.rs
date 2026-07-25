use crate::services::InstanceDto;
use compact_str::CompactString;
use serde::Serialize;
use std::borrow::Cow;
use std::sync::{Arc, OnceLock};
use tauri::{AppHandle, Emitter};

static APP: OnceLock<AppHandle> = OnceLock::new();

#[derive(Clone, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum AppEvent {
    InstanceEdited {
        id: CompactString,
    },
    InstanceCreated {
        id: CompactString,
        dto: InstanceDto,
    },
    DProgress {
        version: Arc<str>,
        stage: Cow<'static, str>,
        item_current: u64,
        item_total: u64,
        bytes_current: u64,
        bytes_total: u64,
        current_item: Option<String>,
    },
    DStage {
        version: Arc<str>,
        stage: Cow<'static, str>,
        info: Option<String>,
    },
    DEnqueue {
        version: Arc<str>,
    },
    DFinish {
        version: Arc<str>,
    },
    DError {
        version: Arc<str>,
        message: CompactString,
    },
    JREChanged,
    STChanged,
    ThemeChanged {
        id: CompactString,
    },
    ModsEnriched {
        id: CompactString,
    },
    ResourcepacksEnriched {
        id: CompactString,
    },
    ShaderpacksEnriched {
        id: CompactString,
    },
    InstanceCrashed {
        id: CompactString,
        name: CompactString,
        exit_code: Option<i32>,
        reason: Option<CompactString>,
    },
}

pub fn init(app: AppHandle) {
    let _ = APP.set(app);
}

pub fn emit(event: AppEvent) {
    if let Some(app) = APP.get()
        && let Err(err) = app.emit("app-event", event)
    {
        tracing::warn!("failed to emit event: {}", err);
    }
}
