# Provenance and licenses

Original Raycast extension: **UUID Generator**, author **jmaeso**. Contributors: **GastroGeek, pernielsentikaer, LunaticMuch, hmarr, harsh_khandelwal, lukas-runge, ridemountainpig**. Source: Raycast extensions commit `6734d85992290066fd31a7f5bee3cca3d3545a1a`, path `extensions/uuid-generator`. Raycast MIT license text was fetched from that exact commit and is preserved at `licenses/raycast-MIT.txt` (also `LICENSE-MIT`). The command icon is that commit's unmodified 17477-byte PNG; see `PROVENANCE.json` and the hash inventory.

Behavior truth is the own **almatkai/uuid-generator** source/lock at `5a17980eaad254c8e95f845cd8b1002788998695`, retained byte-exact in `fixtures/original`. Its MIT license has literal backslash-n sequences in the original bytes. That file is intentionally retained unchanged, alongside the independently obtained readable upstream MIT text. Own V1 adaptation copyright is recorded there. V2 Rust port and test/integration additions: Copyright 2026 Almat, MIT, subject to the retained notices for adapted algorithms and included works. The original author and contributors are not replaced by the own manifest's author field.

TypeID validation/Base32 and nested UUIDv7 behavior were translated from **typeid-js 1.2.0** (Apache-2.0, actual license in `oracle/vendor/typeid-js/LICENSE` and `LICENSE-APACHE-2.0`) and **uuid 10.0.0** (MIT, `oracle/vendor/uuid10/LICENSE.md`). Direct UUIDv4/v5/v7 semantics come from **uuid 11.1.1** (MIT). ULID mapping/monotonic semantics come from **ulidx 2.4.1** (MIT; actual license retained), with **layerr 3.0.0** (MIT). These are the exact original lock versions, not ranges from its manifest. The oracle executes original source and those original library distributions without rewriting their algorithms. New Rust files are a translation/adaptation, not upstream originals.

Beta host common/SDK/NativeSessions/renderer fixtures are byte-exact from Tezbar `fd300e276356587726566026f072563eb9c34cb1`, under its actual Apache-2.0 license in `fixtures/beta/LICENSE`. Headless Tauri/router/registry files are explicitly new test substitutes; the original beta runtime/registry/platform fixtures are retained for comparison, not presented as host changes.

The **complete source/test distribution** aggregate SPDX expression is:

```text
MIT AND Apache-2.0 AND Unicode-3.0 AND Unicode-DFS-2016 AND W3C-20150513 AND CC-BY-4.0 AND BSD-3-Clause AND LicenseRef-W3C-Community-Final-Specification-Agreement
```

The extra terms cover validation/build materials, not a JavaScript production runtime. In particular, the vendored **TypeScript 5.9.3** compiler package is Apache-2.0 and includes its unchanged full `ThirdPartyNoticeText.txt`: DefinitelyTyped/MIT, Unicode data/Unicode-DFS-2016, DOM/W3C software/document license, WHATWG DOM/CC-BY-4.0, W3C Community Final Specification Agreement (retained as the named LicenseRef in `licenses/`), and WebGL/MIT. These original notices identify copyrights and specifications; those distributions are unmodified. **React 19.0.0** is MIT. The offline schema validator **Ajv 8.17.1**, json-schema-traverse, fast-deep-equal and require-from-string are MIT; **fast-uri 3.1.9** is BSD-3-Clause. `validation/schema-package-lock.json` pins this validation-only graph.

Locked Rust dependencies and their actual license files are enumerated in `licenses/rust-dependencies.json` and `licenses/rust/`. For alternatives, this distribution selects MIT or Apache-2.0 as offered; `unicode-ident` additionally requires Unicode-3.0. Actual alternative license texts remain supplied, including where unused targets have additional alternatives. `licenses/javascript-dependencies.json` lists each original JS package's version/license/files. Metadata does not replace the actual license texts. `provenance/files.sha256.json` verifies the immutable fixtures, icon, library distributions and notices.

No original asset was redrawn, transformed or re-encoded. No third-party author, asset ownership or real OS verification is claimed by V2. Keep these notices, license files and provenance when vendoring/distributing the adapter or its validation artifacts.
