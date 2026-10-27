//! Headless compile fixture only. Never emits events, opens windows or accesses the OS.
#[derive(Clone)]
pub struct AppHandle;
pub trait Emitter {
    fn emit<T>(&self, channel: &str, payload: T) -> Result<(), String>;
}
impl Emitter for AppHandle {
    fn emit<T>(&self, _channel: &str, _payload: T) -> Result<(), String> {
        Err("Headless Tauri stub cannot emit".into())
    }
}
