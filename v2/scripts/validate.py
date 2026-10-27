#!/usr/bin/env python3
"""Reproducible locked/offline Rust gates. Node oracle is validation-only and offline."""
from pathlib import Path
import subprocess,json,time,sys
r=Path(__file__).resolve().parents[1]
subprocess.run(['python3','scripts/materialize.py'],cwd=r,check=True)
commands=[] if '--recorded-oracle' in sys.argv else [['node','oracle/run.cjs']]
commands.append(['node','scripts/schema.cjs'])
for manifest in ['Cargo.toml','validation/headless/Cargo.toml','validation/beta-host/Cargo.toml','validation/tauri-stub/Cargo.toml']:
 commands.append(['cargo','+stable','fmt','--manifest-path',manifest,'--','--check'])
for toolchain in ['stable','1.85.0']:
 for manifest in ['Cargo.toml','validation/headless/Cargo.toml']:
  commands.append(['cargo','+'+toolchain,'test','--manifest-path',manifest,'--locked','--offline'])
 for manifest in ['Cargo.toml','validation/headless/Cargo.toml','validation/beta-host/Cargo.toml']:
  commands.append(['cargo','+'+toolchain,'clippy','--manifest-path',manifest,'--locked','--offline','--all-targets','--','-D','warnings'])
for manifest in ['Cargo.toml','validation/headless/Cargo.toml','validation/beta-host/Cargo.toml']:
 for operation in ['check','clippy']:
  cmd=['cargo','+stable',operation,'--manifest-path',manifest,'--locked','--offline','--all-targets','--target','x86_64-pc-windows-gnu']
  if operation=='clippy':cmd+=['--','-D','warnings']
  commands.append(cmd)
commands += [['node','validation/renderer.cjs'],['python3','scripts/v1.py'],['python3','scripts/audit.py']]
report=[]
for command in commands:
 print('RUN',' '.join(command),flush=True);start=time.monotonic()
 subprocess.run(command,cwd=r,check=True)
 report.append({'command':command,'state':'pass','seconds':round(time.monotonic()-start,3)})
out=r/'.work/validation-report.json';out.parent.mkdir(exist_ok=True);out.write_text(json.dumps(report,indent=2)+'\n');print(f'{len(report)} validation gates passed')
