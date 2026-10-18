use fs2::FileExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{self, BufRead, BufReader, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error;
use uuid::Uuid;
use vsn_security::{verify_signature, DeviceIdentity, SecurityError};

#[derive(Debug, Error)]
pub enum AuditError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("security error: {0}")]
    Security(#[from] SecurityError),
    #[error("audit chain is invalid at line {0}")]
    InvalidChain(usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEventInput {
    pub actor_type: String,
    pub actor_id: String,
    pub action: String,
    pub target: String,
    pub result: String,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub version: u32,
    pub event_id: String,
    pub timestamp_unix_ms: u128,
    pub device_id: String,
    pub signer_public_key: String,
    pub actor_type: String,
    pub actor_id: String,
    pub action: String,
    pub target: String,
    pub result: String,
    pub metadata: BTreeMap<String, String>,
    pub previous_hash: String,
    pub event_hash: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize)]
struct CanonicalEvent<'a> {
    version: u32,
    event_id: &'a str,
    timestamp_unix_ms: u128,
    device_id: &'a str,
    signer_public_key: &'a str,
    actor_type: &'a str,
    actor_id: &'a str,
    action: &'a str,
    target: &'a str,
    result: &'a str,
    metadata: &'a BTreeMap<String, String>,
    previous_hash: &'a str,
}

pub fn default_audit_path() -> Result<PathBuf, AuditError> {
    Ok(vsn_security::data_dir()?.join("audit").join("agent.jsonl"))
}

pub fn append(
    path: &Path,
    identity: &DeviceIdentity,
    input: AuditEventInput,
) -> Result<AuditEvent, AuditError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(path)?;
    file.lock_exclusive()?;

    let operation = (|| -> Result<AuditEvent, AuditError> {
        let previous_hash = last_hash_locked(&mut file)?.unwrap_or_else(|| "GENESIS".to_string());
        let meta = identity.metadata();
        let event_id = Uuid::new_v4().to_string();
        let timestamp_unix_ms = now_ms();

        let canonical = CanonicalEvent {
            version: 1,
            event_id: &event_id,
            timestamp_unix_ms,
            device_id: &meta.device_id,
            signer_public_key: &meta.public_key,
            actor_type: &input.actor_type,
            actor_id: &input.actor_id,
            action: &input.action,
            target: &input.target,
            result: &input.result,
            metadata: &input.metadata,
            previous_hash: &previous_hash,
        };
        let canonical_bytes = serde_json::to_vec(&canonical)?;
        let event_hash = sha256_hex(&canonical_bytes);
        let signature = identity.sign(event_hash.as_bytes());

        let event = AuditEvent {
            version: 1,
            event_id,
            timestamp_unix_ms,
            device_id: meta.device_id.clone(),
            signer_public_key: meta.public_key.clone(),
            actor_type: input.actor_type,
            actor_id: input.actor_id,
            action: input.action,
            target: input.target,
            result: input.result,
            metadata: input.metadata,
            previous_hash,
            event_hash,
            signature,
        };

        file.seek(SeekFrom::End(0))?;
        serde_json::to_writer(&mut file, &event)?;
        file.write_all(b"\n")?;
        file.sync_data()?;
        Ok(event)
    })();

    let unlock_result = file.unlock();
    match (operation, unlock_result) {
        (Ok(event), Ok(())) => Ok(event),
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(AuditError::Io(error)),
    }
}

pub fn verify(path: &Path) -> Result<usize, AuditError> {
    if !path.exists() {
        return Ok(0);
    }
    let file = File::open(path)?;
    file.lock_shared()?;
    let result = verify_reader(BufReader::new(&file));
    let unlock_result = file.unlock();
    match (result, unlock_result) {
        (Ok(count), Ok(())) => Ok(count),
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(AuditError::Io(error)),
    }
}

