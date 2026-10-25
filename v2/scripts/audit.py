#!/usr/bin/env python3
"""Offline immutable inputs/icon/license hashes and, in checkout, V1/Git visibility audit."""
from pathlib import Path
import hashlib,json,subprocess
root=Path(__file__).resolve().parents[1]
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
files=json.loads((root/'provenance/files.sha256.json').read_text())
for name,expected in files.items():assert sha(root/name)==expected,name
assert (root/'assets/command-icon.png').stat().st_size==17477
assert sha(root/'assets/command-icon.png')=='423115229079dcd49a77183284e0704df85c2ecbc9bc106e14664eaaf55d95c9'
for name in ['original','beta']:
 assert any(k.startswith('fixtures/'+name+'/') for k in files)
# Git operations only when this is the explicitly owned checkout, never an outer profile.
repo=root.parent
if (repo/'.git').exists():
 assert subprocess.check_output(['git','-C',str(repo),'rev-parse','--show-toplevel'],text=True).strip()==str(repo)
 for f in (root/'fixtures/original').rglob('*'):
  if f.is_file():
   rel=f.relative_to(root/'fixtures/original');actual=repo/rel
   if str(rel)=='README.md':assert actual.read_bytes().startswith(f.read_bytes()),rel
   else:assert actual.read_bytes()==f.read_bytes(),rel
 changed=subprocess.check_output(['git','-C',str(repo),'diff','--name-only'],text=True).splitlines()
 assert set(changed)<= {'README.md'},changed
 assert not subprocess.check_output(['git','-C',str(repo),'diff','--cached','--name-only'],text=True).strip(),'Do not stage delivery'
 # Audit every deliverable, including dist dirs otherwise hidden by root ignore rules.
 paths=[]
 for p in root.rglob('*'):
  if not p.is_file() or any(part in ['.work','target','__pycache__'] for part in p.relative_to(root).parts):continue
  if p.name in ['Cargo.toml','Cargo.lock'] and 'validation' in p.relative_to(root).parts:continue
  paths.append(str(p.relative_to(repo)))
 result=subprocess.run(['git','-C',str(repo),'check-ignore','--no-index','--stdin'],input='\n'.join(paths)+'\n',capture_output=True,text=True)
 assert not result.stdout,'Hidden deliverables:\n'+result.stdout
 print(f'V1 byte parity and Git visibility checked: {len(paths)} files, no staged changes')
print(f'Immutable inputs verified: {len(files)} SHA256 hashes; exact original icon')
