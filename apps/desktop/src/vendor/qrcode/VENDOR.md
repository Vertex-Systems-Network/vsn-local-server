# Vendored qrcode-generator

- Package: `qrcode-generator@2.0.4` (npm), file `dist/qrcode.mjs` copied unmodified as `qrcode.js`.
- License: MIT, Copyright (c) 2009 Kazuhiko Arase (header kept in the file).
- Tarball SHA-256: `02e2e18a99a90b02dad940851f59b7c3c5fd1ab79cbdece8595cb06328878159`
- `qrcode.js` SHA-256: `ea91d7118a5395289170da848b7c6758b996163bfbccf312591ab65a4911b7c0`
- Why vendored: `apps/desktop/package-lock.json` is hash-pinned by the frozen PKG-01 gates (01.13–01.15),
  same as GSAP. Used only to draw share-link QR codes (Networking → Share).
