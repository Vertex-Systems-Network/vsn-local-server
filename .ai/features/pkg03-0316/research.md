# PKG-03 03.16 Research — Idempotent reinstall and repair

Reviewed: 2026-08-30
Canonical base: `f3afb66e588d01ff2e8cb37273ad413862a4edaf`
Linear: `ABD-91`
Change required: **false (certification-first)**

## Canonical findings

- Canonical PKG-03 is `15/25 = 60%`, active task `03.16`, and dependencies `03.11`, `03.12`, `03.14`, and `03.15` are DONE.
- 03.14 already certifies detection of `MATCH`, `MISSING`, and `HASH_MISMATCH` for accepted owned executables, but explicitly does not execute installer repair. 03.16 owns the first genuine repair/reinstall lifecycle.
- Microsoft documents native Windows Installer repair through `msiexec /f...`. `/fa <ProductCode>` forces all files in the installed product to be reinstalled. `REINSTALL=ALL` together with `REINSTALLMODE` is the property-level equivalent when reinstall behavior must be selected explicitly.
- Microsoft also documents that reinstalling an application may reinstall services, environment variables, custom actions, registry data, and shortcuts belonging to the selected feature. 03.16 must therefore verify accepted service/ACL/registration invariants rather than treating file restoration alone as sufficient.
- `REINSTALLMODE=c` repairs checksum mismatches only for files that carry MSI checksum metadata. 03.16 must not assume every Tauri/WiX payload file has that attribute. The bounded MSI repair contract therefore uses force-reinstall semantics for the damaged-file repair proof and independently verifies exact SHA-256 restoration.
- Tauri 2 continues to produce Windows `.msi` packages through WiX and `-setup.exe` through NSIS. The official Tauri installer documentation does not define a separate NSIS repair API or guarantee same-version repair semantics. NSIS same-package reinstall behavior must be proven by the exact generated installer, not assumed.
- 03.19 owns running Desktop/CLI/Agent coordination. 03.16 will run repair against a quiescent product state. For per-machine cases the Agent service must be stopped before destructive payload probes; running-service/file-in-use repair is a nonclaim.
- 03.17 owns comprehensive dirty-user-data uninstall preservation. 03.16 may verify that repair does not relocate or permission-widen accepted state/security locations, but it must not claim the 03.17 dirty-data matrix.
- 03.18 owns transactional failure rollback and interrupted-install recovery; 03.20 owns reboot semantics; 03.21 owns unattended deployment. None are implied by a successful repair run.

Official references:
- https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/msiexec
- https://learn.microsoft.com/en-us/windows/win32/msi/reinstall
- https://learn.microsoft.com/en-us/windows/win32/msi/reinstallmode
- https://learn.microsoft.com/en-us/windows/win32/msi/reinstalling-a-feature-or-application
- https://v2.tauri.app/distribute/windows-installer/

## Frozen repair model

1. **Healthy idempotence**
   - install the exact candidate;
   - capture accepted payload hashes and installation identity;
   - execute same-version reinstall/repair while the product is quiescent;
   - require success and prove payload hashes, install root, application identity, shortcut/ARP cardinality, and allowed service/ACL invariants remain stable.
2. **Missing-file repair**
   - remove an allowed owned executable after a healthy install;
   - prove the pre-repair state is `MISSING`;
   - invoke the format-specific genuine reinstall/repair path;
   - require exact package-owned bytes to be restored and classify `MATCH`.
3. **Tampered-file repair**
   - replace an allowed owned executable with deterministic non-matching bytes;
   - prove the pre-repair state is `HASH_MISMATCH`;
   - invoke genuine reinstall/repair;
   - require exact candidate bytes and `MATCH`.
4. **Second healthy pass**
   - run the same reinstall/repair once more after successful restoration;
   - require no duplicate registration/service/shortcut/ARP state and no hash drift.
5. **Format-specific boundary**
   - NSIS current-user may destructively probe Desktop, CLI, and Agent because no machine service is installed;
   - NSIS per-machine and MSI/WiX destructively probe Desktop + CLI only; Agent remains read-only while service identity/health is verified with the service intentionally quiesced during repair;
   - MSI repair uses documented Windows Installer repair semantics and evidence-bound verbose logs;
   - NSIS uses the exact generated setup executable and must fail closed if same-version rerun does not genuinely restore the damaged file.

No product/config/template/toolchain mutation is authorized by this planning conclusion. `change_required=false` means certification-first; a genuine exact-head failure may justify a separate minimum-scope change-control decision, but may not silently widen 03.16.
