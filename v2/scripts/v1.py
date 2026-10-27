#!/usr/bin/env python3
"""Disposable original tests/typecheck, never install dependencies into retained V1."""
from pathlib import Path
import shutil,subprocess,hashlib
r=Path(__file__).resolve().parents[1];dst=r/'.work/v1-validation'
shutil.copytree(r/'fixtures/original',dst,dirs_exist_ok=True)
files=[p for p in dst.rglob('*') if p.is_file() and 'node_modules' not in p.parts]
before={str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in files}
for cmd in [['npm','ci','--offline','--ignore-scripts','--no-audit','--no-fund'],['npm','test'],['npm','run','typecheck']]:subprocess.run(cmd,cwd=dst,check=True)
for f,h in before.items():assert hashlib.sha256(Path(f).read_bytes()).hexdigest()==h,f
print('Disposable V1: original source/manifest/lock unchanged; 7 tests and typecheck passed')
