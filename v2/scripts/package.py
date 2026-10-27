#!/usr/bin/env python3
"""Build Cargo artifact, assert shipped fixtures, extract, run real offline test harnesses."""
from pathlib import Path
import subprocess,tarfile,json,shutil
r=Path(__file__).resolve().parents[1]
# Cargo suppresses entire nested packages when a real nested Cargo.toml exists.
# Keep template sources/locks in package; temporarily remove only our generated manifests.
generated={}
for pattern in ['validation/*/Cargo.toml','validation/*/Cargo.lock']:
 for p in r.glob(pattern):generated[p]=p.read_bytes();p.unlink()
try:
 subprocess.run(['cargo','+stable','package','--allow-dirty','--locked','--offline'],cwd=r,check=True)
finally:
 for p,data in generated.items():p.write_bytes(data)
archive=r/'target/package/tezbar-uuid-generator-v2-0.1.0.crate'
dst=r/'.work/extracted';shutil.rmtree(dst,ignore_errors=True);dst.mkdir(parents=True)
with tarfile.open(archive) as t:
 names=t.getnames();prefix='tezbar-uuid-generator-v2-0.1.0/'
 for required in ['extension.json','adapter/v2_uuid_generator.rs','validation/beta-host/src/lib.rs','validation/beta-host/Cargo.template.toml','validation/headless/tests/adapter.rs','validation/headless/Cargo.template.toml','validation/headless/Cargo.lock.template','validation/tauri-stub/Cargo.template.toml','oracle/vendor/typeid-js/dist/index.js','oracle/vendor/uuid10/dist/v7.js','oracle/vendor/uuid/dist/cjs/v7.js','fixtures/beta/src-tauri/src/native_core/extensions/native.rs','fixtures/original/package-lock.json','assets/command-icon.png']:
  # uuid10 dist/v7.js exists in the actual locked package.
  assert prefix+required in names,'Not shipped: '+required
 for name in json.loads((r/'provenance/files.sha256.json').read_text()):
  assert prefix+name in names,'Immutable input missing from package: '+name
 for member in t.getmembers():
  target=(dst/member.name).resolve()
  assert target==dst.resolve() or dst.resolve() in target.parents,'Unsafe archive path'
  assert member.isfile() or member.isdir(),'Unexpected archive link/device'
 t.extractall(dst) # Paths/types validated above; compatible with the supplied Python 3.9.

extracted=dst/'tezbar-uuid-generator-v2-0.1.0'
commands=[['python3','scripts/materialize.py'],['python3','scripts/audit.py'],['node','scripts/schema.cjs'],['node','oracle/run.cjs'],['cargo','+stable','test','--locked','--offline'],['cargo','+stable','test','--manifest-path','validation/headless/Cargo.toml','--locked','--offline'],['node','validation/renderer.cjs'],['cargo','+1.85.0','test','--locked','--offline'],['cargo','+1.85.0','test','--manifest-path','validation/headless/Cargo.toml','--locked','--offline']]
for command in commands:print('EXTRACTED',' '.join(command),flush=True);subprocess.run(command,cwd=extracted,check=True)
report={'archive':archive.name,'size':archive.stat().st_size,'files':len(names),'gates':commands,'state':'pass'}
(r/'.work/package-report.json').write_text(json.dumps(report,indent=2)+'\n');print('Package/extracted offline validation passed')
