# Vault key lifecycle — 0.19

`vault rotate` decrypts the current generation, persists an encrypted recovery snapshot, creates a fresh secure-store key ID, re-encrypts every secret with fresh nonces, atomically writes the new generation, and records key history.

`vault restore <key-id> true` verifies the retained secure-store key can decrypt the selected recovery generation before replacing the current vault. The current generation is first preserved as another recovery snapshot.

`vault retire <key-id> true` rejects the current key, verifies the current vault is decryptable, removes the selected recovery snapshot and asks the OS secure store to delete that retired recovery credential. Retired generations cannot be restored through the normal lifecycle.
