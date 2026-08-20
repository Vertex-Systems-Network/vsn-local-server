// Design scaffold only; this is intentionally not compiled into the local workspace yet.
pub struct DeviceEnrollmentV1 {
    pub device_id: String,
    pub public_key: String,
    pub pairing_nonce: String,
    pub proof_signature: String,
}

pub struct RemoteCommandV1 {
    pub command_id: String,
    pub device_id: String,
    pub principal_id: String,
    pub issued_at_unix_ms: u128,
    pub expires_at_unix_ms: u128,
    pub permission: String,
    pub command: String,
    pub session_id: String,
    pub signature: String,
}
