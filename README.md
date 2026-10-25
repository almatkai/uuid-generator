# UUID Generator for Tezbar

Generate UUIDs, ULIDs, and TypeIDs from Tezbar on Windows or macOS.

## Commands

- **Generate UUIDs** (v4) — configurable count, uppercase and copy/paste actions.
- **Generate V5 UUIDs** — deterministic v5 from name + namespace.
- **Generate V7 UUIDs** — time-ordered v7 with optional Base32 (TypeID) encoding.
- **Generate ULIDs** — monotonic ULIDs.
- **Generate TypeIDs** — `prefix_uuid` TypeIDs from `typeid-js`.
- **Parse TypeIDs** — decode TypeID to UUID.
- **Format UUID** — add dashes to 32-hex strings.
- **Pack TypeID** — convert UUID + suffix to TypeID.
- **View History** — browse, copy, delete, and clear generated IDs (stored in `LocalStorage`).

All no-view commands copy or paste via `Clipboard` based on the `Default Action` preference; history is persisted with `LocalStorage`.

## Compatibility

- Windows: built and exercised in Tezbar on Windows 11.
- macOS: uses only cross-platform `@raycast/api` and pure JS libraries (`uuid`, `ulidx`, `typeid-js`) and retains the upstream macOS-compatible design.
- Linux: not currently declared.

## Local verification

```powershell
pnpm install
pnpm typecheck
pnpm test
pnpm build
```

## Credits

Adapted for Tezbar from Raycast's MIT-licensed `uuid-generator` extension at upstream commit `6734d85992290066fd31a7f5bee3cca3d3545a1a`. Original author `jmaeso` and contributors remain credited in `package.json`.


## Rust Native V2

The retained V1 is accompanied by a compiled-in Rust port. See [v2/README.md](v2/README.md) for behavior, integration and validation.
