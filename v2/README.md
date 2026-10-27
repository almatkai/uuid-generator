# UUID Generator — Rust Native V2

All nine commands from the retained own V1 source are implemented by the offline Rust engine and the concrete `adapter/v2_uuid_generator.rs`. The original icon is unchanged. Native V2 is compiled into Tezbar; installing `extension.json` alone cannot install Rust code. Host registration, rebuild and real macOS/Windows UI/clipboard verification remain **not-run**.

Source baseline: `almatkai/uuid-generator` at `5a17980eaad254c8e95f845cd8b1002788998695`. Beta ABI: `fd300e276356587726566026f072563eb9c34cb1`. See [INTEGRATION.md](INTEGRATION.md), [VALIDATION.md](VALIDATION.md), [PROVENANCE.json](PROVENANCE.json) and [NOTICES.md](NOTICES.md).

## Behavior

- `generate`: UUIDv4 with cryptographic operating system entropy.
- `generateV5`: SHA-1 over the UUID namespace bytes followed by UTF-8 name bytes. The default empty namespace is invalid, as in V1; configure a valid namespace. Repeated names produce repeated identifiers and history entries.
- `generateV7`: `uuid 11.1.1` timestamp/32-bit sequence algorithm. Base32 preference instead uses the **different** `uuid 10.0.0` algorithm inside `typeid-js 1.2.0`, with an empty prefix.
- `generateUlid`: `ulidx 2.4.1` monotonic factory. Uppercase even when `upperCaseLetters=false`. New randomness uses sixteen one-byte OS draws and the original `min(floor(byte*32/255),31)` mapping in reverse character order. Repeated/backwards clocks increment the 80-bit suffix at the previous timestamp. Suffix overflow fails explicitly. Epoch-zero first call preserves the original failure.
- `generateTypeID`: the nested UUIDv7 algorithm and lowercase Crockford Base32, optional global uppercase conversion. Prefix: empty or at most 63 ASCII lowercase letters/underscores, beginning/ending with a letter. No version-7-only restriction is invented for parsing/packing.
- `parseTypeID`: last underscore separates the prefix; validates length, first suffix character `0..7` and lowercase Base32. The decoded UUID must pass `uuid 10` stringify validation (versions 1–8 with RFC variant, or nil/max). Global uppercase preference does not change decoded output. Thus uppercased generated TypeIDs cannot be parsed by the original parser.
- `formatUuid`: removes every non-ASCII-hex character; requires 32 remaining characters; preserves their case and accepts any version/variant bits. Global uppercase preference is ignored.
- `packTypeID`: validates the input with `uuid 11` parse semantics (versions 1–8/RFC variant or nil/max), then encodes all 128 bits with the given suffix-as-prefix. Global uppercase applies. Missing suffix/UUID and invalid prefixes fail.
- `viewHistory`: initial newest-first display, individual copy always copies, Copy/Paste All follows the preference and uses the full displayed history (including when server search filters rows). Delete removes **all** matching UUID strings; after deletion the source switches to oldest-first order, which is preserved. Clear requires confirmation and removes only `uuidHistory`. Search preserves source order.

Counts use JavaScript `parseInt(...,10)` and original truthiness/defaulting, including numeric/array coercions. Empty/falsy input defaults to 1; string `"0"` gives an empty clipboard result; `"1e3"` gives one; negative counts fail the original array allocation. Values over 10000 fail. Generated output uses CRLF. Only the five generation commands append history; parse/format/pack do not. Entries retain `uuid`, ISO milliseconds timestamp and `type`, with `uuidHistory` stored as a JSON **string**. Generation records all output before copy/paste, matching V1.

UUIDv7 state is serialized in a process-wide mutex across commands/sessions. UUID 11 reseeds its sequence only when the clock advances, wrapping the 32-bit sequence into a synthetic millisecond. UUID 10 carries its split sequence across advancing clocks; backwards clocks within 10 seconds retain timestamp, larger reversals reset timestamp. Sequence rollover and the full timestamp limit are tested. No deterministic seed or JavaScript/network fallback exists in production.

