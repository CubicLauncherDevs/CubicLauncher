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
        current: u32,
        total: u32,
        d_type: Cow<'static, str>,
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
    DRetry {
        version: Arc<str>,
        attempt: u8,
        max: u8,
    },
    JREChanged,
    STChanged,
    ThemeChanged {
        id: CompactString,
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
