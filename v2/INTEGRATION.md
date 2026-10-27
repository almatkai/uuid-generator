# Compiled-in integration at beta fd300e276356587726566026f072563eb9c34cb1

These are required host edits for the parent/host maintainer. This delivery does not modify the dirty Tezbar checkout, install a package or change a profile.

1. Vendor the **contents of this `v2/` directory** under the stable host path `src-tauri/native-extensions/uuid-generator/`. Keep its Cargo.lock, source, adapter, manifest, notices and original PNG. Do not vendor a temporary clone path.
2. Add the host Cargo dependency:
   ```toml
   tezbar-uuid-generator-v2 = { path = "native-extensions/uuid-generator" }
   ```
   The host already supplies serde_json and SDK common dependencies. The adapter additionally needs `getrandom = "=0.2.16"` in the host dependencies for cryptographic process-capability nonces. The engine has its own locked dependencies. Host Cargo.lock must be updated/reviewed deliberately; it cannot use this leaf lock as the entire app lock.
3. Register the adapter as an SDK **sibling** in `src-tauri/src/native_core/extensions/mod.rs`:
   ```rust
   #[path = "../../../native-extensions/uuid-generator/adapter/v2_uuid_generator.rs"]
   mod v2_uuid_generator;
   ```
   Its exported `commands()` returns exactly the nine original command names. `handle(ctx, command, operation, payload, state)` uses the actual `CoreContext` and sibling `v2_sdk`. No replacement SDK/runtime is shipped for production.
4. In `v2_runtime.rs`, add `"uuid-generator"` to `slug`, map `commands("uuid-generator")` to `super::v2_uuid_generator::commands()`, and add a dispatch arm calling `super::v2_uuid_generator::handle(ctx, command, operation, payload, state).await`. Canonical `com.tezbar.native.uuid-generator`, `raycast.uuid-generator` alias and bare slug must resolve to the same dispatch. Installation requires a compiled handler, supported platform and installed command.
5. Add the `extension.json` object to the bundled `v2_catalog.json`. Update **all derived catalog/compiled coverage assertions** and fixtures: entry count increases by one, declared and compiled command coverage increases by nine on macOS and Windows. Inspect actual host tests rather than assuming old totals. Verify catalog and runtime commands are equal; leave both verification states `not-run`. Retain the V1 catalog/source package.
6. Copy `assets/command-icon.png` unchanged to **`src/renderer/public/extensions/icons/uuid-generator.png`**. Manifest URL is `/extensions/icons/uuid-generator.png`. Required size 17477 bytes and SHA256 `423115229079dcd49a77183284e0704df85c2ecbc9bc106e14664eaaf55d95c9`.
7. Rebuild/restart the native host and install the canonical bundled package through the actual host install flow. A manifest/marker by itself cannot install or execute this Rust extension. No runtime Node/Bun/JS/download is required. No legacy storage migration is asserted.
8. Run host catalog/registry/native-session/renderer suites plus actual macOS and Windows QA. Test preferences, all nine commands, failure/retry, corrupt native storage, concurrent sessions, action freshness, copy/paste/focus and native uninstall. Record real QA artifacts before changing verification state.

Launch arguments live at `payload.argumentValues`; the real native session injects its preference snapshot at `payload.preferences`. The adapter retains that launch snapshot and supports `extension:run-command`, `extension:invoke-action`, `extension:search-text-changed`, `extension:refresh-session`, `extension:load-more`, `extension:dispose-session`. Form actions use `formValues`, including `{value: text}` field edits. User event text cannot replace a computed copy/paste target.

Actual effects used: SDK `copy` → `clipboard:write [text]`; `core:platform:paste-text [text]` performs the entire paste lifecycle; copy success can then invoke `window:hide []` for no-view commands. History copy stays in the List and does not hide. There are no browser/open, selection or clipboard-read requirements in the original UUID source. If adding them later, actual beta routes are `clipboard:read [] -> string`, `core:platform:selected-text [] -> string`, `core:platform:paste []` for an already prepared clipboard and SDK `open` → `shell:open [target]`. Do not treat the read routes as effect acknowledgements.

All **effect** acknowledgements require an object, `ok:true`, no non-null `error`, and absent/true `success`. Boolean/null/malformed/contradictory replies fail. If an actual host effect currently returns another shape, fix and validate that host route explicitly before claiming successful integration; weakening the adapter would create false success. No genuine beta HUD/toast call is assumed.

`validation/beta-host` compiles the concrete adapter with byte-exact beta common/SDK/NativeSessions. Tauri, registration and installation lookup are explicitly labelled headless fixture substitutes. The original runtime/registry/platform and renderer sources are also retained for ABI inspection. NativeSessions is compiled as a normal dependency so its unrelated Perplexity unit-test module is not substituted or altered; our headless integration tests exercise its real session operations. The separate headless unit target runs the unchanged common/SDK's five tests. Production host registration/rebuild and real OS UI effects have **not run**.