## Deliberate Native adaptations and limits

No legacy history/profile is read or migrated. Storage uses the unchanged Native V2 SDK namespace `native-extensions-v2/uuid-generator/storage.json`, its shared transaction lock and atomic writer. Unknown root keys are preserved, malformed storage/history is rejected unchanged. A bounded internal receipt map makes uncertain storage completion safe to retry. Restarting or disposing a session loses pending output/effect state; reopening is a **new** generation, not a claim of exactly-once OS effects across process crashes.

Original TypeID generation catches an obsolete error string and silently suppresses several errors. V2 reports those errors inline. Invalid preferences/argument shapes and corrupt history get explicit errors instead of false success. A form supplies missing input; it keeps draft IDs and queued edits stable. Back from a failure restores drafts. After a failed effect, the original output stays pending; submitting changed drafts resumes that pending output until the session is closed. An uncertain copy/paste requires the separate explicit retry action, labelled as potentially duplicating the effect. Acknowledged copy/paste, storage and hide are checkpointed independently. Clipboard success must be an object with `ok:true`, absent/null `error`, and absent/true `success`; transport errors are sanitized. `paste-text` already writes clipboard, hides and restores focus; V2 does not call paste twice.

History action IDs are session bound, regenerated by each committed list render and revalidated against current storage. A persisted `_uuidV2Revision` changes on every history mutation, so actions and pending history retries are rejected after any intervening change, including deletion followed by identical-value recreation. This conservative freshness rule is a Native adaptation. Stale/forged/cross-session IDs, IDs from discarded staged searches and deleted-entry copies are rejected. A process nonce and global counter prevent revival when cloned search states are discarded. Refresh and load-more return unchanged without new IDs. History is complete within the limits; pagination is unnecessary. The row search is a Native adaptation to the Raycast List's client filtering, using `__hasServerSearch:true` and `filtering:false`.

V1 has **no history retention limit**. V2 never evicts/truncates history: it explicitly rejects more than 100000 entries, serialized history over 32 MiB, text/count/search input over 1 MiB, more than 100000 operation receipts, clocks outside the 48-bit timestamp range, and more than 64 MiB total UUIDv5 name hashing per invocation. Delete/clear entries to free history space; receipt exhaustion requires deliberate native storage maintenance after closing sessions. Root SDK reads itself are unchanged/unbounded; these limits apply after SDK decoding. Original valid large files can therefore use memory while the SDK reads them.

There is no genuine HUD/toast API in this beta. Success uses native result/inline messages and failure uses Form/Detail. Failure messages are prefixed `Failed` because the beta parent renderer derives notice tone from message text; the actual parent action handler is tested. History metadata is represented using beta accessories; exact tooltip/date presentation, arbitrary Raycast shortcuts and destructive styling are beta UI limitations. Parser/routing tests are not visual QA. Windows GNU checks compile against labelled headless Tauri/router/registration fixtures, not the full production app or an OS clipboard.

## Reproduce

Requires Rust stable and 1.85.0, Node (validated with 26.7.0), Python 3 for materialization/auditing, and cached locked crates for `--offline`. JavaScript and the vendored compiler/validator are validation-only.

```sh
python3 scripts/materialize.py
node oracle/run.cjs
node scripts/schema.cjs
cargo test --locked --offline
cargo test --manifest-path validation/headless/Cargo.toml --locked --offline
node validation/renderer.cjs
python3 scripts/audit.py
python3 scripts/validate.py
python3 scripts/package.py
```

Original Raycast extension by **jmaeso**, contributors **GastroGeek, pernielsentikaer, LunaticMuch, hmarr, harsh_khandelwal, lukas-runge, ridemountainpig**. Own V1 adaptation by almatkai. License texts and byte hashes are retained; see [NOTICES.md](NOTICES.md).
