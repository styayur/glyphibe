# SPDX-License-Identifier: MPL-2.0
"""Keep Rust, Tauri and npm package versions consistent without creating tags."""
import json
from pathlib import Path
import tomllib

root=Path(__file__).resolve().parent.parent
rust=tomllib.loads((root/'Cargo.toml').read_text())['workspace']['package']['version']
versions={
 'workspace':rust,
 'desktop':tomllib.loads((root/'apps/desktop/src-tauri/Cargo.toml').read_text())['package']['version'],
 'tauri':json.loads((root/'apps/desktop/src-tauri/tauri.conf.json').read_text())['version'],
 'npm':json.loads((root/'apps/desktop/package.json').read_text())['version'],
 'npm-lock':json.loads((root/'apps/desktop/package-lock.json').read_text())['packages']['']['version'],
}
if len(set(versions.values()))!=1:raise SystemExit('Version mismatch: '+str(versions))
print(json.dumps(versions))
