#!/usr/bin/env python3
"""Print deterministic bootstrap instructions for an equipped P30 runner. Does not install or fetch software itself."""
from __future__ import annotations
import argparse,json,platform,shutil
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]; VERSION=(ROOT/'VERSION').read_text().strip(); TOOLCHAIN=(ROOT/'rust-toolchain.toml').read_text().split('channel = "',1)[1].split('"',1)[0]
PLANS={
 'linux-core':{
  'commands':[f'rustup toolchain install {TOOLCHAIN} --profile minimal --component rustfmt --component clippy',f'rustup override set {TOOLCHAIN}','cargo install cargo-audit --locked','test -f Cargo.lock || cargo generate-lockfile','cd apps/desktop && test -f package-lock.json || npm install --package-lock-only --ignore-scripts --no-audit --no-fund','cd apps/desktop && npm ci --no-audit --no-fund','cd cloud/dashboard && test -f package-lock.json || npm install --package-lock-only --ignore-scripts --no-audit --no-fund','cd cloud/dashboard && npm ci --no-audit --no-fund'],
  'notes':['Requires dpkg-deb and root/sudo for real deb install/uninstall acceptance.']},
 'windows-core':{
  'commands':[f'rustup toolchain install {TOOLCHAIN} --profile minimal --component rustfmt --component clippy',f'rustup override set {TOOLCHAIN}','dotnet tool install --global wix'],
  'notes':['Authenticode additionally requires an imported code-signing certificate and VSN_WINDOWS_CERT_THUMBPRINT.']},
 'macos-core':{
  'commands':[f'rustup toolchain install {TOOLCHAIN} --profile minimal --component rustfmt --component clippy',f'rustup override set {TOOLCHAIN}'],
  'notes':['Notarization requires Developer ID Installer identity, temporary keychain and App Store Connect API credentials.']},
 'security-nightly':{
  'commands':[f'rustup toolchain install {TOOLCHAIN} --profile minimal','cargo install cargo-fuzz --locked'],
  'notes':['Runs each fuzz target for 300 seconds in the pack executor.']},
 'operations':{'commands':[],'notes':['Requires a real multi-node/live environment and explicit operator verification commands; toy/local substitutes are not certification evidence.']},
 'independent-review':{'commands':[],'notes':['Requires an independent penetration-test verification command/evidence approved by the certification reviewer.']}
}
def main():
 ap=argparse.ArgumentParser();ap.add_argument('--pack',required=True,choices=sorted(PLANS));a=ap.parse_args();p=PLANS[a.pack]
 out={'schema_version':1,'product_version':VERSION,'rust_toolchain':TOOLCHAIN,'pack_id':a.pack,'host':platform.system().lower(),'commands':p['commands'],'notes':p['notes']};print(json.dumps(out,indent=2));return 0
if __name__=='__main__':raise SystemExit(main())
