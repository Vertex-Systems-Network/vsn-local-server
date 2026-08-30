# PKG-03 03.19 — Running Process & Restart Manager Coordination Contract v1

Canonical base: `f3afb66e588d01ff2e8cb37273ad413862a4edaf`
Task: `03.19`
Linear: `ABD-94`

## Contract

Installer behavior with active VSN resources must be explicit and safe. The installer may coordinate a bounded shutdown/service stop and complete, or it may block deterministically before destructive mutation. It may not hang indefinitely, silently force-kill user processes, or leave partial package state.

## Evidence identity

Every running Desktop/CLI process must be proven to originate from the exact installed path and hash. The per-machine Agent must match the accepted `VSN-Agent` service identity/account/path before the operation.

## Harness non-interference

The certification harness does not pre-stop or pre-kill the product to make the installer pass. Harness cleanup is allowed only after a failed/blocked observation and is separately evidence-tagged.

## MSI Restart Manager rule

Verbose Windows Installer evidence must show the effective Restart Manager path or an explicit policy-driven alternative. Default behavior is not assumed merely from package format.

## Service rule

If the product remains installed, the Agent service identity/configuration must remain invariant and return to the expected state. After accepted uninstall, service registration must be absent.

## Nonclaims

No reboot policy, silent deployment, signing, updater/recovery, or cross-platform claim is made here.
