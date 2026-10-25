#!/usr/bin/env python3
"""Materialize nested manifests and locked graphs after package extraction."""
from pathlib import Path
root=Path(__file__).resolve().parents[1]
for p in root.glob('validation/*/Cargo.template.toml'):
    p.with_name('Cargo.toml').write_bytes(p.read_bytes())
for p in root.glob('validation/*/Cargo.lock.template'):
    p.with_name('Cargo.lock').write_bytes(p.read_bytes())

# Cargo strips [workspace] while normalizing a .crate. Re-establish a local boundary
# so an extraction nested under another checkout is still independently testable.
manifest=root/'Cargo.toml'
text=manifest.read_text()
if '\n[workspace]\n' not in text:
    manifest.write_text(text+'\n[workspace]\nexclude = ["validation/headless", "validation/beta-host", "validation/tauri-stub"]\n')
