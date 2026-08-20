# Vault v2 master-key rotation

`vsn vault rotate` requires `SecretsManage` and operates under the serialized vault lock. The current vault's `key_id` is used to fetch the old key from the OS secure store. Every entry is decrypted in memory, a fresh random key ID/key is created in secure storage, every value is encrypted with a fresh nonce, and the complete v2 file is committed with the existing fsynced staged/backup write path.

The previous key is intentionally retained after a successful rotation. Destructive key retirement is a separate future recovery-policy operation; removing the old key in the same transaction would make rollback/recovery needlessly fragile.
