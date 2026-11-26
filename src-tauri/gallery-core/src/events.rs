use crate::models::PhotoMetadata;

pub trait EventHub: Send + Sync {
    fn emit_progress(&self, event: &str, payload: u32) -> anyhow::Result<()>;

    /// Batched photo rows after materialize / EXIF so the UI can merge by path.
    /// `generation` is the folder session that produced the patch.
    fn emit_photos_updated(&self, generation: u64, photos: &[PhotoMetadata]) -> anyhow::Result<()>;
}
