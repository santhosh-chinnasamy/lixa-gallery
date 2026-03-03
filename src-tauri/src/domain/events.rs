pub trait EventHub: Send + Sync {
    fn emit_progress(&self, event: &str, payload: u32) -> anyhow::Result<()>;
}
