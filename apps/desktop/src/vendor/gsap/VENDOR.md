# Vendored GSAP

- Package: `gsap@3.12.7` (npm), files `index.js`, `gsap-core.js`, `CSSPlugin.js` copied unmodified.
- License: GreenSock Standard "no charge" license — https://gsap.com/standard-license
- Why vendored: `apps/desktop/package-lock.json` is hash-pinned by the frozen PKG-01 gates
  (01.13–01.15). Adding an npm dependency would change the certified lock, so GSAP is vendored instead.
  Move it to `package.json` when the lock is next re-certified.

| File | SHA-256 |
|---|---|
| index.js | 070038235ba75ec2186d054ebd83ae94e6de6a971f5d3f3d6cd1037551f94faa |
| gsap-core.js | 44e9466e0cdb479c85d940851e9afdc675f2543f9006f24259cbd1030cf90125 |
| CSSPlugin.js | fd8816d2b4be2bc4879fc046a87fc6f434828a3d4088405fc5b49baf2e7a0293 |
