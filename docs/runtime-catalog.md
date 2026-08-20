# Runtime Catalog and Installation

Runtime installation is catalog-driven instead of hard-coded into VSN Core.

A release selects a platform artifact by runtime/version/OS/architecture and declares:

- HTTPS or local `file://` artifact source
- SHA-256 digest
- archive format
- executable path inside the installed runtime

Flow:

```text
catalog -> target selection -> download/copy -> SHA-256 verify -> extract -> executable verify -> registry -> shim
```

Commands:

```bash
vsn runtime catalog ./catalog.json
vsn runtime install ./catalog.json php 8.4.0
vsn runtime registry
vsn runtime activate C:\projects\app php 8.4.0
vsn runtime uninstall php 8.4.0
```

The example catalog under `providers/examples/runtime-catalog` is schema/example material. Its placeholder digest is intentionally not a trusted production runtime source.

SHA-256 confirms that an artifact matches the catalog but does not by itself establish that the catalog is trustworthy. Signed/curated catalog distribution remains a production-hardening item.
