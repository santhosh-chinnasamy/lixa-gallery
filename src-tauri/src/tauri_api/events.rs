use gallery_core::events::EventHub;
use gallery_core::models::{PhotoMetadata, PhotosUpdatedEvent};
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

    fn emit_photos_updated(&self, generation: u64, photos: &[PhotoMetadata]) -> anyhow::Result<()> {
        if photos.is_empty() {
            return Ok(());
        }
        self.app.emit(
            "photos-updated",
            PhotosUpdatedEvent {
                generation,
                photos: photos.to_vec(),
            },
        )?;
        Ok(())
    }
}