fn verify_reader<R: BufRead>(reader: R) -> Result<usize, AuditError> {
    let mut expected_previous = "GENESIS".to_string();
    let mut count = 0usize;

    for (index, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let event: AuditEvent = serde_json::from_str(&line)?;
        if event.previous_hash != expected_previous {
            return Err(AuditError::InvalidChain(index + 1));
        }
        let canonical = CanonicalEvent {
            version: event.version,
            event_id: &event.event_id,
            timestamp_unix_ms: event.timestamp_unix_ms,
            device_id: &event.device_id,
            signer_public_key: &event.signer_public_key,
            actor_type: &event.actor_type,
            actor_id: &event.actor_id,
            action: &event.action,
            target: &event.target,
            result: &event.result,
            metadata: &event.metadata,
            previous_hash: &event.previous_hash,
        };
        let canonical_bytes = serde_json::to_vec(&canonical)?;
        let computed_hash = sha256_hex(&canonical_bytes);
        if computed_hash != event.event_hash {
            return Err(AuditError::InvalidChain(index + 1));
        }
        verify_signature(
            &event.signer_public_key,
            event.event_hash.as_bytes(),
            &event.signature,
        )
        .map_err(|_| AuditError::InvalidChain(index + 1))?;
        expected_previous = event.event_hash;
        count += 1;
    }
    Ok(count)
}

/// Recompute an event's hash from its canonical fields.
fn computed_hash(event: &AuditEvent) -> Result<String, AuditError> {
    let canonical = CanonicalEvent {
        version: event.version,
        event_id: &event.event_id,
        timestamp_unix_ms: event.timestamp_unix_ms,
        device_id: &event.device_id,
        signer_public_key: &event.signer_public_key,
        actor_type: &event.actor_type,
        actor_id: &event.actor_id,
        action: &event.action,
        target: &event.target,
        result: &event.result,
        metadata: &event.metadata,
        previous_hash: &event.previous_hash,
    };
    Ok(sha256_hex(&serde_json::to_vec(&canonical)?))
}

/// Check one link of the chain: the event points at `expected_previous` and its stored
/// hash matches its contents. Signatures are checked separately where needed.
fn check_link(event: &AuditEvent, expected_previous: &str, line: usize) -> Result<(), AuditError> {
    if event.previous_hash != expected_previous || computed_hash(event)? != event.event_hash {
        return Err(AuditError::InvalidChain(line));
    }
    Ok(())
}

pub fn verify_event(event: &AuditEvent) -> Result<(), AuditError> {
    let canonical = CanonicalEvent {
        version: event.version,
        event_id: &event.event_id,
        timestamp_unix_ms: event.timestamp_unix_ms,
        device_id: &event.device_id,
        signer_public_key: &event.signer_public_key,
        actor_type: &event.actor_type,
        actor_id: &event.actor_id,
        action: &event.action,
        target: &event.target,
        result: &event.result,
        metadata: &event.metadata,
        previous_hash: &event.previous_hash,
    };
    let canonical_bytes = serde_json::to_vec(&canonical)?;
    let computed_hash = sha256_hex(&canonical_bytes);
    if computed_hash != event.event_hash {
        return Err(AuditError::InvalidChain(1));
    }
    verify_signature(
        &event.signer_public_key,
        event.event_hash.as_bytes(),
        &event.signature,
    )
    .map_err(|_| AuditError::InvalidChain(1))?;
    Ok(())
}

