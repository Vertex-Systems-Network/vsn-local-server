use native_tls::{Certificate, TlsConnector};
use postgres::config::SslMode;
use postgres::{Client as PgClient, Config as PgConfig};
use postgres_native_tls::MakeTlsConnector;
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("control store sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("control store I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("control store PostgreSQL error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("control store TLS error: {0}")]
    Tls(#[from] native_tls::Error),
    #[error("control store snapshot hash mismatch")]
    HashMismatch,
    #[error("control store invalid request: {0}")]
    Invalid(String),
    #[error("control store generation conflict: expected {expected}, actual {actual}")]
    GenerationConflict { expected: u64, actual: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ClusterInstance {
    pub instance_id: String,
    pub endpoint: String,
    pub updated_at_unix_ms: u128,
    pub expires_at_unix_ms: u128,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ClusterBusMessage {
    pub id: i64,
    pub source_instance_id: String,
    pub target_instance_id: String,
    pub topic: String,
    pub payload: String,
    pub created_at_unix_ms: u128,
    pub expires_at_unix_ms: u128,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SharedStreamCheckpoint {
    pub relay_id: String,
    pub device_id: String,
    pub principal_id: String,
    pub permission: String,
    pub request_json: String,
    pub agent_instance_id: String,
    pub resume_token_hash: String,
    pub resource_id: Option<String>,
    pub next_input_seq: u64,
    pub acked_input_seq: u64,
    pub committed_bytes: Option<u64>,
    pub resource_progress_bytes: u64,
    pub created_at_unix_ms: u128,
    pub last_activity_unix_ms: u128,
    pub detached_until_unix_ms: Option<u128>,
    pub expires_at_unix_ms: u128,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SharedStreamFrame {
    pub relay_id: String,
    pub seq: u64,
    pub frame_json: String,
    pub created_at_unix_ms: u128,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SharedSessionRecord {
    pub session_id: String,
    pub account_id: String,
    pub token_hash: String,
    pub payload: String,
    pub created_at_unix_ms: u128,
    pub expires_at_unix_ms: u128,
    pub last_activity_unix_ms: u128,
    pub revoked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SharedRoleRecord {
    pub role_id: String,
    pub payload: String,
    pub updated_at_unix_ms: u128,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SharedAccountRecord {
    pub account_id: String,
    pub email: String,
    pub role_id: String,
    pub payload: String,
    pub disabled: bool,
    pub updated_at_unix_ms: u128,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SharedAuthTransaction {
    pub transaction_id: String,
    pub kind: String,
    pub payload: String,
    pub created_at_unix_ms: u128,
    pub expires_at_unix_ms: u128,
    pub consumed_at_unix_ms: Option<u128>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SharedAuthPolicyRecord {
    pub policy_id: String,
    pub payload: String,
    pub updated_at_unix_ms: u128,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SharedScimGroupRecord {
    pub group_id: String,
    pub display_name: String,
    pub payload: String,
    pub updated_at_unix_ms: u128,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SharedApiTokenRecord {
    pub token_id: String,
    pub principal_id: String,
    pub role_id: String,
    pub token_hash: String,
    pub created_at_unix_ms: u128,
    pub revoked: bool,
    pub updated_at_unix_ms: u128,
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SharedFleetGroupRecord {
    pub group_id: String,
    pub payload: String,
    pub updated_at_unix_ms: u128,
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SharedEnvironmentRecord {
    pub environment_id: String,
    pub payload: String,
    pub updated_at_unix_ms: u128,
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SharedDeviceFleetRecord {
    pub device_id: String,
    pub payload: String,
    pub updated_at_unix_ms: u128,
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SharedTeamSecretRecord {
    pub name: String,
    pub key_id: String,
    pub nonce_b64: String,
    pub ciphertext_b64: String,
    pub created_by: String,
    pub updated_at_unix_ms: u128,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SharedDeviceRecord {
    pub device_id: String,
    pub public_key: String,
    pub display_name: String,
    pub os: String,
    pub enrolled_at_unix_ms: u128,
    pub last_seen_unix_ms: u128,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SharedCommandRecord {
    pub command_id: String,
    pub device_id: String,
    pub payload: String,
    pub state: String,
    pub attempts: u32,
    pub leased_by: Option<String>,
    pub lease_until_unix_ms: Option<u128>,
    pub expires_at_unix_ms: u128,
    pub created_at_unix_ms: u128,
    pub completed_at_unix_ms: Option<u128>,
    pub last_error: Option<String>,
    pub result_payload: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SharedApprovalRecord {
    pub approval_id: String,
    pub payload: String,
    pub state: String,
    pub created_at_unix_ms: u128,
    pub expires_at_unix_ms: u128,
    pub approver_id: Option<String>,
    pub decided_at_unix_ms: Option<u128>,
    pub command_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SharedAuditInput {
    pub event_id: String,
    pub previous_hash: String,
    pub event_hash: String,
    pub timestamp_unix_ms: u128,
    pub payload: String,
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SharedAuditRecord {
    pub seq: i64,
    pub device_id: String,
    pub event_id: String,
    pub previous_hash: String,
    pub event_hash: String,
    pub timestamp_unix_ms: u128,
    pub payload: String,
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SharedAuditAppendResult {
    pub accepted: u32,
    pub duplicates: u32,
    pub last_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub name: String,
    pub generation: u64,
    pub payload: Vec<u8>,
    pub sha256: String,
    pub updated_at_unix_ms: u128,
}

#[derive(Debug, Clone)]
pub struct SnapshotStore {
    path: PathBuf,
}

impl SnapshotStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
            harden_dir(parent)?;
        }
        let store = Self { path };
        let conn = store.connection()?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;\n\
             PRAGMA synchronous=FULL;\n\
             PRAGMA foreign_keys=ON;\n\
             CREATE TABLE IF NOT EXISTS snapshots (\n\
               name TEXT PRIMARY KEY,\n\
               generation INTEGER NOT NULL,\n\
               payload BLOB NOT NULL,\n\
               sha256 TEXT NOT NULL,\n\
               updated_at_unix_ms TEXT NOT NULL\n\
             );",
        )?;
        harden_file(&store.path)?;
        Ok(store)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self, name: &str) -> Result<Option<Snapshot>, StoreError> {
        validate_name(name)?;
        let conn = self.connection()?;
        let row = conn
            .query_row(
                "SELECT generation,payload,sha256,updated_at_unix_ms FROM snapshots WHERE name=?1",
                params![name],
                |row| {
                    let generation: i64 = row.get(0)?;
                    let payload: Vec<u8> = row.get(1)?;
                    let sha256: String = row.get(2)?;
                    let updated: String = row.get(3)?;
                    Ok((generation, payload, sha256, updated))
                },
            )
            .optional()?;
        let Some((generation, payload, sha256, updated)) = row else {
            return Ok(None);
        };
        if sha256_hex(&payload) != sha256 {
            return Err(StoreError::HashMismatch);
        }
        Ok(Some(Snapshot {
            name: name.into(),
            generation: u64::try_from(generation)
                .map_err(|_| StoreError::Invalid("negative generation".into()))?,
            payload,
            sha256,
            updated_at_unix_ms: updated
                .parse::<u128>()
                .map_err(|_| StoreError::Invalid("invalid timestamp".into()))?,
        }))
    }

    pub fn save(&self, name: &str, payload: &[u8]) -> Result<Snapshot, StoreError> {
        validate_name(name)?;
        if payload.len() > 128 * 1024 * 1024 {
            return Err(StoreError::Invalid("snapshot exceeds 128 MiB".into()));
        }
        let sha256 = sha256_hex(payload);
        let updated = now_ms();
        let mut conn = self.connection()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current: Option<i64> = tx
            .query_row(
                "SELECT generation FROM snapshots WHERE name=?1",
                params![name],
                |row| row.get(0),
            )
            .optional()?;
        let generation = current.unwrap_or(0).saturating_add(1);
        tx.execute(
            "INSERT INTO snapshots(name,generation,payload,sha256,updated_at_unix_ms) VALUES(?1,?2,?3,?4,?5)\n\
             ON CONFLICT(name) DO UPDATE SET generation=excluded.generation,payload=excluded.payload,sha256=excluded.sha256,updated_at_unix_ms=excluded.updated_at_unix_ms",
            params![name,generation,payload,sha256,updated.to_string()],
        )?;
        tx.commit()?;
        harden_file(&self.path)?;
        Ok(Snapshot {
            name: name.into(),
            generation: generation as u64,
            payload: payload.to_vec(),
            sha256,
            updated_at_unix_ms: updated,
        })
    }

    pub fn save_if_generation(
        &self,
        name: &str,
        expected_generation: u64,
        payload: &[u8],
    ) -> Result<Snapshot, StoreError> {
        validate_name(name)?;
        if payload.len() > 128 * 1024 * 1024 {
            return Err(StoreError::Invalid("snapshot exceeds 128 MiB".into()));
        }
        let sha256 = sha256_hex(payload);
        let updated = now_ms();
        let mut conn = self.connection()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current: Option<i64> = tx
            .query_row(
                "SELECT generation FROM snapshots WHERE name=?1",
                params![name],
                |row| row.get(0),
            )
            .optional()?;
        let actual = u64::try_from(current.unwrap_or(0))
            .map_err(|_| StoreError::Invalid("negative generation".into()))?;
        if actual != expected_generation {
            return Err(StoreError::GenerationConflict {
                expected: expected_generation,
                actual,
            });
        }
        let generation = actual.saturating_add(1);
        if actual == 0 {
            tx.execute("INSERT INTO snapshots(name,generation,payload,sha256,updated_at_unix_ms) VALUES(?1,?2,?3,?4,?5)", params![name,generation as i64,payload,sha256,updated.to_string()])?;
        } else {
            let changed=tx.execute("UPDATE snapshots SET generation=?2,payload=?3,sha256=?4,updated_at_unix_ms=?5 WHERE name=?1 AND generation=?6", params![name,generation as i64,payload,sha256,updated.to_string(),actual as i64])?;
            if changed != 1 {
                let latest: Option<i64> = tx
                    .query_row(
                        "SELECT generation FROM snapshots WHERE name=?1",
                        params![name],
                        |row| row.get(0),
                    )
                    .optional()?;
                let latest = u64::try_from(latest.unwrap_or(0)).unwrap_or(0);
                return Err(StoreError::GenerationConflict {
                    expected: expected_generation,
                    actual: latest,
                });
            }
        }
        tx.commit()?;
        harden_file(&self.path)?;
        Ok(Snapshot {
            name: name.into(),
            generation,
            payload: payload.to_vec(),
            sha256,
            updated_at_unix_ms: updated,
        })
    }

    fn connection(&self) -> Result<Connection, StoreError> {
        let conn = Connection::open(&self.path)?;
        conn.busy_timeout(std::time::Duration::from_secs(10))?;
        Ok(conn)
    }
}

#[derive(Debug, Clone)]
pub struct PostgresSnapshotStore {
    connection_string: String,
    root_ca_pem_path: PathBuf,
}

impl PostgresSnapshotStore {
    pub fn open(
        connection_string: impl Into<String>,
        root_ca_pem_path: impl AsRef<Path>,
    ) -> Result<Self, StoreError> {
        let connection_string = connection_string.into();
        if !(connection_string.starts_with("postgres://")
            || connection_string.starts_with("postgresql://"))
        {
            return Err(StoreError::Invalid(
                "PostgreSQL snapshot store requires postgres:// or postgresql:// DSN".into(),
            ));
        }
        let root_ca_pem_path = validate_root_ca(root_ca_pem_path.as_ref())?;
        let store = Self {
            connection_string,
            root_ca_pem_path,
        };
        let mut client = store.connection()?;
        client.batch_execute(
            "CREATE TABLE IF NOT EXISTS vsn_control_snapshots (
               name TEXT PRIMARY KEY,
               generation BIGINT NOT NULL,
               payload BYTEA NOT NULL,
               sha256 TEXT NOT NULL,
               updated_at_unix_ms TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS vsn_control_instances (
               instance_id TEXT PRIMARY KEY,
               endpoint TEXT NOT NULL,
               updated_at_unix_ms BIGINT NOT NULL,
               expires_at_unix_ms BIGINT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS vsn_control_routes (
               route_type TEXT NOT NULL,
               route_key TEXT NOT NULL,
               instance_id TEXT NOT NULL,
               expires_at_unix_ms BIGINT NOT NULL,
               PRIMARY KEY(route_type,route_key)
             );
             CREATE TABLE IF NOT EXISTS vsn_control_bus (
               id BIGSERIAL PRIMARY KEY,
               source_instance_id TEXT NOT NULL,
               target_instance_id TEXT NOT NULL,
               topic TEXT NOT NULL,
               payload TEXT NOT NULL,
               created_at_unix_ms BIGINT NOT NULL,
               expires_at_unix_ms BIGINT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS vsn_control_bus_target_idx ON vsn_control_bus(target_instance_id,id);
             CREATE INDEX IF NOT EXISTS vsn_control_bus_expiry_idx ON vsn_control_bus(expires_at_unix_ms);
             CREATE TABLE IF NOT EXISTS vsn_control_commands (
               command_id TEXT PRIMARY KEY,
               device_id TEXT NOT NULL,
               payload TEXT NOT NULL,
               state TEXT NOT NULL,
               attempts INTEGER NOT NULL DEFAULT 0,
               leased_by TEXT,
               lease_until_unix_ms BIGINT,
               expires_at_unix_ms BIGINT NOT NULL,
               created_at_unix_ms BIGINT NOT NULL,
               completed_at_unix_ms BIGINT,
               last_error TEXT,
               result_payload TEXT
             );
             CREATE INDEX IF NOT EXISTS vsn_control_commands_device_idx ON vsn_control_commands(device_id,state,created_at_unix_ms);
             CREATE INDEX IF NOT EXISTS vsn_control_commands_lease_idx ON vsn_control_commands(state,lease_until_unix_ms);
             ALTER TABLE vsn_control_commands ADD COLUMN IF NOT EXISTS result_payload TEXT;
             CREATE TABLE IF NOT EXISTS vsn_control_devices (
               device_id TEXT PRIMARY KEY, public_key TEXT NOT NULL, display_name TEXT NOT NULL, os TEXT NOT NULL,
               enrolled_at_unix_ms BIGINT NOT NULL, last_seen_unix_ms BIGINT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS vsn_control_pairings (
               pairing_nonce TEXT PRIMARY KEY, expires_at_unix_ms BIGINT NOT NULL, consumed_at_unix_ms BIGINT
             );
             CREATE INDEX IF NOT EXISTS vsn_control_pairings_expiry_idx ON vsn_control_pairings(expires_at_unix_ms);
             CREATE TABLE IF NOT EXISTS vsn_control_rate_limits (
               rate_key TEXT NOT NULL, window_start_unix_ms BIGINT NOT NULL, count BIGINT NOT NULL, expires_at_unix_ms BIGINT NOT NULL,
               PRIMARY KEY(rate_key,window_start_unix_ms)
             );
             CREATE INDEX IF NOT EXISTS vsn_control_rate_limits_expiry_idx ON vsn_control_rate_limits(expires_at_unix_ms);
             CREATE TABLE IF NOT EXISTS vsn_control_approvals (
               approval_id TEXT PRIMARY KEY, payload TEXT NOT NULL, state TEXT NOT NULL, created_at_unix_ms BIGINT NOT NULL, expires_at_unix_ms BIGINT NOT NULL, approver_id TEXT, decided_at_unix_ms BIGINT, command_id TEXT
             );
             CREATE INDEX IF NOT EXISTS vsn_control_approvals_state_idx ON vsn_control_approvals(state,created_at_unix_ms DESC);
             CREATE INDEX IF NOT EXISTS vsn_control_approvals_expiry_idx ON vsn_control_approvals(expires_at_unix_ms);
             CREATE TABLE IF NOT EXISTS vsn_control_audit (
               seq BIGSERIAL PRIMARY KEY, device_id TEXT NOT NULL, event_id TEXT NOT NULL, previous_hash TEXT NOT NULL, event_hash TEXT NOT NULL, timestamp_unix_ms BIGINT NOT NULL, payload TEXT NOT NULL,
               UNIQUE(device_id,event_id)
             );
             CREATE INDEX IF NOT EXISTS vsn_control_audit_device_seq_idx ON vsn_control_audit(device_id,seq DESC);
             CREATE INDEX IF NOT EXISTS vsn_control_audit_time_idx ON vsn_control_audit(timestamp_unix_ms DESC);
             CREATE TABLE IF NOT EXISTS vsn_control_stream_relays (
               relay_id TEXT PRIMARY KEY, device_id TEXT NOT NULL, principal_id TEXT NOT NULL, permission TEXT NOT NULL, request_json TEXT NOT NULL,
               agent_instance_id TEXT NOT NULL, resume_token_hash TEXT NOT NULL, resource_id TEXT, next_input_seq BIGINT NOT NULL, acked_input_seq BIGINT NOT NULL,
               committed_bytes BIGINT, resource_progress_bytes BIGINT NOT NULL DEFAULT 0, created_at_unix_ms BIGINT NOT NULL, last_activity_unix_ms BIGINT NOT NULL,
               detached_until_unix_ms BIGINT, expires_at_unix_ms BIGINT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS vsn_control_stream_relays_device_idx ON vsn_control_stream_relays(device_id,expires_at_unix_ms);
             CREATE INDEX IF NOT EXISTS vsn_control_stream_relays_expiry_idx ON vsn_control_stream_relays(expires_at_unix_ms);
             CREATE TABLE IF NOT EXISTS vsn_control_stream_frames (
               relay_id TEXT NOT NULL REFERENCES vsn_control_stream_relays(relay_id) ON DELETE CASCADE, seq BIGINT NOT NULL, frame_json TEXT NOT NULL, created_at_unix_ms BIGINT NOT NULL,
               PRIMARY KEY(relay_id,seq)
             );
             CREATE INDEX IF NOT EXISTS vsn_control_stream_frames_relay_idx ON vsn_control_stream_frames(relay_id,seq);
             CREATE TABLE IF NOT EXISTS vsn_control_sessions (
               session_id TEXT PRIMARY KEY, account_id TEXT NOT NULL, token_hash TEXT NOT NULL UNIQUE, payload TEXT NOT NULL, created_at_unix_ms BIGINT NOT NULL, expires_at_unix_ms BIGINT NOT NULL, last_activity_unix_ms BIGINT NOT NULL, revoked BOOLEAN NOT NULL DEFAULT FALSE
             );
             CREATE INDEX IF NOT EXISTS vsn_control_sessions_account_idx ON vsn_control_sessions(account_id,revoked,expires_at_unix_ms);
             CREATE INDEX IF NOT EXISTS vsn_control_sessions_expiry_idx ON vsn_control_sessions(expires_at_unix_ms);
             CREATE TABLE IF NOT EXISTS vsn_control_roles (
               role_id TEXT PRIMARY KEY, payload TEXT NOT NULL, updated_at_unix_ms BIGINT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS vsn_control_accounts (
               account_id TEXT PRIMARY KEY, email TEXT NOT NULL UNIQUE, role_id TEXT NOT NULL, payload TEXT NOT NULL, disabled BOOLEAN NOT NULL DEFAULT FALSE, updated_at_unix_ms BIGINT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS vsn_control_accounts_role_idx ON vsn_control_accounts(role_id,disabled);
             CREATE TABLE IF NOT EXISTS vsn_control_auth_transactions (
               transaction_id TEXT PRIMARY KEY, kind TEXT NOT NULL, payload TEXT NOT NULL, created_at_unix_ms BIGINT NOT NULL, expires_at_unix_ms BIGINT NOT NULL, consumed_at_unix_ms BIGINT
             );
             CREATE INDEX IF NOT EXISTS vsn_control_auth_transactions_expiry_idx ON vsn_control_auth_transactions(expires_at_unix_ms,consumed_at_unix_ms);
             CREATE TABLE IF NOT EXISTS vsn_control_auth_policy (
               policy_id TEXT PRIMARY KEY, payload TEXT NOT NULL, updated_at_unix_ms BIGINT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS vsn_control_scim_groups (
               group_id TEXT PRIMARY KEY, display_name TEXT NOT NULL, payload TEXT NOT NULL, updated_at_unix_ms BIGINT NOT NULL
             );
             CREATE UNIQUE INDEX IF NOT EXISTS vsn_control_scim_groups_name_idx ON vsn_control_scim_groups(display_name);
             CREATE TABLE IF NOT EXISTS vsn_control_api_tokens (
               token_id TEXT PRIMARY KEY, principal_id TEXT NOT NULL, role_id TEXT NOT NULL, token_hash TEXT NOT NULL UNIQUE, created_at_unix_ms BIGINT NOT NULL, revoked BOOLEAN NOT NULL DEFAULT FALSE, updated_at_unix_ms BIGINT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS vsn_control_api_tokens_principal_idx ON vsn_control_api_tokens(principal_id,revoked);
             CREATE TABLE IF NOT EXISTS vsn_control_fleet_groups (
               group_id TEXT PRIMARY KEY, payload TEXT NOT NULL, updated_at_unix_ms BIGINT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS vsn_control_environments (
               environment_id TEXT PRIMARY KEY, payload TEXT NOT NULL, updated_at_unix_ms BIGINT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS vsn_control_device_fleet (
               device_id TEXT PRIMARY KEY, payload TEXT NOT NULL, updated_at_unix_ms BIGINT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS vsn_control_team_secrets (
               name TEXT PRIMARY KEY, key_id TEXT NOT NULL DEFAULT 'legacy', nonce_b64 TEXT NOT NULL, ciphertext_b64 TEXT NOT NULL, created_by TEXT NOT NULL, updated_at_unix_ms BIGINT NOT NULL
             );
             ALTER TABLE vsn_control_team_secrets ADD COLUMN IF NOT EXISTS key_id TEXT NOT NULL DEFAULT 'legacy';
             CREATE INDEX IF NOT EXISTS vsn_control_team_secrets_updated_idx ON vsn_control_team_secrets(updated_at_unix_ms DESC);
             CREATE TABLE IF NOT EXISTS vsn_control_team_vault_meta (
               singleton BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK(singleton), active_key_id TEXT NOT NULL, updated_at_unix_ms BIGINT NOT NULL
             );"
        )?;
        Ok(store)
    }

    pub fn load(&self, name: &str) -> Result<Option<Snapshot>, StoreError> {
        validate_name(name)?;
        let mut client = self.connection()?;
        let row = client.query_opt(
            "SELECT generation,payload,sha256,updated_at_unix_ms FROM vsn_control_snapshots WHERE name=$1",
            &[&name],
        )?;
        let Some(row) = row else {
            return Ok(None);
        };
        snapshot_from_pg_row(name, row)
    }

    pub fn save(&self, name: &str, payload: &[u8]) -> Result<Snapshot, StoreError> {
        validate_name(name)?;
        validate_payload(payload)?;
        let sha256 = sha256_hex(payload);
        let updated = now_ms();
        let mut client = self.connection()?;
        let mut tx = client.transaction()?;
        tx.query_one(
            "SELECT pg_advisory_xact_lock(hashtext($1)::bigint)",
            &[&name],
        )?;
        let current = tx
            .query_opt(
                "SELECT generation FROM vsn_control_snapshots WHERE name=$1",
                &[&name],
            )?
            .map(|r| r.get::<_, i64>(0))
            .unwrap_or(0);
        let generation = u64::try_from(current)
            .map_err(|_| StoreError::Invalid("negative generation".into()))?
            .saturating_add(1);
        tx.execute(
            "INSERT INTO vsn_control_snapshots(name,generation,payload,sha256,updated_at_unix_ms) VALUES($1,$2,$3,$4,$5)
             ON CONFLICT(name) DO UPDATE SET generation=EXCLUDED.generation,payload=EXCLUDED.payload,sha256=EXCLUDED.sha256,updated_at_unix_ms=EXCLUDED.updated_at_unix_ms",
            &[&name, &(generation as i64), &payload, &sha256, &updated.to_string()],
        )?;
        tx.commit()?;
        Ok(Snapshot {
            name: name.into(),
            generation,
            payload: payload.to_vec(),
            sha256,
            updated_at_unix_ms: updated,
        })
    }

    pub fn save_if_generation(
        &self,
        name: &str,
        expected_generation: u64,
        payload: &[u8],
    ) -> Result<Snapshot, StoreError> {
        validate_name(name)?;
        validate_payload(payload)?;
        let sha256 = sha256_hex(payload);
        let updated = now_ms();
        let mut client = self.connection()?;
        let mut tx = client.transaction()?;
        tx.query_one(
            "SELECT pg_advisory_xact_lock(hashtext($1)::bigint)",
            &[&name],
        )?;
        let current = tx
            .query_opt(
                "SELECT generation FROM vsn_control_snapshots WHERE name=$1 FOR UPDATE",
                &[&name],
            )?
            .map(|r| r.get::<_, i64>(0))
            .unwrap_or(0);
        let actual = u64::try_from(current)
            .map_err(|_| StoreError::Invalid("negative generation".into()))?;
        if actual != expected_generation {
            return Err(StoreError::GenerationConflict {
                expected: expected_generation,
                actual,
            });
        }
        let generation = actual.saturating_add(1);
        if actual == 0 {
            tx.execute(
                "INSERT INTO vsn_control_snapshots(name,generation,payload,sha256,updated_at_unix_ms) VALUES($1,$2,$3,$4,$5)",
                &[&name, &(generation as i64), &payload, &sha256, &updated.to_string()],
            )?;
        } else {
            let changed = tx.execute(
                "UPDATE vsn_control_snapshots SET generation=$2,payload=$3,sha256=$4,updated_at_unix_ms=$5 WHERE name=$1 AND generation=$6",
                &[&name, &(generation as i64), &payload, &sha256, &updated.to_string(), &(actual as i64)],
            )?;
            if changed != 1 {
                return Err(StoreError::GenerationConflict {
                    expected: expected_generation,
                    actual,
                });
            }
        }
        tx.commit()?;
        Ok(Snapshot {
            name: name.into(),
            generation,
            payload: payload.to_vec(),
            sha256,
            updated_at_unix_ms: updated,
        })
    }

    pub fn heartbeat_instance(
        &self,
        instance_id: &str,
        endpoint: &str,
        ttl_ms: u64,
    ) -> Result<(), StoreError> {
        validate_name(instance_id)?;
        if endpoint.len() > 512 || endpoint.chars().any(char::is_control) {
            return Err(StoreError::Invalid("invalid instance endpoint".into()));
        }
        let now = now_i64();
        let expires =
            now.saturating_add(i64::try_from(ttl_ms.clamp(5_000, 300_000)).unwrap_or(300_000));
        let mut client = self.connection()?;
        client.execute("INSERT INTO vsn_control_instances(instance_id,endpoint,updated_at_unix_ms,expires_at_unix_ms) VALUES($1,$2,$3,$4) ON CONFLICT(instance_id) DO UPDATE SET endpoint=EXCLUDED.endpoint,updated_at_unix_ms=EXCLUDED.updated_at_unix_ms,expires_at_unix_ms=EXCLUDED.expires_at_unix_ms",&[&instance_id,&endpoint,&now,&expires])?;
        client.execute(
            "DELETE FROM vsn_control_instances WHERE expires_at_unix_ms < $1",
            &[&now],
        )?;
        Ok(())
    }
    pub fn live_instances(&self) -> Result<Vec<ClusterInstance>, StoreError> {
        let now = now_i64();
        let mut client = self.connection()?;
        let rows=client.query("SELECT instance_id,endpoint,updated_at_unix_ms,expires_at_unix_ms FROM vsn_control_instances WHERE expires_at_unix_ms >= $1 ORDER BY instance_id",&[&now])?;
        Ok(rows
            .into_iter()
            .map(|r| ClusterInstance {
                instance_id: r.get(0),
                endpoint: r.get(1),
                updated_at_unix_ms: u128::try_from(r.get::<_, i64>(2)).unwrap_or(0),
                expires_at_unix_ms: u128::try_from(r.get::<_, i64>(3)).unwrap_or(0),
            })
            .collect())
    }
    pub fn upsert_route(
        &self,
        route_type: &str,
        route_key: &str,
        instance_id: &str,
        ttl_ms: u64,
    ) -> Result<(), StoreError> {
        validate_name(route_type)?;
        validate_route_key(route_key)?;
        validate_name(instance_id)?;
        let now = now_i64();
        let expires =
            now.saturating_add(i64::try_from(ttl_ms.clamp(5_000, 300_000)).unwrap_or(300_000));
        let mut client = self.connection()?;
        client.execute("INSERT INTO vsn_control_routes(route_type,route_key,instance_id,expires_at_unix_ms) VALUES($1,$2,$3,$4) ON CONFLICT(route_type,route_key) DO UPDATE SET instance_id=EXCLUDED.instance_id,expires_at_unix_ms=EXCLUDED.expires_at_unix_ms",&[&route_type,&route_key,&instance_id,&expires])?;
        client.execute(
            "DELETE FROM vsn_control_routes WHERE expires_at_unix_ms < $1",
            &[&now],
        )?;
        Ok(())
    }
    pub fn route_owner(
        &self,
        route_type: &str,
        route_key: &str,
    ) -> Result<Option<String>, StoreError> {
        validate_name(route_type)?;
        validate_route_key(route_key)?;
        let now = now_i64();
        let mut client = self.connection()?;
        Ok(client.query_opt("SELECT instance_id FROM vsn_control_routes WHERE route_type=$1 AND route_key=$2 AND expires_at_unix_ms >= $3",&[&route_type,&route_key,&now])?.map(|r|r.get(0)))
    }
    pub fn remove_route_if_owner(
        &self,
        route_type: &str,
        route_key: &str,
        instance_id: &str,
    ) -> Result<(), StoreError> {
        validate_name(route_type)?;
        validate_route_key(route_key)?;
        validate_name(instance_id)?;
        let mut client = self.connection()?;
        client.execute("DELETE FROM vsn_control_routes WHERE route_type=$1 AND route_key=$2 AND instance_id=$3",&[&route_type,&route_key,&instance_id])?;
        Ok(())
    }

    pub fn publish_bus(
        &self,
        source_instance_id: &str,
        target_instance_id: &str,
        topic: &str,
        payload: &str,
        ttl_ms: u64,
    ) -> Result<i64, StoreError> {
        validate_name(source_instance_id)?;
        validate_name(target_instance_id)?;
        validate_name(topic)?;
        if payload.len() > 2 * 1024 * 1024 {
            return Err(StoreError::Invalid(
                "cluster bus payload exceeds 2 MiB".into(),
            ));
        }
        let now = now_i64();
        let expires =
            now.saturating_add(i64::try_from(ttl_ms.clamp(1_000, 120_000)).unwrap_or(120_000));
        let mut client = self.connection()?;
        let row=client.query_one("INSERT INTO vsn_control_bus(source_instance_id,target_instance_id,topic,payload,created_at_unix_ms,expires_at_unix_ms) VALUES($1,$2,$3,$4,$5,$6) RETURNING id",&[&source_instance_id,&target_instance_id,&topic,&payload,&now,&expires])?;
        client.execute(
            "DELETE FROM vsn_control_bus WHERE expires_at_unix_ms < $1",
            &[&now],
        )?;
        Ok(row.get(0))
    }
    pub fn poll_bus(
        &self,
        target_instance_id: &str,
        after_id: i64,
        limit: u32,
    ) -> Result<Vec<ClusterBusMessage>, StoreError> {
        validate_name(target_instance_id)?;
        let now = now_i64();
        let limit = i64::from(limit.clamp(1, 256));
        let mut client = self.connection()?;
        let rows=client.query("SELECT id,source_instance_id,target_instance_id,topic,payload,created_at_unix_ms,expires_at_unix_ms FROM vsn_control_bus WHERE target_instance_id=$1 AND id>$2 AND expires_at_unix_ms >= $3 ORDER BY id LIMIT $4",&[&target_instance_id,&after_id,&now,&limit])?;
        Ok(rows
            .into_iter()
            .map(|r| ClusterBusMessage {
                id: r.get(0),
                source_instance_id: r.get(1),
                target_instance_id: r.get(2),
                topic: r.get(3),
                payload: r.get(4),
                created_at_unix_ms: u128::try_from(r.get::<_, i64>(5)).unwrap_or(0),
                expires_at_unix_ms: u128::try_from(r.get::<_, i64>(6)).unwrap_or(0),
            })
            .collect())
    }
    pub fn ack_bus(&self, target_instance_id: &str, id: i64) -> Result<bool, StoreError> {
        validate_name(target_instance_id)?;
        let mut client = self.connection()?;
        Ok(client.execute(
            "DELETE FROM vsn_control_bus WHERE target_instance_id=$1 AND id=$2",
            &[&target_instance_id, &id],
        )? == 1)
    }
    pub fn bus_depth(&self, target_instance_id: &str) -> Result<u64, StoreError> {
        validate_name(target_instance_id)?;
        let now = now_i64();
        let mut client = self.connection()?;
        let row=client.query_one("SELECT COUNT(*) FROM vsn_control_bus WHERE target_instance_id=$1 AND expires_at_unix_ms >= $2",&[&target_instance_id,&now])?;
        let count: i64 = row.get(0);
        Ok(u64::try_from(count).unwrap_or(0))
    }

    pub fn create_pairing(&self, nonce: &str, expires_at_unix_ms: u128) -> Result<(), StoreError> {
        validate_route_key(nonce)?;
        let expires = i64::try_from(expires_at_unix_ms)
            .map_err(|_| StoreError::Invalid("pairing expiry exceeds integer range".into()))?;
        let now = now_i64();
        if expires <= now {
            return Err(StoreError::Invalid(
                "pairing expiry must be in the future".into(),
            ));
        }
        let mut client = self.connection()?;
        client.execute("INSERT INTO vsn_control_pairings(pairing_nonce,expires_at_unix_ms,consumed_at_unix_ms) VALUES($1,$2,NULL) ON CONFLICT(pairing_nonce) DO UPDATE SET expires_at_unix_ms=EXCLUDED.expires_at_unix_ms,consumed_at_unix_ms=NULL",&[&nonce,&expires])?;
        client.execute(
            "DELETE FROM vsn_control_pairings WHERE expires_at_unix_ms < $1",
            &[&now],
        )?;
        Ok(())
    }
    pub fn consume_pairing(&self, nonce: &str) -> Result<bool, StoreError> {
        validate_route_key(nonce)?;
        let now = now_i64();
        let mut client = self.connection()?;
        let changed=client.execute("UPDATE vsn_control_pairings SET consumed_at_unix_ms=$2 WHERE pairing_nonce=$1 AND consumed_at_unix_ms IS NULL AND expires_at_unix_ms >= $2",&[&nonce,&now])?;
        Ok(changed == 1)
    }
    pub fn upsert_device(&self, device: &SharedDeviceRecord) -> Result<(), StoreError> {
        validate_route_key(&device.device_id)?;
        if device.public_key.len() > 4096
            || device.display_name.len() > 256
            || device.os.len() > 128
        {
            return Err(StoreError::Invalid(
                "shared device fields exceed limits".into(),
            ));
        }
        let enrolled = i64::try_from(device.enrolled_at_unix_ms)
            .map_err(|_| StoreError::Invalid("invalid enrolled timestamp".into()))?;
        let seen = i64::try_from(device.last_seen_unix_ms)
            .map_err(|_| StoreError::Invalid("invalid last-seen timestamp".into()))?;
        let mut client = self.connection()?;
        client.execute("INSERT INTO vsn_control_devices(device_id,public_key,display_name,os,enrolled_at_unix_ms,last_seen_unix_ms) VALUES($1,$2,$3,$4,$5,$6) ON CONFLICT(device_id) DO UPDATE SET public_key=EXCLUDED.public_key,display_name=EXCLUDED.display_name,os=EXCLUDED.os,last_seen_unix_ms=GREATEST(vsn_control_devices.last_seen_unix_ms,EXCLUDED.last_seen_unix_ms)",&[&device.device_id,&device.public_key,&device.display_name,&device.os,&enrolled,&seen])?;
        Ok(())
    }
    pub fn shared_device(&self, device_id: &str) -> Result<Option<SharedDeviceRecord>, StoreError> {
        validate_route_key(device_id)?;
        let mut client = self.connection()?;
        let row=client.query_opt("SELECT device_id,public_key,display_name,os,enrolled_at_unix_ms,last_seen_unix_ms FROM vsn_control_devices WHERE device_id=$1",&[&device_id])?;
        Ok(row.map(|r| SharedDeviceRecord {
            device_id: r.get(0),
            public_key: r.get(1),
            display_name: r.get(2),
            os: r.get(3),
            enrolled_at_unix_ms: u128::try_from(r.get::<_, i64>(4)).unwrap_or(0),
            last_seen_unix_ms: u128::try_from(r.get::<_, i64>(5)).unwrap_or(0),
        }))
    }
    pub fn shared_devices(&self, limit: u32) -> Result<Vec<SharedDeviceRecord>, StoreError> {
        let mut client = self.connection()?;
        let limit = i64::from(limit.clamp(1, 5000));
        let rows=client.query("SELECT device_id,public_key,display_name,os,enrolled_at_unix_ms,last_seen_unix_ms FROM vsn_control_devices ORDER BY last_seen_unix_ms DESC LIMIT $1",&[&limit])?;
        Ok(rows
            .into_iter()
            .map(|r| SharedDeviceRecord {
                device_id: r.get(0),
                public_key: r.get(1),
                display_name: r.get(2),
                os: r.get(3),
                enrolled_at_unix_ms: u128::try_from(r.get::<_, i64>(4)).unwrap_or(0),
                last_seen_unix_ms: u128::try_from(r.get::<_, i64>(5)).unwrap_or(0),
            })
            .collect())
    }
    pub fn touch_device(
        &self,
        device_id: &str,
        last_seen_unix_ms: u128,
    ) -> Result<bool, StoreError> {
        validate_route_key(device_id)?;
        let seen = i64::try_from(last_seen_unix_ms)
            .map_err(|_| StoreError::Invalid("invalid last-seen timestamp".into()))?;
        let mut client = self.connection()?;
        Ok(client.execute("UPDATE vsn_control_devices SET last_seen_unix_ms=GREATEST(last_seen_unix_ms,$2) WHERE device_id=$1",&[&device_id,&seen])?==1)
    }
    pub fn consume_rate_limit(
        &self,
        key: &str,
        limit: u32,
        window_ms: u64,
        now_unix_ms: u128,
    ) -> Result<bool, StoreError> {
        if key.is_empty()
            || key.len() > 256
            || key.chars().any(char::is_control)
            || limit == 0
            || window_ms < 1000
        {
            return Err(StoreError::Invalid(
                "invalid shared rate-limit request".into(),
            ));
        }
        let now = i64::try_from(now_unix_ms)
            .map_err(|_| StoreError::Invalid("invalid rate-limit timestamp".into()))?;
        let window = i64::try_from(window_ms)
            .map_err(|_| StoreError::Invalid("invalid rate-limit window".into()))?;
        let start = (now / window) * window;
        let expiry = start.saturating_add(window.saturating_mul(2));
        let mut client = self.connection()?;
        let mut tx = client.transaction()?;
        let row=tx.query_one("INSERT INTO vsn_control_rate_limits(rate_key,window_start_unix_ms,count,expires_at_unix_ms) VALUES($1,$2,1,$3) ON CONFLICT(rate_key,window_start_unix_ms) DO UPDATE SET count=vsn_control_rate_limits.count+1,expires_at_unix_ms=GREATEST(vsn_control_rate_limits.expires_at_unix_ms,EXCLUDED.expires_at_unix_ms) RETURNING count",&[&key,&start,&expiry])?;
        let count: i64 = row.get(0);
        tx.execute(
            "DELETE FROM vsn_control_rate_limits WHERE expires_at_unix_ms < $1",
            &[&now],
        )?;
        tx.commit()?;
        Ok(count <= i64::from(limit))
    }

    pub fn create_approval(
        &self,
        approval_id: &str,
        payload: &str,
        created_at_unix_ms: u128,
        expires_at_unix_ms: u128,
    ) -> Result<(), StoreError> {
        validate_route_key(approval_id)?;
        if payload.len() > 512 * 1024 || expires_at_unix_ms <= created_at_unix_ms {
            return Err(StoreError::Invalid("invalid shared approval".into()));
        }
        let created = i64::try_from(created_at_unix_ms)
            .map_err(|_| StoreError::Invalid("invalid approval created timestamp".into()))?;
        let expires = i64::try_from(expires_at_unix_ms)
            .map_err(|_| StoreError::Invalid("invalid approval expiry timestamp".into()))?;
        let mut client = self.connection()?;
        client.execute("INSERT INTO vsn_control_approvals(approval_id,payload,state,created_at_unix_ms,expires_at_unix_ms) VALUES($1,$2,'pending',$3,$4)",&[&approval_id,&payload,&created,&expires])?;
        Ok(())
    }
    pub fn approval(&self, approval_id: &str) -> Result<Option<SharedApprovalRecord>, StoreError> {
        validate_route_key(approval_id)?;
        let mut client = self.connection()?;
        let now = now_i64();
        client.execute("UPDATE vsn_control_approvals SET state='expired' WHERE approval_id=$1 AND state='pending' AND expires_at_unix_ms < $2",&[&approval_id,&now])?;
        let row=client.query_opt("SELECT approval_id,payload,state,created_at_unix_ms,expires_at_unix_ms,approver_id,decided_at_unix_ms,command_id FROM vsn_control_approvals WHERE approval_id=$1",&[&approval_id])?;
        Ok(row.map(Self::shared_approval_from_row))
    }
    pub fn recent_approvals(&self, limit: u32) -> Result<Vec<SharedApprovalRecord>, StoreError> {
        let mut client = self.connection()?;
        let now = now_i64();
        client.execute("UPDATE vsn_control_approvals SET state='expired' WHERE state='pending' AND expires_at_unix_ms < $1",&[&now])?;
        let limit = i64::from(limit.clamp(1, 1000));
        let rows=client.query("SELECT approval_id,payload,state,created_at_unix_ms,expires_at_unix_ms,approver_id,decided_at_unix_ms,command_id FROM vsn_control_approvals ORDER BY created_at_unix_ms DESC LIMIT $1",&[&limit])?;
        Ok(rows
            .into_iter()
            .map(Self::shared_approval_from_row)
            .collect())
    }
    pub fn approve_and_enqueue(
        &self,
        approval_id: &str,
        approver_id: &str,
        command_id: &str,
        device_id: &str,
        command_payload: &str,
        command_expires_at_unix_ms: u128,
    ) -> Result<bool, StoreError> {
        validate_route_key(approval_id)?;
        validate_route_key(approver_id)?;
        validate_route_key(command_id)?;
        validate_route_key(device_id)?;
        if command_payload.len() > 2 * 1024 * 1024 {
            return Err(StoreError::Invalid(
                "shared command payload exceeds 2 MiB".into(),
            ));
        }
        let now = now_i64();
        let expires = i64::try_from(command_expires_at_unix_ms)
            .map_err(|_| StoreError::Invalid("invalid command expiry".into()))?;
        let mut client = self.connection()?;
        let mut tx = client.transaction()?;
        let row=tx.query_opt("SELECT state,expires_at_unix_ms FROM vsn_control_approvals WHERE approval_id=$1 FOR UPDATE",&[&approval_id])?;
        let Some(row) = row else {
            tx.rollback()?;
            return Ok(false);
        };
        let state: String = row.get(0);
        let approval_expires: i64 = row.get(1);
        if state != "pending" || approval_expires < now {
            if state == "pending" {
                tx.execute(
                    "UPDATE vsn_control_approvals SET state='expired' WHERE approval_id=$1",
                    &[&approval_id],
                )?;
            }
            tx.commit()?;
            return Ok(false);
        };
        tx.execute("INSERT INTO vsn_control_commands(command_id,device_id,payload,state,attempts,expires_at_unix_ms,created_at_unix_ms) VALUES($1,$2,$3,'queued',0,$4,$5)",&[&command_id,&device_id,&command_payload,&expires,&now])?;
        tx.execute("UPDATE vsn_control_approvals SET state='approved',approver_id=$2,decided_at_unix_ms=$3,command_id=$4 WHERE approval_id=$1",&[&approval_id,&approver_id,&now,&command_id])?;
        tx.commit()?;
        Ok(true)
    }
    pub fn reject_approval(
        &self,
        approval_id: &str,
        approver_id: &str,
    ) -> Result<bool, StoreError> {
        validate_route_key(approval_id)?;
        validate_route_key(approver_id)?;
        let now = now_i64();
        let mut client = self.connection()?;
        let changed=client.execute("UPDATE vsn_control_approvals SET state=CASE WHEN expires_at_unix_ms < $3 THEN 'expired' ELSE 'rejected' END,approver_id=CASE WHEN expires_at_unix_ms < $3 THEN approver_id ELSE $2 END,decided_at_unix_ms=CASE WHEN expires_at_unix_ms < $3 THEN decided_at_unix_ms ELSE $3 END WHERE approval_id=$1 AND state='pending'",&[&approval_id,&approver_id,&now])?;
        Ok(changed == 1)
    }

    pub fn append_audit_batch(
        &self,
        device_id: &str,
        events: &[SharedAuditInput],
    ) -> Result<SharedAuditAppendResult, StoreError> {
        validate_route_key(device_id)?;
        if events.is_empty() || events.len() > 256 {
            return Err(StoreError::Invalid(
                "audit batch must contain 1..256 events".into(),
            ));
        }
        let mut client = self.connection()?;
        let mut tx = client.transaction()?;
        let lock_key = format!("audit:{device_id}");
        tx.query_one(
            "SELECT pg_advisory_xact_lock(hashtext($1)::bigint)",
            &[&lock_key],
        )?;
        let mut last_hash=tx.query_opt("SELECT event_hash FROM vsn_control_audit WHERE device_id=$1 ORDER BY seq DESC LIMIT 1",&[&device_id])?.map(|r|r.get::<_,String>(0)).unwrap_or_else(||"GENESIS".into());
        let mut accepted = 0u32;
        let mut duplicates = 0u32;
        for event in events {
            validate_route_key(&event.event_id)?;
            if event.event_hash.len() != 64
                || event.previous_hash.len() > 128
                || event.payload.len() > 512 * 1024
            {
                return Err(StoreError::Invalid(
                    "audit event exceeds shared-store limits".into(),
                ));
            }
            if let Some(row) = tx.query_opt(
                "SELECT event_hash FROM vsn_control_audit WHERE device_id=$1 AND event_id=$2",
                &[&device_id, &event.event_id],
            )? {
                let existing: String = row.get(0);
                if existing != event.event_hash {
                    return Err(StoreError::Invalid(
                        "audit event id already exists with a different hash".into(),
                    ));
                }
                duplicates = duplicates.saturating_add(1);
                continue;
            }
            if event.previous_hash != last_hash {
                return Err(StoreError::Invalid(format!(
                    "audit chain continuity mismatch: expected {last_hash}, got {}",
                    event.previous_hash
                )));
            }
            let ts = i64::try_from(event.timestamp_unix_ms)
                .map_err(|_| StoreError::Invalid("invalid audit timestamp".into()))?;
            tx.execute("INSERT INTO vsn_control_audit(device_id,event_id,previous_hash,event_hash,timestamp_unix_ms,payload) VALUES($1,$2,$3,$4,$5,$6)",&[&device_id,&event.event_id,&event.previous_hash,&event.event_hash,&ts,&event.payload])?;
            last_hash = event.event_hash.clone();
            accepted = accepted.saturating_add(1);
        }
        tx.commit()?;
        Ok(SharedAuditAppendResult {
            accepted,
            duplicates,
            last_hash,
        })
    }
    pub fn recent_audit(&self, limit: u32) -> Result<Vec<SharedAuditRecord>, StoreError> {
        let mut client = self.connection()?;
        let limit = i64::from(limit.clamp(1, 5000));
        let rows=client.query("SELECT seq,device_id,event_id,previous_hash,event_hash,timestamp_unix_ms,payload FROM vsn_control_audit ORDER BY seq DESC LIMIT $1",&[&limit])?;
        Ok(rows
            .into_iter()
            .map(|r| SharedAuditRecord {
                seq: r.get(0),
                device_id: r.get(1),
                event_id: r.get(2),
                previous_hash: r.get(3),
                event_hash: r.get(4),
                timestamp_unix_ms: u128::try_from(r.get::<_, i64>(5)).unwrap_or(0),
                payload: r.get(6),
            })
            .collect())
    }

    pub fn enqueue_command(
        &self,
        command_id: &str,
        device_id: &str,
        payload: &str,
        expires_at_unix_ms: u128,
    ) -> Result<(), StoreError> {
        validate_route_key(command_id)?;
        validate_route_key(device_id)?;
        if payload.len() > 2 * 1024 * 1024 {
            return Err(StoreError::Invalid(
                "shared command payload exceeds 2 MiB".into(),
            ));
        }
        let expires = i64::try_from(expires_at_unix_ms).map_err(|_| {
            StoreError::Invalid("command expiry exceeds PostgreSQL integer range".into())
        })?;
        let now = now_i64();
        if expires <= now {
            return Err(StoreError::Invalid(
                "cannot enqueue an already expired command".into(),
            ));
        }
        let mut client = self.connection()?;
        client.execute("INSERT INTO vsn_control_commands(command_id,device_id,payload,state,attempts,expires_at_unix_ms,created_at_unix_ms) VALUES($1,$2,$3,'queued',0,$4,$5) ON CONFLICT(command_id) DO NOTHING",&[&command_id,&device_id,&payload,&expires,&now])?;
        Ok(())
    }
    pub fn lease_command(
        &self,
        device_id: &str,
        instance_id: &str,
        lease_ms: u64,
        max_attempts: u32,
    ) -> Result<Option<SharedCommandRecord>, StoreError> {
        validate_route_key(device_id)?;
        validate_name(instance_id)?;
        let now = now_i64();
        let lease_until =
            now.saturating_add(i64::try_from(lease_ms.clamp(1_000, 300_000)).unwrap_or(300_000));
        let max_attempts = i64::from(max_attempts.clamp(1, 100));
        let mut client = self.connection()?;
        let mut tx = client.transaction()?;
        tx.execute("UPDATE vsn_control_commands SET state='failed',last_error='command expired before completion',lease_until_unix_ms=NULL,leased_by=NULL WHERE device_id=$1 AND state IN ('queued','inflight') AND expires_at_unix_ms < $2",&[&device_id,&now])?;
        tx.execute("UPDATE vsn_control_commands SET state='failed',last_error='delivery attempt limit reached',lease_until_unix_ms=NULL,leased_by=NULL WHERE device_id=$1 AND state IN ('queued','inflight') AND attempts >= $2",&[&device_id,&max_attempts])?;
        let row=tx.query_opt("SELECT command_id,payload,attempts,expires_at_unix_ms,created_at_unix_ms FROM vsn_control_commands WHERE device_id=$1 AND expires_at_unix_ms >= $2 AND attempts < $3 AND (state='queued' OR (state='inflight' AND COALESCE(lease_until_unix_ms,0) <= $2)) ORDER BY created_at_unix_ms,command_id FOR UPDATE SKIP LOCKED LIMIT 1",&[&device_id,&now,&max_attempts])?;
        let Some(row) = row else {
            tx.commit()?;
            return Ok(None);
        };
        let command_id: String = row.get(0);
        let payload: String = row.get(1);
        let attempts: i64 = row.get(2);
        let expires: i64 = row.get(3);
        let created: i64 = row.get(4);
        let next_attempts = attempts.saturating_add(1);
        tx.execute("UPDATE vsn_control_commands SET state='inflight',attempts=$2,leased_by=$3,lease_until_unix_ms=$4,last_error=NULL WHERE command_id=$1",&[&command_id,&next_attempts,&instance_id,&lease_until])?;
        tx.commit()?;
        Ok(Some(SharedCommandRecord {
            command_id,
            device_id: device_id.into(),
            payload,
            state: "inflight".into(),
            attempts: u32::try_from(next_attempts).unwrap_or(u32::MAX),
            leased_by: Some(instance_id.into()),
            lease_until_unix_ms: u128::try_from(lease_until).ok(),
            expires_at_unix_ms: u128::try_from(expires).unwrap_or(0),
            created_at_unix_ms: u128::try_from(created).unwrap_or(0),
            completed_at_unix_ms: None,
            last_error: None,
            result_payload: None,
        }))
    }
    pub fn command(&self, command_id: &str) -> Result<Option<SharedCommandRecord>, StoreError> {
        validate_route_key(command_id)?;
        let mut client = self.connection()?;
        let row=client.query_opt("SELECT command_id,device_id,payload,state,attempts,leased_by,lease_until_unix_ms,expires_at_unix_ms,created_at_unix_ms,completed_at_unix_ms,last_error,result_payload FROM vsn_control_commands WHERE command_id=$1",&[&command_id])?;
        Ok(row.map(Self::shared_command_from_row))
    }
    pub fn complete_command(
        &self,
        command_id: &str,
        device_id: &str,
        result_payload: &str,
    ) -> Result<bool, StoreError> {
        validate_route_key(command_id)?;
        validate_route_key(device_id)?;
        if result_payload.len() > 2 * 1024 * 1024 {
            return Err(StoreError::Invalid(
                "shared command result exceeds 2 MiB".into(),
            ));
        }
        let now = now_i64();
        let mut client = self.connection()?;
        let changed=client.execute("UPDATE vsn_control_commands SET state='completed',completed_at_unix_ms=$3,lease_until_unix_ms=NULL,leased_by=NULL,last_error=NULL,result_payload=$4 WHERE command_id=$1 AND device_id=$2 AND state IN ('queued','inflight','completed')",&[&command_id,&device_id,&now,&result_payload])?;
        Ok(changed == 1)
    }
    pub fn recent_commands(&self, limit: u32) -> Result<Vec<SharedCommandRecord>, StoreError> {
        let mut client = self.connection()?;
        let limit = i64::from(limit.clamp(1, 1000));
        let rows=client.query("SELECT command_id,device_id,payload,state,attempts,leased_by,lease_until_unix_ms,expires_at_unix_ms,created_at_unix_ms,completed_at_unix_ms,last_error,result_payload FROM vsn_control_commands ORDER BY created_at_unix_ms DESC LIMIT $1",&[&limit])?;
        Ok(rows
            .into_iter()
            .map(Self::shared_command_from_row)
            .collect())
    }
    pub fn cleanup_commands(&self, retention_ms: u64) -> Result<u64, StoreError> {
        let now = now_i64();
        let cutoff = now.saturating_sub(
            i64::try_from(retention_ms.clamp(60_000, 30 * 24 * 60 * 60 * 1000))
                .unwrap_or(30 * 24 * 60 * 60 * 1000),
        );
        let mut client = self.connection()?;
        let changed=client.execute("DELETE FROM vsn_control_commands WHERE state IN ('completed','failed') AND COALESCE(completed_at_unix_ms,created_at_unix_ms) < $1",&[&cutoff])?;
        Ok(changed)
    }

    pub fn upsert_stream_checkpoint(
        &self,
        record: &SharedStreamCheckpoint,
    ) -> Result<(), StoreError> {
        validate_route_key(&record.relay_id)?;
        validate_route_key(&record.device_id)?;
        validate_route_key(&record.principal_id)?;
        validate_name(&record.permission.replace('.', "_"))?;
        validate_name(&record.agent_instance_id)?;
        if record.request_json.len() > 512 * 1024 {
            return Err(StoreError::Invalid(
                "stream checkpoint request exceeds 512 KiB".into(),
            ));
        }
        if record.resume_token_hash.len() != 64
            || !record
                .resume_token_hash
                .bytes()
                .all(|b| b.is_ascii_hexdigit())
        {
            return Err(StoreError::Invalid(
                "stream resume token hash must be SHA-256 hex".into(),
            ));
        }
        let next = i64::try_from(record.next_input_seq).map_err(|_| {
            StoreError::Invalid("stream input sequence exceeds PostgreSQL integer range".into())
        })?;
        let acked = i64::try_from(record.acked_input_seq).map_err(|_| {
            StoreError::Invalid("stream ack sequence exceeds PostgreSQL integer range".into())
        })?;
        let committed = record
            .committed_bytes
            .map(i64::try_from)
            .transpose()
            .map_err(|_| {
                StoreError::Invalid(
                    "stream committed byte count exceeds PostgreSQL integer range".into(),
                )
            })?;
        let progress = i64::try_from(record.resource_progress_bytes).map_err(|_| {
            StoreError::Invalid("stream progress exceeds PostgreSQL integer range".into())
        })?;
        let created = to_i64_ms(record.created_at_unix_ms, "stream created timestamp")?;
        let activity = to_i64_ms(record.last_activity_unix_ms, "stream activity timestamp")?;
        let detached = record
            .detached_until_unix_ms
            .map(|v| to_i64_ms(v, "stream detached timestamp"))
            .transpose()?;
        let expires = to_i64_ms(record.expires_at_unix_ms, "stream expiry timestamp")?;
        let mut client = self.connection()?;
        client.execute("INSERT INTO vsn_control_stream_relays(relay_id,device_id,principal_id,permission,request_json,agent_instance_id,resume_token_hash,resource_id,next_input_seq,acked_input_seq,committed_bytes,resource_progress_bytes,created_at_unix_ms,last_activity_unix_ms,detached_until_unix_ms,expires_at_unix_ms) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16) ON CONFLICT(relay_id) DO UPDATE SET device_id=EXCLUDED.device_id,principal_id=EXCLUDED.principal_id,permission=EXCLUDED.permission,request_json=EXCLUDED.request_json,agent_instance_id=EXCLUDED.agent_instance_id,resume_token_hash=EXCLUDED.resume_token_hash,resource_id=EXCLUDED.resource_id,next_input_seq=EXCLUDED.next_input_seq,acked_input_seq=EXCLUDED.acked_input_seq,committed_bytes=EXCLUDED.committed_bytes,resource_progress_bytes=EXCLUDED.resource_progress_bytes,last_activity_unix_ms=EXCLUDED.last_activity_unix_ms,detached_until_unix_ms=EXCLUDED.detached_until_unix_ms,expires_at_unix_ms=EXCLUDED.expires_at_unix_ms",&[&record.relay_id,&record.device_id,&record.principal_id,&record.permission,&record.request_json,&record.agent_instance_id,&record.resume_token_hash,&record.resource_id,&next,&acked,&committed,&progress,&created,&activity,&detached,&expires])?;
        Ok(())
    }
    pub fn stream_checkpoint(
        &self,
        relay_id: &str,
    ) -> Result<Option<SharedStreamCheckpoint>, StoreError> {
        validate_route_key(relay_id)?;
        let now = now_i64();
        let mut client = self.connection()?;
        let row=client.query_opt("SELECT relay_id,device_id,principal_id,permission,request_json,agent_instance_id,resume_token_hash,resource_id,next_input_seq,acked_input_seq,committed_bytes,resource_progress_bytes,created_at_unix_ms,last_activity_unix_ms,detached_until_unix_ms,expires_at_unix_ms FROM vsn_control_stream_relays WHERE relay_id=$1 AND expires_at_unix_ms >= $2",&[&relay_id,&now])?;
        Ok(row.map(Self::shared_stream_checkpoint_from_row))
    }
    pub fn stream_checkpoints_for_device(
        &self,
        device_id: &str,
        limit: u32,
    ) -> Result<Vec<SharedStreamCheckpoint>, StoreError> {
        validate_route_key(device_id)?;
        let now = now_i64();
        let limit = i64::from(limit.clamp(1, 4096));
        let mut client = self.connection()?;
        let rows=client.query("SELECT relay_id,device_id,principal_id,permission,request_json,agent_instance_id,resume_token_hash,resource_id,next_input_seq,acked_input_seq,committed_bytes,resource_progress_bytes,created_at_unix_ms,last_activity_unix_ms,detached_until_unix_ms,expires_at_unix_ms FROM vsn_control_stream_relays WHERE device_id=$1 AND expires_at_unix_ms >= $2 ORDER BY last_activity_unix_ms DESC LIMIT $3",&[&device_id,&now,&limit])?;
        Ok(rows
            .into_iter()
            .map(Self::shared_stream_checkpoint_from_row)
            .collect())
    }
    pub fn append_stream_frame(
        &self,
        relay_id: &str,
        seq: u64,
        frame_json: &str,
        created_at_unix_ms: u128,
        max_frames: u32,
    ) -> Result<(), StoreError> {
        validate_route_key(relay_id)?;
        if frame_json.len() > 512 * 1024 {
            return Err(StoreError::Invalid(
                "stream replay frame exceeds 512 KiB".into(),
            ));
        }
        let seq = i64::try_from(seq).map_err(|_| {
            StoreError::Invalid("stream frame sequence exceeds PostgreSQL integer range".into())
        })?;
        let created = to_i64_ms(created_at_unix_ms, "stream frame timestamp")?;
        let keep = i64::from(max_frames.clamp(1, 1024));
        let mut client = self.connection()?;
        let mut tx = client.transaction()?;
        tx.execute("INSERT INTO vsn_control_stream_frames(relay_id,seq,frame_json,created_at_unix_ms) VALUES($1,$2,$3,$4) ON CONFLICT(relay_id,seq) DO UPDATE SET frame_json=EXCLUDED.frame_json,created_at_unix_ms=EXCLUDED.created_at_unix_ms",&[&relay_id,&seq,&frame_json,&created])?;
        tx.execute("DELETE FROM vsn_control_stream_frames WHERE relay_id=$1 AND seq < (SELECT COALESCE(MAX(seq),0)-$2 FROM vsn_control_stream_frames WHERE relay_id=$1)",&[&relay_id,&keep])?;
        tx.commit()?;
        Ok(())
    }
    pub fn stream_frames_after(
        &self,
        relay_id: &str,
        after_seq: Option<u64>,
        limit: u32,
    ) -> Result<Vec<SharedStreamFrame>, StoreError> {
        validate_route_key(relay_id)?;
        let after = after_seq
            .map(i64::try_from)
            .transpose()
            .map_err(|_| {
                StoreError::Invalid(
                    "stream replay sequence exceeds PostgreSQL integer range".into(),
                )
            })?
            .unwrap_or(-1);
        let limit = i64::from(limit.clamp(1, 1024));
        let mut client = self.connection()?;
        let rows=client.query("SELECT relay_id,seq,frame_json,created_at_unix_ms FROM vsn_control_stream_frames WHERE relay_id=$1 AND seq>$2 ORDER BY seq ASC LIMIT $3",&[&relay_id,&after,&limit])?;
        Ok(rows
            .into_iter()
            .map(|r| SharedStreamFrame {
                relay_id: r.get(0),
                seq: u64::try_from(r.get::<_, i64>(1)).unwrap_or(0),
                frame_json: r.get(2),
                created_at_unix_ms: u128::try_from(r.get::<_, i64>(3)).unwrap_or(0),
            })
            .collect())
    }
    pub fn delete_stream_checkpoint(&self, relay_id: &str) -> Result<(), StoreError> {
        validate_route_key(relay_id)?;
        let mut client = self.connection()?;
        client.execute(
            "DELETE FROM vsn_control_stream_relays WHERE relay_id=$1",
            &[&relay_id],
        )?;
        Ok(())
    }
    pub fn cleanup_stream_checkpoints(&self) -> Result<u64, StoreError> {
        let now = now_i64();
        let mut client = self.connection()?;
        let changed = client.execute(
            "DELETE FROM vsn_control_stream_relays WHERE expires_at_unix_ms < $1",
            &[&now],
        )?;
        Ok(changed)
    }
    fn shared_stream_checkpoint_from_row(r: postgres::Row) -> SharedStreamCheckpoint {
        SharedStreamCheckpoint {
            relay_id: r.get(0),
            device_id: r.get(1),
            principal_id: r.get(2),
            permission: r.get(3),
            request_json: r.get(4),
            agent_instance_id: r.get(5),
            resume_token_hash: r.get(6),
            resource_id: r.get(7),
            next_input_seq: u64::try_from(r.get::<_, i64>(8)).unwrap_or(0),
            acked_input_seq: u64::try_from(r.get::<_, i64>(9)).unwrap_or(0),
            committed_bytes: r
                .get::<_, Option<i64>>(10)
                .and_then(|v| u64::try_from(v).ok()),
            resource_progress_bytes: u64::try_from(r.get::<_, i64>(11)).unwrap_or(0),
            created_at_unix_ms: u128::try_from(r.get::<_, i64>(12)).unwrap_or(0),
            last_activity_unix_ms: u128::try_from(r.get::<_, i64>(13)).unwrap_or(0),
            detached_until_unix_ms: r
                .get::<_, Option<i64>>(14)
                .and_then(|v| u128::try_from(v).ok()),
            expires_at_unix_ms: u128::try_from(r.get::<_, i64>(15)).unwrap_or(0),
        }
    }

    pub fn upsert_session(&self, record: &SharedSessionRecord) -> Result<(), StoreError> {
        validate_route_key(&record.session_id)?;
        validate_route_key(&record.account_id)?;
        validate_sha256_hex(&record.token_hash)?;
        if record.payload.len() > 512 * 1024 {
            return Err(StoreError::Invalid(
                "shared session payload exceeds 512 KiB".into(),
            ));
        }
        let created = to_i64_ms(record.created_at_unix_ms, "session created timestamp")?;
        let expires = to_i64_ms(record.expires_at_unix_ms, "session expiry timestamp")?;
        let last = to_i64_ms(record.last_activity_unix_ms, "session activity timestamp")?;
        let mut client = self.connection()?;
        client.execute("INSERT INTO vsn_control_sessions(session_id,account_id,token_hash,payload,created_at_unix_ms,expires_at_unix_ms,last_activity_unix_ms,revoked) VALUES($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT(session_id) DO UPDATE SET account_id=EXCLUDED.account_id,token_hash=EXCLUDED.token_hash,payload=EXCLUDED.payload,expires_at_unix_ms=EXCLUDED.expires_at_unix_ms,last_activity_unix_ms=EXCLUDED.last_activity_unix_ms,revoked=EXCLUDED.revoked",&[&record.session_id,&record.account_id,&record.token_hash,&record.payload,&created,&expires,&last,&record.revoked])?;
        Ok(())
    }
    pub fn session_by_token_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<SharedSessionRecord>, StoreError> {
        validate_sha256_hex(token_hash)?;
        let now = now_i64();
        let mut client = self.connection()?;
        let row=client.query_opt("SELECT session_id,account_id,token_hash,payload,created_at_unix_ms,expires_at_unix_ms,last_activity_unix_ms,revoked FROM vsn_control_sessions WHERE token_hash=$1 AND revoked=FALSE AND expires_at_unix_ms >= $2",&[&token_hash,&now])?;
        Ok(row.map(Self::shared_session_from_row))
    }
    pub fn session_count(&self) -> Result<u64, StoreError> {
        let mut client = self.connection()?;
        let count: i64 = client
            .query_one("SELECT COUNT(*) FROM vsn_control_sessions", &[])?
            .get(0);
        u64::try_from(count)
            .map_err(|_| StoreError::Invalid("negative shared session count".into()))
    }
    pub fn touch_session(
        &self,
        session_id: &str,
        last_activity_unix_ms: u128,
        payload: &str,
    ) -> Result<bool, StoreError> {
        validate_route_key(session_id)?;
        if payload.len() > 512 * 1024 {
            return Err(StoreError::Invalid(
                "shared session payload exceeds 512 KiB".into(),
            ));
        }
        let now = now_i64();
        let last = to_i64_ms(last_activity_unix_ms, "session activity timestamp")?;
        let mut client = self.connection()?;
        let changed=client.execute("UPDATE vsn_control_sessions SET last_activity_unix_ms=$2,payload=$3 WHERE session_id=$1 AND revoked=FALSE AND expires_at_unix_ms >= $4",&[&session_id,&last,&payload,&now])?;
        Ok(changed == 1)
    }
    pub fn revoke_session(&self, session_id: &str) -> Result<bool, StoreError> {
        validate_route_key(session_id)?;
        let mut client = self.connection()?;
        Ok(client.execute(
            "UPDATE vsn_control_sessions SET revoked=TRUE WHERE session_id=$1",
            &[&session_id],
        )? == 1)
    }
    pub fn revoke_account_sessions(&self, account_id: &str) -> Result<u64, StoreError> {
        validate_route_key(account_id)?;
        let mut client = self.connection()?;
        Ok(client.execute(
            "UPDATE vsn_control_sessions SET revoked=TRUE WHERE account_id=$1 AND revoked=FALSE",
            &[&account_id],
        )?)
    }
    pub fn cleanup_sessions(&self, retention_ms: u64) -> Result<u64, StoreError> {
        let now = now_i64();
        let cutoff = now.saturating_sub(
            i64::try_from(retention_ms.clamp(60_000, 30 * 24 * 60 * 60 * 1000))
                .unwrap_or(30 * 24 * 60 * 60 * 1000),
        );
        let mut client = self.connection()?;
        Ok(client.execute("DELETE FROM vsn_control_sessions WHERE expires_at_unix_ms < $1 OR (revoked=TRUE AND last_activity_unix_ms < $2)",&[&now,&cutoff])?)
    }
    fn shared_session_from_row(r: postgres::Row) -> SharedSessionRecord {
        SharedSessionRecord {
            session_id: r.get(0),
            account_id: r.get(1),
            token_hash: r.get(2),
            payload: r.get(3),
            created_at_unix_ms: u128::try_from(r.get::<_, i64>(4)).unwrap_or(0),
            expires_at_unix_ms: u128::try_from(r.get::<_, i64>(5)).unwrap_or(0),
            last_activity_unix_ms: u128::try_from(r.get::<_, i64>(6)).unwrap_or(0),
            revoked: r.get(7),
        }
    }

    pub fn role_count(&self) -> Result<u64, StoreError> {
        let mut client = self.connection()?;
        let count: i64 = client
            .query_one("SELECT COUNT(*) FROM vsn_control_roles", &[])?
            .get(0);
        u64::try_from(count).map_err(|_| StoreError::Invalid("negative shared role count".into()))
    }
    pub fn upsert_role(&self, record: &SharedRoleRecord) -> Result<(), StoreError> {
        validate_route_key(&record.role_id)?;
        if record.payload.len() > 512 * 1024 {
            return Err(StoreError::Invalid(
                "shared role payload exceeds 512 KiB".into(),
            ));
        }
        let updated = to_i64_ms(record.updated_at_unix_ms, "role update timestamp")?;
        let mut client = self.connection()?;
        client.execute("INSERT INTO vsn_control_roles(role_id,payload,updated_at_unix_ms) VALUES($1,$2,$3) ON CONFLICT(role_id) DO UPDATE SET payload=EXCLUDED.payload,updated_at_unix_ms=EXCLUDED.updated_at_unix_ms",&[&record.role_id,&record.payload,&updated])?;
        Ok(())
    }
    pub fn role(&self, role_id: &str) -> Result<Option<SharedRoleRecord>, StoreError> {
        validate_route_key(role_id)?;
        let mut client = self.connection()?;
        let row = client.query_opt(
            "SELECT role_id,payload,updated_at_unix_ms FROM vsn_control_roles WHERE role_id=$1",
            &[&role_id],
        )?;
        Ok(row.map(|r| SharedRoleRecord {
            role_id: r.get(0),
            payload: r.get(1),
            updated_at_unix_ms: u128::try_from(r.get::<_, i64>(2)).unwrap_or(0),
        }))
    }
    pub fn list_roles_shared(&self) -> Result<Vec<SharedRoleRecord>, StoreError> {
        let mut client = self.connection()?;
        let rows = client.query(
            "SELECT role_id,payload,updated_at_unix_ms FROM vsn_control_roles ORDER BY role_id",
            &[],
        )?;
        Ok(rows
            .into_iter()
            .map(|r| SharedRoleRecord {
                role_id: r.get(0),
                payload: r.get(1),
                updated_at_unix_ms: u128::try_from(r.get::<_, i64>(2)).unwrap_or(0),
            })
            .collect())
    }

    pub fn account_count(&self) -> Result<u64, StoreError> {
        let mut client = self.connection()?;
        let count: i64 = client
            .query_one("SELECT COUNT(*) FROM vsn_control_accounts", &[])?
            .get(0);
        u64::try_from(count)
            .map_err(|_| StoreError::Invalid("negative shared account count".into()))
    }
    pub fn upsert_account(&self, record: &SharedAccountRecord) -> Result<(), StoreError> {
        validate_route_key(&record.account_id)?;
        validate_route_key(&record.role_id)?;
        if record.email.is_empty()
            || record.email.len() > 320
            || record.email.chars().any(char::is_control)
        {
            return Err(StoreError::Invalid(
                "shared account email is invalid".into(),
            ));
        }
        if record.payload.len() > 2 * 1024 * 1024 {
            return Err(StoreError::Invalid(
                "shared account payload exceeds 2 MiB".into(),
            ));
        }
        let updated = to_i64_ms(record.updated_at_unix_ms, "account update timestamp")?;
        let mut client = self.connection()?;
        client.execute("INSERT INTO vsn_control_accounts(account_id,email,role_id,payload,disabled,updated_at_unix_ms) VALUES($1,$2,$3,$4,$5,$6) ON CONFLICT(account_id) DO UPDATE SET email=EXCLUDED.email,role_id=EXCLUDED.role_id,payload=EXCLUDED.payload,disabled=EXCLUDED.disabled,updated_at_unix_ms=EXCLUDED.updated_at_unix_ms",&[&record.account_id,&record.email,&record.role_id,&record.payload,&record.disabled,&updated])?;
        Ok(())
    }
    pub fn account(&self, account_id: &str) -> Result<Option<SharedAccountRecord>, StoreError> {
        validate_route_key(account_id)?;
        let mut client = self.connection()?;
        let row=client.query_opt("SELECT account_id,email,role_id,payload,disabled,updated_at_unix_ms FROM vsn_control_accounts WHERE account_id=$1",&[&account_id])?;
        Ok(row.map(Self::shared_account_from_row))
    }
    pub fn account_by_email(&self, email: &str) -> Result<Option<SharedAccountRecord>, StoreError> {
        if email.is_empty() || email.len() > 320 || email.chars().any(char::is_control) {
            return Err(StoreError::Invalid(
                "shared account email is invalid".into(),
            ));
        }
        let mut client = self.connection()?;
        let row=client.query_opt("SELECT account_id,email,role_id,payload,disabled,updated_at_unix_ms FROM vsn_control_accounts WHERE email=$1",&[&email])?;
        Ok(row.map(Self::shared_account_from_row))
    }
    pub fn list_accounts_shared(&self) -> Result<Vec<SharedAccountRecord>, StoreError> {
        let mut client = self.connection()?;
        let rows=client.query("SELECT account_id,email,role_id,payload,disabled,updated_at_unix_ms FROM vsn_control_accounts ORDER BY email",&[])?;
        Ok(rows
            .into_iter()
            .map(Self::shared_account_from_row)
            .collect())
    }
    pub fn delete_account(&self, account_id: &str) -> Result<bool, StoreError> {
        validate_route_key(account_id)?;
        let mut client = self.connection()?;
        Ok(client.execute(
            "DELETE FROM vsn_control_accounts WHERE account_id=$1",
            &[&account_id],
        )? == 1)
    }
    fn shared_account_from_row(r: postgres::Row) -> SharedAccountRecord {
        SharedAccountRecord {
            account_id: r.get(0),
            email: r.get(1),
            role_id: r.get(2),
            payload: r.get(3),
            disabled: r.get(4),
            updated_at_unix_ms: u128::try_from(r.get::<_, i64>(5)).unwrap_or(0),
        }
    }

    pub fn put_auth_transaction(&self, record: &SharedAuthTransaction) -> Result<(), StoreError> {
        validate_route_key(&record.transaction_id)?;
        validate_route_key(&record.kind)?;
        if record.payload.len() > 2 * 1024 * 1024 {
            return Err(StoreError::Invalid(
                "auth transaction payload exceeds 2 MiB".into(),
            ));
        }
        let created = to_i64_ms(
            record.created_at_unix_ms,
            "auth transaction created timestamp",
        )?;
        let expires = to_i64_ms(
            record.expires_at_unix_ms,
            "auth transaction expiry timestamp",
        )?;
        let consumed = record
            .consumed_at_unix_ms
            .map(|v| to_i64_ms(v, "auth transaction consumed timestamp"))
            .transpose()?;
        let mut client = self.connection()?;
        client.execute("INSERT INTO vsn_control_auth_transactions(transaction_id,kind,payload,created_at_unix_ms,expires_at_unix_ms,consumed_at_unix_ms) VALUES($1,$2,$3,$4,$5,$6) ON CONFLICT(transaction_id) DO UPDATE SET kind=EXCLUDED.kind,payload=EXCLUDED.payload,created_at_unix_ms=EXCLUDED.created_at_unix_ms,expires_at_unix_ms=EXCLUDED.expires_at_unix_ms,consumed_at_unix_ms=EXCLUDED.consumed_at_unix_ms",&[&record.transaction_id,&record.kind,&record.payload,&created,&expires,&consumed])?;
        Ok(())
    }
    pub fn consume_auth_transaction(
        &self,
        transaction_id: &str,
        kind: &str,
        now_unix_ms: u128,
    ) -> Result<Option<SharedAuthTransaction>, StoreError> {
        validate_route_key(transaction_id)?;
        validate_route_key(kind)?;
        let now = to_i64_ms(now_unix_ms, "auth transaction consume timestamp")?;
        let mut client = self.connection()?;
        let mut tx = client.transaction()?;
        let row=tx.query_opt("SELECT transaction_id,kind,payload,created_at_unix_ms,expires_at_unix_ms,consumed_at_unix_ms FROM vsn_control_auth_transactions WHERE transaction_id=$1 AND kind=$2 FOR UPDATE",&[&transaction_id,&kind])?;
        let Some(row) = row else {
            tx.commit()?;
            return Ok(None);
        };
        let record = Self::shared_auth_transaction_from_row(row);
        if record.consumed_at_unix_ms.is_some() || record.expires_at_unix_ms < now_unix_ms {
            tx.commit()?;
            return Ok(None);
        };
        let changed=tx.execute("UPDATE vsn_control_auth_transactions SET consumed_at_unix_ms=$2 WHERE transaction_id=$1 AND consumed_at_unix_ms IS NULL",&[&transaction_id,&now])?;
        tx.commit()?;
        if changed == 1 {
            Ok(Some(SharedAuthTransaction {
                consumed_at_unix_ms: Some(now_unix_ms),
                ..record
            }))
        } else {
            Ok(None)
        }
    }
    pub fn auth_transaction(
        &self,
        transaction_id: &str,
        kind: &str,
    ) -> Result<Option<SharedAuthTransaction>, StoreError> {
        validate_route_key(transaction_id)?;
        validate_route_key(kind)?;
        let now = now_i64();
        let mut client = self.connection()?;
        let row=client.query_opt("SELECT transaction_id,kind,payload,created_at_unix_ms,expires_at_unix_ms,consumed_at_unix_ms FROM vsn_control_auth_transactions WHERE transaction_id=$1 AND kind=$2 AND consumed_at_unix_ms IS NULL AND expires_at_unix_ms >= $3",&[&transaction_id,&kind,&now])?;
        Ok(row.map(Self::shared_auth_transaction_from_row))
    }
    pub fn cleanup_auth_transactions(&self) -> Result<u64, StoreError> {
        let now = now_i64();
        let retention = now.saturating_sub(24 * 60 * 60 * 1000);
        let mut client = self.connection()?;
        Ok(client.execute("DELETE FROM vsn_control_auth_transactions WHERE expires_at_unix_ms < $1 OR (consumed_at_unix_ms IS NOT NULL AND consumed_at_unix_ms < $2)",&[&now,&retention])?)
    }
    fn shared_auth_transaction_from_row(r: postgres::Row) -> SharedAuthTransaction {
        SharedAuthTransaction {
            transaction_id: r.get(0),
            kind: r.get(1),
            payload: r.get(2),
            created_at_unix_ms: u128::try_from(r.get::<_, i64>(3)).unwrap_or(0),
            expires_at_unix_ms: u128::try_from(r.get::<_, i64>(4)).unwrap_or(0),
            consumed_at_unix_ms: r
                .get::<_, Option<i64>>(5)
                .and_then(|v| u128::try_from(v).ok()),
        }
    }

    pub fn upsert_auth_policy(&self, record: &SharedAuthPolicyRecord) -> Result<(), StoreError> {
        validate_route_key(&record.policy_id)?;
        if record.payload.len() > 2 * 1024 * 1024 {
            return Err(StoreError::Invalid(
                "auth policy payload exceeds 2 MiB".into(),
            ));
        }
        let updated = to_i64_ms(record.updated_at_unix_ms, "auth policy update timestamp")?;
        let mut client = self.connection()?;
        client.execute("INSERT INTO vsn_control_auth_policy(policy_id,payload,updated_at_unix_ms) VALUES($1,$2,$3) ON CONFLICT(policy_id) DO UPDATE SET payload=EXCLUDED.payload,updated_at_unix_ms=EXCLUDED.updated_at_unix_ms",&[&record.policy_id,&record.payload,&updated])?;
        Ok(())
    }
    pub fn auth_policy(
        &self,
        policy_id: &str,
    ) -> Result<Option<SharedAuthPolicyRecord>, StoreError> {
        validate_route_key(policy_id)?;
        let mut client = self.connection()?;
        let row=client.query_opt("SELECT policy_id,payload,updated_at_unix_ms FROM vsn_control_auth_policy WHERE policy_id=$1",&[&policy_id])?;
        Ok(row.map(|r| SharedAuthPolicyRecord {
            policy_id: r.get(0),
            payload: r.get(1),
            updated_at_unix_ms: u128::try_from(r.get::<_, i64>(2)).unwrap_or(0),
        }))
    }

    pub fn scim_group_count(&self) -> Result<u64, StoreError> {
        let mut client = self.connection()?;
        let count: i64 = client
            .query_one("SELECT COUNT(*) FROM vsn_control_scim_groups", &[])?
            .get(0);
        u64::try_from(count).map_err(|_| StoreError::Invalid("negative SCIM group count".into()))
    }
    pub fn upsert_scim_group(&self, record: &SharedScimGroupRecord) -> Result<(), StoreError> {
        validate_route_key(&record.group_id)?;
        if record.display_name.trim().is_empty()
            || record.display_name.len() > 256
            || record.display_name.chars().any(char::is_control)
        {
            return Err(StoreError::Invalid(
                "SCIM group displayName is invalid".into(),
            ));
        }
        if record.payload.len() > 2 * 1024 * 1024 {
            return Err(StoreError::Invalid(
                "SCIM group payload exceeds 2 MiB".into(),
            ));
        }
        let updated = to_i64_ms(record.updated_at_unix_ms, "SCIM group update timestamp")?;
        let mut client = self.connection()?;
        client.execute("INSERT INTO vsn_control_scim_groups(group_id,display_name,payload,updated_at_unix_ms) VALUES($1,$2,$3,$4) ON CONFLICT(group_id) DO UPDATE SET display_name=EXCLUDED.display_name,payload=EXCLUDED.payload,updated_at_unix_ms=EXCLUDED.updated_at_unix_ms",&[&record.group_id,&record.display_name,&record.payload,&updated])?;
        Ok(())
    }
    pub fn list_scim_groups(&self) -> Result<Vec<SharedScimGroupRecord>, StoreError> {
        let mut client = self.connection()?;
        let rows=client.query("SELECT group_id,display_name,payload,updated_at_unix_ms FROM vsn_control_scim_groups ORDER BY display_name",&[])?;
        Ok(rows
            .into_iter()
            .map(|r| SharedScimGroupRecord {
                group_id: r.get(0),
                display_name: r.get(1),
                payload: r.get(2),
                updated_at_unix_ms: u128::try_from(r.get::<_, i64>(3)).unwrap_or(0),
            })
            .collect())
    }
    pub fn scim_group(&self, group_id: &str) -> Result<Option<SharedScimGroupRecord>, StoreError> {
        validate_route_key(group_id)?;
        let mut client = self.connection()?;
        let row=client.query_opt("SELECT group_id,display_name,payload,updated_at_unix_ms FROM vsn_control_scim_groups WHERE group_id=$1",&[&group_id])?;
        Ok(row.map(|r| SharedScimGroupRecord {
            group_id: r.get(0),
            display_name: r.get(1),
            payload: r.get(2),
            updated_at_unix_ms: u128::try_from(r.get::<_, i64>(3)).unwrap_or(0),
        }))
    }
    pub fn delete_scim_group(&self, group_id: &str) -> Result<bool, StoreError> {
        validate_route_key(group_id)?;
        let mut client = self.connection()?;
        Ok(client.execute(
            "DELETE FROM vsn_control_scim_groups WHERE group_id=$1",
            &[&group_id],
        )? == 1)
    }

    pub fn api_token_count(&self) -> Result<u64, StoreError> {
        let mut client = self.connection()?;
        let count: i64 = client
            .query_one("SELECT COUNT(*) FROM vsn_control_api_tokens", &[])?
            .get(0);
        u64::try_from(count).map_err(|_| StoreError::Invalid("negative API token count".into()))
    }
    pub fn upsert_api_token(&self, record: &SharedApiTokenRecord) -> Result<(), StoreError> {
        validate_route_key(&record.token_id)?;
        validate_route_key(&record.principal_id)?;
        validate_route_key(&record.role_id)?;
        validate_sha256_hex(&record.token_hash)?;
        let created = to_i64_ms(record.created_at_unix_ms, "API token created timestamp")?;
        let updated = to_i64_ms(record.updated_at_unix_ms, "API token updated timestamp")?;
        let mut client = self.connection()?;
        client.execute("INSERT INTO vsn_control_api_tokens(token_id,principal_id,role_id,token_hash,created_at_unix_ms,revoked,updated_at_unix_ms) VALUES($1,$2,$3,$4,$5,$6,$7) ON CONFLICT(token_id) DO UPDATE SET principal_id=EXCLUDED.principal_id,role_id=EXCLUDED.role_id,token_hash=EXCLUDED.token_hash,revoked=EXCLUDED.revoked,updated_at_unix_ms=EXCLUDED.updated_at_unix_ms",&[&record.token_id,&record.principal_id,&record.role_id,&record.token_hash,&created,&record.revoked,&updated])?;
        Ok(())
    }
    pub fn api_token_by_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<SharedApiTokenRecord>, StoreError> {
        validate_sha256_hex(token_hash)?;
        let mut client = self.connection()?;
        let row=client.query_opt("SELECT token_id,principal_id,role_id,token_hash,created_at_unix_ms,revoked,updated_at_unix_ms FROM vsn_control_api_tokens WHERE token_hash=$1",&[&token_hash])?;
        Ok(row.map(Self::shared_api_token_from_row))
    }
    pub fn list_api_tokens(&self) -> Result<Vec<SharedApiTokenRecord>, StoreError> {
        let mut client = self.connection()?;
        let rows=client.query("SELECT token_id,principal_id,role_id,token_hash,created_at_unix_ms,revoked,updated_at_unix_ms FROM vsn_control_api_tokens ORDER BY created_at_unix_ms DESC",&[])?;
        Ok(rows
            .into_iter()
            .map(Self::shared_api_token_from_row)
            .collect())
    }
    pub fn revoke_api_token(
        &self,
        token_id: &str,
        updated_at_unix_ms: u128,
    ) -> Result<bool, StoreError> {
        validate_route_key(token_id)?;
        let updated = to_i64_ms(updated_at_unix_ms, "API token update timestamp")?;
        let mut client = self.connection()?;
        Ok(client.execute("UPDATE vsn_control_api_tokens SET revoked=TRUE,updated_at_unix_ms=$2 WHERE token_id=$1",&[&token_id,&updated])?==1)
    }
    fn shared_api_token_from_row(r: postgres::Row) -> SharedApiTokenRecord {
        SharedApiTokenRecord {
            token_id: r.get(0),
            principal_id: r.get(1),
            role_id: r.get(2),
            token_hash: r.get(3),
            created_at_unix_ms: u128::try_from(r.get::<_, i64>(4)).unwrap_or(0),
            revoked: r.get(5),
            updated_at_unix_ms: u128::try_from(r.get::<_, i64>(6)).unwrap_or(0),
        }
    }

    pub fn fleet_group_count(&self) -> Result<u64, StoreError> {
        let mut client = self.connection()?;
        let count: i64 = client
            .query_one("SELECT COUNT(*) FROM vsn_control_fleet_groups", &[])?
            .get(0);
        u64::try_from(count).map_err(|_| StoreError::Invalid("negative fleet group count".into()))
    }
    pub fn upsert_fleet_group(&self, record: &SharedFleetGroupRecord) -> Result<(), StoreError> {
        validate_route_key(&record.group_id)?;
        if record.payload.len() > 2 * 1024 * 1024 {
            return Err(StoreError::Invalid(
                "fleet group payload exceeds 2 MiB".into(),
            ));
        }
        let updated = to_i64_ms(record.updated_at_unix_ms, "fleet group update timestamp")?;
        let mut client = self.connection()?;
        client.execute("INSERT INTO vsn_control_fleet_groups(group_id,payload,updated_at_unix_ms) VALUES($1,$2,$3) ON CONFLICT(group_id) DO UPDATE SET payload=EXCLUDED.payload,updated_at_unix_ms=EXCLUDED.updated_at_unix_ms",&[&record.group_id,&record.payload,&updated])?;
        Ok(())
    }
    pub fn list_fleet_groups(&self) -> Result<Vec<SharedFleetGroupRecord>, StoreError> {
        let mut client = self.connection()?;
        let rows=client.query("SELECT group_id,payload,updated_at_unix_ms FROM vsn_control_fleet_groups ORDER BY group_id",&[])?;
        Ok(rows
            .into_iter()
            .map(|r| SharedFleetGroupRecord {
                group_id: r.get(0),
                payload: r.get(1),
                updated_at_unix_ms: u128::try_from(r.get::<_, i64>(2)).unwrap_or(0),
            })
            .collect())
    }
    pub fn delete_fleet_group(&self, group_id: &str) -> Result<bool, StoreError> {
        validate_route_key(group_id)?;
        let mut client = self.connection()?;
        Ok(client.execute(
            "DELETE FROM vsn_control_fleet_groups WHERE group_id=$1",
            &[&group_id],
        )? == 1)
    }
    pub fn environment_count(&self) -> Result<u64, StoreError> {
        let mut client = self.connection()?;
        let count: i64 = client
            .query_one("SELECT COUNT(*) FROM vsn_control_environments", &[])?
            .get(0);
        u64::try_from(count).map_err(|_| StoreError::Invalid("negative environment count".into()))
    }
    pub fn upsert_environment(&self, record: &SharedEnvironmentRecord) -> Result<(), StoreError> {
        validate_route_key(&record.environment_id)?;
        if record.payload.len() > 2 * 1024 * 1024 {
            return Err(StoreError::Invalid(
                "environment payload exceeds 2 MiB".into(),
            ));
        }
        let updated = to_i64_ms(record.updated_at_unix_ms, "environment update timestamp")?;
        let mut client = self.connection()?;
        client.execute("INSERT INTO vsn_control_environments(environment_id,payload,updated_at_unix_ms) VALUES($1,$2,$3) ON CONFLICT(environment_id) DO UPDATE SET payload=EXCLUDED.payload,updated_at_unix_ms=EXCLUDED.updated_at_unix_ms",&[&record.environment_id,&record.payload,&updated])?;
        Ok(())
    }
    pub fn list_environments_shared(&self) -> Result<Vec<SharedEnvironmentRecord>, StoreError> {
        let mut client = self.connection()?;
        let rows=client.query("SELECT environment_id,payload,updated_at_unix_ms FROM vsn_control_environments ORDER BY environment_id",&[])?;
        Ok(rows
            .into_iter()
            .map(|r| SharedEnvironmentRecord {
                environment_id: r.get(0),
                payload: r.get(1),
                updated_at_unix_ms: u128::try_from(r.get::<_, i64>(2)).unwrap_or(0),
            })
            .collect())
    }
    pub fn delete_environment(&self, environment_id: &str) -> Result<bool, StoreError> {
        validate_route_key(environment_id)?;
        let mut client = self.connection()?;
        Ok(client.execute(
            "DELETE FROM vsn_control_environments WHERE environment_id=$1",
            &[&environment_id],
        )? == 1)
    }

    pub fn upsert_device_fleet(&self, record: &SharedDeviceFleetRecord) -> Result<(), StoreError> {
        validate_route_key(&record.device_id)?;
        if record.payload.len() > 1024 * 1024 {
            return Err(StoreError::Invalid(
                "device fleet payload exceeds 1 MiB".into(),
            ));
        }
        let updated = to_i64_ms(record.updated_at_unix_ms, "device fleet update timestamp")?;
        let mut client = self.connection()?;
        client.execute("INSERT INTO vsn_control_device_fleet(device_id,payload,updated_at_unix_ms) VALUES($1,$2,$3) ON CONFLICT(device_id) DO UPDATE SET payload=EXCLUDED.payload,updated_at_unix_ms=EXCLUDED.updated_at_unix_ms",&[&record.device_id,&record.payload,&updated])?;
        Ok(())
    }
    pub fn device_fleet(
        &self,
        device_id: &str,
    ) -> Result<Option<SharedDeviceFleetRecord>, StoreError> {
        validate_route_key(device_id)?;
        let mut client = self.connection()?;
        let row=client.query_opt("SELECT device_id,payload,updated_at_unix_ms FROM vsn_control_device_fleet WHERE device_id=$1",&[&device_id])?;
        Ok(row.map(|r| SharedDeviceFleetRecord {
            device_id: r.get(0),
            payload: r.get(1),
            updated_at_unix_ms: u128::try_from(r.get::<_, i64>(2)).unwrap_or(0),
        }))
    }
    pub fn list_device_fleet(&self) -> Result<Vec<SharedDeviceFleetRecord>, StoreError> {
        let mut client = self.connection()?;
        let rows=client.query("SELECT device_id,payload,updated_at_unix_ms FROM vsn_control_device_fleet ORDER BY device_id",&[])?;
        Ok(rows
            .into_iter()
            .map(|r| SharedDeviceFleetRecord {
                device_id: r.get(0),
                payload: r.get(1),
                updated_at_unix_ms: u128::try_from(r.get::<_, i64>(2)).unwrap_or(0),
            })
            .collect())
    }
    pub fn upsert_team_secret(&self, record: &SharedTeamSecretRecord) -> Result<(), StoreError> {
        validate_team_secret_name(&record.name)?;
        validate_route_key(&record.created_by)?;
        validate_vault_key_id(&record.key_id)?;
        if record.nonce_b64.len() > 256 || record.ciphertext_b64.len() > 2 * 1024 * 1024 {
            return Err(StoreError::Invalid(
                "team secret encrypted payload exceeds limits".into(),
            ));
        }
        let updated = to_i64_ms(record.updated_at_unix_ms, "team secret update timestamp")?;
        let mut client = self.connection()?;
        client.execute("INSERT INTO vsn_control_team_secrets(name,key_id,nonce_b64,ciphertext_b64,created_by,updated_at_unix_ms) VALUES($1,$2,$3,$4,$5,$6) ON CONFLICT(name) DO UPDATE SET key_id=EXCLUDED.key_id,nonce_b64=EXCLUDED.nonce_b64,ciphertext_b64=EXCLUDED.ciphertext_b64,created_by=EXCLUDED.created_by,updated_at_unix_ms=EXCLUDED.updated_at_unix_ms",&[&record.name,&record.key_id,&record.nonce_b64,&record.ciphertext_b64,&record.created_by,&updated])?;
        Ok(())
    }
    pub fn team_secret(&self, name: &str) -> Result<Option<SharedTeamSecretRecord>, StoreError> {
        validate_team_secret_name(name)?;
        let mut client = self.connection()?;
        let row=client.query_opt("SELECT name,key_id,nonce_b64,ciphertext_b64,created_by,updated_at_unix_ms FROM vsn_control_team_secrets WHERE name=$1",&[&name])?;
        Ok(row.map(|r| SharedTeamSecretRecord {
            name: r.get(0),
            key_id: r.get(1),
            nonce_b64: r.get(2),
            ciphertext_b64: r.get(3),
            created_by: r.get(4),
            updated_at_unix_ms: u128::try_from(r.get::<_, i64>(5)).unwrap_or(0),
        }))
    }
    pub fn list_team_secrets(&self) -> Result<Vec<SharedTeamSecretRecord>, StoreError> {
        let mut client = self.connection()?;
        let rows=client.query("SELECT name,key_id,nonce_b64,ciphertext_b64,created_by,updated_at_unix_ms FROM vsn_control_team_secrets ORDER BY name",&[])?;
        Ok(rows
            .into_iter()
            .map(|r| SharedTeamSecretRecord {
                name: r.get(0),
                key_id: r.get(1),
                nonce_b64: r.get(2),
                ciphertext_b64: r.get(3),
                created_by: r.get(4),
                updated_at_unix_ms: u128::try_from(r.get::<_, i64>(5)).unwrap_or(0),
            })
            .collect())
    }
    pub fn delete_team_secret(&self, name: &str) -> Result<bool, StoreError> {
        validate_team_secret_name(name)?;
        let mut client = self.connection()?;
        Ok(client.execute(
            "DELETE FROM vsn_control_team_secrets WHERE name=$1",
            &[&name],
        )? == 1)
    }
    pub fn team_vault_active_key(&self) -> Result<Option<String>, StoreError> {
        let mut client = self.connection()?;
        Ok(client
            .query_opt(
                "SELECT active_key_id FROM vsn_control_team_vault_meta WHERE singleton=TRUE",
                &[],
            )?
            .map(|r| r.get(0)))
    }
    pub fn set_team_vault_active_key(&self, key_id: &str) -> Result<(), StoreError> {
        validate_vault_key_id(key_id)?;
        let now = now_i64();
        let mut client = self.connection()?;
        client.execute("INSERT INTO vsn_control_team_vault_meta(singleton,active_key_id,updated_at_unix_ms) VALUES(TRUE,$1,$2) ON CONFLICT(singleton) DO UPDATE SET active_key_id=EXCLUDED.active_key_id,updated_at_unix_ms=EXCLUDED.updated_at_unix_ms",&[&key_id,&now])?;
        Ok(())
    }
    pub fn rotate_team_secrets(
        &self,
        records: &[SharedTeamSecretRecord],
        new_key_id: &str,
    ) -> Result<u64, StoreError> {
        validate_vault_key_id(new_key_id)?;
        let mut client = self.connection()?;
        let mut tx = client.transaction()?;
        let now = now_i64();
        for record in records {
            validate_team_secret_name(&record.name)?;
            validate_route_key(&record.created_by)?;
            validate_vault_key_id(&record.key_id)?;
            let updated = to_i64_ms(record.updated_at_unix_ms, "team secret rotation timestamp")?;
            tx.execute("UPDATE vsn_control_team_secrets SET key_id=$2,nonce_b64=$3,ciphertext_b64=$4,created_by=$5,updated_at_unix_ms=$6 WHERE name=$1",&[&record.name,&record.key_id,&record.nonce_b64,&record.ciphertext_b64,&record.created_by,&updated])?;
        }
        tx.execute("INSERT INTO vsn_control_team_vault_meta(singleton,active_key_id,updated_at_unix_ms) VALUES(TRUE,$1,$2) ON CONFLICT(singleton) DO UPDATE SET active_key_id=EXCLUDED.active_key_id,updated_at_unix_ms=EXCLUDED.updated_at_unix_ms",&[&new_key_id,&now])?;
        tx.commit()?;
        Ok(records.len() as u64)
    }

    fn shared_approval_from_row(r: postgres::Row) -> SharedApprovalRecord {
        SharedApprovalRecord {
            approval_id: r.get(0),
            payload: r.get(1),
            state: r.get(2),
            created_at_unix_ms: u128::try_from(r.get::<_, i64>(3)).unwrap_or(0),
            expires_at_unix_ms: u128::try_from(r.get::<_, i64>(4)).unwrap_or(0),
            approver_id: r.get(5),
            decided_at_unix_ms: r
                .get::<_, Option<i64>>(6)
                .and_then(|v| u128::try_from(v).ok()),
            command_id: r.get(7),
        }
    }

    fn shared_command_from_row(r: postgres::Row) -> SharedCommandRecord {
        SharedCommandRecord {
            command_id: r.get(0),
            device_id: r.get(1),
            payload: r.get(2),
            state: r.get(3),
            attempts: u32::try_from(r.get::<_, i64>(4)).unwrap_or(u32::MAX),
            leased_by: r.get(5),
            lease_until_unix_ms: r
                .get::<_, Option<i64>>(6)
                .and_then(|v| u128::try_from(v).ok()),
            expires_at_unix_ms: u128::try_from(r.get::<_, i64>(7)).unwrap_or(0),
            created_at_unix_ms: u128::try_from(r.get::<_, i64>(8)).unwrap_or(0),
            completed_at_unix_ms: r
                .get::<_, Option<i64>>(9)
                .and_then(|v| u128::try_from(v).ok()),
            last_error: r.get(10),
            result_payload: r.get(11),
        }
    }

    fn connection(&self) -> Result<PgClient, StoreError> {
        let pem = fs::read(&self.root_ca_pem_path)?;
        let cert = Certificate::from_pem(&pem)?;
        let mut builder = TlsConnector::builder();
        builder.add_root_certificate(cert);
        let tls = MakeTlsConnector::new(builder.build()?);
        let mut config = PgConfig::from_str(&self.connection_string)?;
        config.ssl_mode(SslMode::Require);
        Ok(config.connect(tls)?)
    }
}

fn snapshot_from_pg_row(name: &str, row: postgres::Row) -> Result<Option<Snapshot>, StoreError> {
    let generation_i64: i64 = row.get(0);
    let payload: Vec<u8> = row.get(1);
    let sha256: String = row.get(2);
    let updated: String = row.get(3);
    if sha256_hex(&payload) != sha256 {
        return Err(StoreError::HashMismatch);
    }
    Ok(Some(Snapshot {
        name: name.into(),
        generation: u64::try_from(generation_i64)
            .map_err(|_| StoreError::Invalid("negative generation".into()))?,
        payload,
        sha256,
        updated_at_unix_ms: updated
            .parse::<u128>()
            .map_err(|_| StoreError::Invalid("invalid timestamp".into()))?,
    }))
}

fn validate_root_ca(path: &Path) -> Result<PathBuf, StoreError> {
    if !path.is_absolute() {
        return Err(StoreError::Invalid("root CA path must be absolute".into()));
    }
    let metadata = fs::metadata(path)?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > 4 * 1024 * 1024 {
        return Err(StoreError::Invalid(
            "root CA must be an existing file between 1 byte and 4 MiB".into(),
        ));
    }
    Ok(path.to_path_buf())
}

fn validate_payload(payload: &[u8]) -> Result<(), StoreError> {
    if payload.len() > 128 * 1024 * 1024 {
        return Err(StoreError::Invalid("snapshot exceeds 128 MiB".into()));
    }
    Ok(())
}

fn validate_sha256_hex(value: &str) -> Result<(), StoreError> {
    if value.len() != 64 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(StoreError::Invalid("value must be SHA-256 hex".into()));
    }
    Ok(())
}
fn validate_team_secret_name(value: &str) -> Result<(), StoreError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
    {
        return Err(StoreError::Invalid("team secret name is invalid".into()));
    }
    Ok(())
}
fn validate_vault_key_id(value: &str) -> Result<(), StoreError> {
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
    {
        return Err(StoreError::Invalid("team Vault key id is invalid".into()));
    }
    Ok(())
}
fn validate_route_key(value: &str) -> Result<(), StoreError> {
    if value.len() < 2
        || value.len() > 160
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':'))
    {
        return Err(StoreError::Invalid(
            "route key must be a safe identifier".into(),
        ));
    }
    Ok(())
}
fn validate_name(name: &str) -> Result<(), StoreError> {
    if name.len() < 2
        || name.len() > 96
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    {
        return Err(StoreError::Invalid(
            "snapshot name must be a safe identifier".into(),
        ));
    }
    Ok(())
}
fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}
fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}
fn now_i64() -> i64 {
    i64::try_from(now_ms()).unwrap_or(i64::MAX)
}
fn to_i64_ms(value: u128, label: &str) -> Result<i64, StoreError> {
    i64::try_from(value)
        .map_err(|_| StoreError::Invalid(format!("{label} exceeds PostgreSQL integer range")))
}
#[cfg(unix)]
fn harden_file(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    if path.exists() {
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}
#[cfg(not(unix))]
fn harden_file(_path: &Path) -> std::io::Result<()> {
    Ok(())
}
#[cfg(unix)]
fn harden_dir(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
}
#[cfg(not(unix))]
fn harden_dir(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn snapshot_roundtrip_and_generation() {
        let temp_root = std::env::temp_dir().join(format!("vsn-control-store-{}", now_ms()));
        let path = temp_root.join("store.db");
        let store = SnapshotStore::open(&path).unwrap();
        let a = store.save("control-plane", br#"{\"a\":1}"#).unwrap();
        let b = store.save("control-plane", br#"{\"a\":2}"#).unwrap();
        assert_eq!(a.generation, 1);
        assert_eq!(b.generation, 2);
        assert_eq!(
            store.load("control-plane").unwrap().unwrap().payload,
            br#"{\"a\":2}"#
        );
        let c = store
            .save_if_generation("control-plane", 2, br#"{\"a\":3}"#)
            .unwrap();
        assert_eq!(c.generation, 3);
        assert!(matches!(
            store.save_if_generation("control-plane", 2, b"stale"),
            Err(StoreError::GenerationConflict { .. })
        ));
        let _ = fs::remove_file(&path);
        let _ = fs::remove_dir(temp_root);
    }
}
