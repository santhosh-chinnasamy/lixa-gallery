use gallery_core::events::EventHub;
use tauri::{AppHandle, Emitter};

pub struct TauriEventHub {
    app: AppHandle,
}

impl TauriEventHub {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl EventHub for TauriEventHub {
    fn emit_progress(&self, event: &str, payload: u32) -> anyhow::Result<()> {
        self.app.emit(event, payload)?;
        Ok(())
    }
}
