//! Validation-only host; no production operating system integrations.
#[cfg(test)]
pub mod common {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/beta/src-tauri/src/native_core/common.rs"
    ));
}
#[cfg(test)]
pub mod native_core {
    pub use crate::common;
}
#[cfg(test)]
#[rustfmt::skip]
#[path = "../../../fixtures/beta/src-tauri/src/native_core/extensions/v2_sdk.rs"]
pub mod v2_sdk;
