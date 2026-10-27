//! Host ABI fixture: byte-exact beta common / SDK / NativeSessions; explicit registration stub.
#![allow(dead_code)]
pub mod common {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/beta/src-tauri/src/native_core/common.rs"
    ));
}
pub mod native_core {
    pub use crate::common;
    pub mod extensions {
        #[cfg(not(test))]
        pub use crate::Sessions;
        pub use crate::{v2_sdk, v2_uuid_generator};
    }
}

#[cfg(not(test))]
#[rustfmt::skip]
#[path = "../../../fixtures/beta/src-tauri/src/native_core/extensions/native.rs"]
mod native;
#[rustfmt::skip]
#[path = "../../../fixtures/beta/src-tauri/src/native_core/extensions/v2_sdk.rs"]
pub mod v2_sdk;
#[path = "../../../adapter/v2_uuid_generator.rs"]
pub mod v2_uuid_generator;
// Explicit compile/test registration substitute, not a patched beta runtime.
mod v2_runtime {
    use super::v2_uuid_generator as a;
    use crate::native_core::common::{CoreContext, CoreResult};
    use serde_json::Value;
    pub fn slug(id: &str) -> Option<&str> {
        match id {
            "uuid-generator" | "raycast.uuid-generator" | a::ID => Some(a::SLUG),
            _ => None,
        }
    }
    pub fn supports(id: &str, cmd: &str) -> bool {
        slug(id).is_some() && a::commands().contains(&cmd)
    }
    pub async fn handle(
        ctx: &CoreContext,
        slug: &str,
        cmd: &str,
        op: &str,
        body: &Value,
        state: &mut Value,
    ) -> CoreResult {
        if slug != a::SLUG {
            return Err("Unsupported test registration".into());
        }
        a::handle(ctx, cmd, op, body, state).await
    }
}
mod registry {
    use crate::native_core::common::CoreContext;
    use serde_json::Value;
    // Only disposable fixture preferences. No real profile access or legacy package lookup.
    pub fn installed_command(ctx: &CoreContext, id: &str, cmd: &str) -> Option<Value> {
        if !super::v2_runtime::supports(id, cmd) {
            return None;
        }
        let path = ctx.paths.data_dir.join("fixture-installed.json");
        if !path.exists() {
            return None;
        }
        let bytes = std::fs::read(path).ok()?;
        let v: Value = serde_json::from_slice(&bytes).ok()?;
        Some(v)
    }
    pub fn preferences(v: &Value, _cmd: Option<&str>) -> Result<Value, String> {
        Ok(v.clone())
    }
}
#[cfg(not(test))]
#[derive(Clone, Default)]
pub struct Sessions {
    native: native::NativeSessions,
    files: std::sync::Arc<std::sync::Mutex<()>>,
}
#[cfg(not(test))]
impl Sessions {
    pub async fn handle(
        &self,
        ctx: &crate::common::CoreContext,
        op: &str,
        v: serde_json::Value,
    ) -> serde_json::Value {
        self.native
            .handle(ctx, &self.files, op, &[v])
            .await
            .unwrap_or_else(|| Err("No native handler".into()))
            .unwrap_or_else(|e| serde_json::json!({"ok":false,"message":e}))
    }
    pub async fn clear(&self) {
        self.native.clear().await;
    }
    pub async fn clear_slug(&self) {
        self.native.clear_slug("uuid-generator").await;
    }
}
