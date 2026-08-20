#!/usr/bin/env python3
from __future__ import annotations
import hashlib,importlib.util,io,shutil,tarfile,tempfile
from pathlib import Path
R=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('b',R/'scripts/pkg01-rust-bootstrap.py')
m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)

# 1) Chunk reconstruction + tamper rejection.
with tempfile.TemporaryDirectory(prefix='pkg01-chunks-') as td:
 d=Path(td);parts=[b'a',b'b',b'c'];payload=b''.join(parts)
 old_names,old_sha=m.CHUNK_NAMES,m.SHA
 m.CHUNK_NAMES=['p0','p1','p2'];m.SHA=hashlib.sha256(payload).hexdigest()
 for n,v in zip(m.CHUNK_NAMES,parts):(d/n).write_bytes(v)
 out=d/'o.tar.xz';assert m.reconstruct_chunks(d,out) and out.read_bytes()==payload
 (d/'p1').write_bytes(b'x')
 try:m.reconstruct_chunks(d,out);raise AssertionError('tampered chunks accepted')
 except RuntimeError:pass
 m.CHUNK_NAMES,m.SHA=old_names,old_sha

# 2) End-to-end archive extraction -> install.sh -> private prefix -> exact version verification.
with tempfile.TemporaryDirectory(prefix='pkg01-install-') as td:
 td=Path(td);root=td/'rust-1.97.1-x86_64-unknown-linux-gnu';root.mkdir()
 install=root/'install.sh'
 install.write_text(r'''#!/bin/sh
set -eu
prefix=""
for a in "$@"; do case "$a" in --prefix=*) prefix="${a#--prefix=}";; esac; done
[ -n "$prefix" ] || exit 20
mkdir -p "$prefix/bin"
cat > "$prefix/bin/rustc" <<'EOF'
#!/bin/sh
echo 'rustc 1.97.1 (pkg01-integration)'
EOF
cat > "$prefix/bin/cargo" <<'EOF'
#!/bin/sh
echo 'cargo 1.97.1 (pkg01-integration)'
EOF
cat > "$prefix/bin/rustfmt" <<'EOF'
#!/bin/sh
echo 'rustfmt 1.8.0-stable (pkg01-integration)'
EOF
cat > "$prefix/bin/cargo-clippy" <<'EOF'
#!/bin/sh
echo 'clippy 0.1.97 (pkg01-integration)'
EOF
chmod +x "$prefix/bin/"*
''')
 install.chmod(0o755)
 archive=td/'synthetic-rust.tar.xz'
 with tarfile.open(archive,'w:xz') as tf: tf.add(root,arcname=root.name)
 old_sha,old_prefix=m.SHA,m.PREFIX
 m.SHA=hashlib.sha256(archive.read_bytes()).hexdigest();m.PREFIX=td/'prefix'
 m.install(archive)
 assert m.valid_install(), 'synthetic Rust distribution did not verify after install'
 # Wrong exact compiler version must be rejected.
 (m.PREFIX/'bin/rustc').write_text("#!/bin/sh\necho 'rustc 1.97.0 (pkg01-integration)'\n");(m.PREFIX/'bin/rustc').chmod(0o755)
 assert not m.valid_install(), 'wrong Rust compiler version accepted'
 m.SHA,m.PREFIX=old_sha,old_prefix

# 3) Archive path traversal must be rejected before extraction.
with tempfile.TemporaryDirectory(prefix='pkg01-unsafe-') as td:
 td=Path(td);archive=td/'unsafe.tar.xz'
 with tarfile.open(archive,'w:xz') as tf:
  info=tarfile.TarInfo('../escape');data=b'bad';info.size=len(data);tf.addfile(info,io.BytesIO(data))
 old_sha,old_prefix=m.SHA,m.PREFIX
 m.SHA=hashlib.sha256(archive.read_bytes()).hexdigest();m.PREFIX=td/'prefix'
 try:m.install(archive);raise AssertionError('unsafe archive path accepted')
 except RuntimeError as e:assert 'unsafe Rust archive path' in str(e)
 m.SHA,m.PREFIX=old_sha,old_prefix

print('PKG-01 Rust bootstrap regression: PASS')