pub fn read_events_after(
    path: &Path,
    after_event_id: Option<&str>,
    limit: usize,
) -> Result<Vec<AuditEvent>, AuditError> {
    if !path.exists() || limit == 0 {
        return Ok(Vec::new());
    }
    let file = File::open(path)?;
    file.lock_shared()?;
    let result = (|| -> Result<Vec<AuditEvent>, AuditError> {
        let mut out = Vec::new();
        let mut found = after_event_id.is_none();
        // Every record up to the end of the page must link to the one before it, so a
        // page (and the cursor it starts after) is only accepted on an unbroken chain.
        let mut expected_previous = "GENESIS".to_string();
        for (index, line) in BufReader::new(&file).lines().enumerate() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let event: AuditEvent = serde_json::from_str(&line)?;
            check_link(&event, &expected_previous, index + 1)?;
            expected_previous = event.event_hash.clone();
            if !found {
                if Some(event.event_id.as_str()) == after_event_id {
                    found = true;
                }
                continue;
            }
            verify_event(&event)?;
            out.push(event);
            if out.len() >= limit.min(512) {
                break;
            }
        }
        if after_event_id.is_some() && !found {
            return Err(AuditError::InvalidChain(0));
        }
        Ok(out)
    })();
    let unlock_result = file.unlock();
    match (result, unlock_result) {
        (Ok(v), Ok(())) => Ok(v),
        (Err(e), _) => Err(e),
        (Ok(_), Err(e)) => Err(AuditError::Io(e)),
    }
}

fn last_hash_locked(file: &mut File) -> Result<Option<String>, AuditError> {
    file.seek(SeekFrom::Start(0))?;
    let cloned = file.try_clone()?;
    let reader = BufReader::new(cloned);
    // Never extend a damaged chain: every link must hold and the tail must carry a valid
    // signature before a new record is bound to its hash.
    let mut last: Option<AuditEvent> = None;
    for (index, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let event: AuditEvent = serde_json::from_str(&line)?;
        let expected = last.as_ref().map_or("GENESIS", |e| e.event_hash.as_str());
        check_link(&event, expected, index + 1)?;
        last = Some(event);
    }
    if let Some(tail) = &last {
        verify_event(tail)?;
    }
    Ok(last.map(|e| e.event_hash))
}

fn sha256_hex(data: &[u8]) -> String {
    let digest = Sha256::digest(data);
    let mut out = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(previous: &str, n: u32) -> AuditEvent {
        let mut e = AuditEvent {
            version: 1,
            event_id: format!("e{n}"),
            timestamp_unix_ms: n as u128,
            device_id: "d".into(),
            signer_public_key: "k".into(),
            actor_type: "user".into(),
            actor_id: "a".into(),
            action: "x".into(),
            target: "t".into(),
            result: "ok".into(),
            metadata: BTreeMap::new(),
            previous_hash: previous.into(),
            event_hash: String::new(),
            signature: String::new(),
        };
        e.event_hash = computed_hash(&e).unwrap();
        e
    }

    fn write(events: &[AuditEvent]) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "vsn-audit-test-{}-{}.jsonl",
            std::process::id(),
            now_ms()
        ));
        let mut text = String::new();
        for e in events {
            text.push_str(&serde_json::to_string(e).unwrap());
            text.push('\n');
        }
        fs::write(&path, text).unwrap();
        path
    }

    #[test]
    fn broken_edges_are_rejected_by_append_and_paged_reads() {
        let a = event("GENESIS", 1);
        let b = event(&a.event_hash, 2);
        assert!(check_link(&b, &a.event_hash, 2).is_ok());
        // `c` is internally consistent but points at the wrong predecessor.
        let c = event("not-the-tail", 3);
        let path = write(&[a.clone(), b.clone(), c]);
        let mut file = OpenOptions::new().read(true).open(&path).unwrap();
        assert!(matches!(
            last_hash_locked(&mut file),
            Err(AuditError::InvalidChain(3))
        ));
        // A page that starts after the cursor still fails on the broken edge.
        assert!(matches!(
            read_events_after(&path, Some("e2"), 10),
            Err(AuditError::InvalidChain(3))
        ));
        // A tampered field breaks the stored hash.
        let _ = fs::remove_file(&path);
        let mut d = event(&b.event_hash, 4);
        d.target = "changed".into();
        assert!(check_link(&d, &b.event_hash, 4).is_err());
    }
}
