use axum::{
    extract::{
        ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
        DefaultBodyLimit, Form, Path as AxumPath, Query, State,
    },
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Key, Nonce,
};
use futures_util::{SinkExt, StreamExt};
use openidconnect::core::{CoreClient, CoreProviderMetadata};
use openidconnect::{
    AuthorizationCode, ClientId, ClientSecret, IssuerUrl, Nonce as OidcNonce, PkceCodeVerifier,
    RedirectUrl, TokenResponse,
};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tokio::sync::mpsc;
use tower_http::services::ServeDir;
use vsn_remote::{
    AgentAuditBatchV1, AgentCommandResultV1, AgentPollResponseV1, AgentPollV1, DeviceEnrollmentV1,
    RemoteCommandV1,
};
use webauthn_rs::prelude::{
    Passkey, PasskeyAuthentication, PasskeyRegistration, PublicKeyCredential,
    RegisterPublicKeyCredential, Url, Uuid, Webauthn, WebauthnBuilder,
};

const VERSION: &str = "0.38.1";
const DELIVERY_LEASE_MS: u128 = 30_000;
const MAX_DELIVERY_ATTEMPTS: u32 = 5;

#[derive(Clone)]
struct AppState {
    admin_token: Arc<String>,
    private_key: Arc<String>,
    public_key: Arc<String>,
    pairings: Arc<Mutex<HashMap<String, u128>>>,
    devices: Arc<Mutex<HashMap<String, DeviceRecord>>>,
    queues: Arc<Mutex<HashMap<String, VecDeque<RemoteCommandV1>>>>,
    deliveries: Arc<Mutex<HashMap<String, DeliveryMeta>>>,
    results: Arc<Mutex<Vec<AgentCommandResultV1>>>,
    roles: Arc<Mutex<HashMap<String, IamRole>>>,
    tokens: Arc<Mutex<HashMap<String, ApiTokenRecord>>>,
    poll_replay: Arc<Mutex<ReplayWindow>>,
    result_replay: Arc<Mutex<ReplayWindow>>,
    fleet_groups: Arc<Mutex<HashMap<String, FleetGroup>>>,
    environments: Arc<Mutex<HashMap<String, EnvironmentRecord>>>,
    approvals: Arc<Mutex<HashMap<String, ApprovalRecord>>>,
    approval_decision_lock: Arc<Mutex<()>>,
    central_audit: Arc<Mutex<Vec<vsn_audit::AuditEvent>>>,
    auth_policy: Arc<Mutex<vsn_auth::EnterpriseAuthPolicy>>,
    accounts: Arc<Mutex<HashMap<String, AccountRecord>>>,
    scim_groups: Arc<Mutex<HashMap<String, ScimGroupRecord>>>,
    sessions: Arc<Mutex<HashMap<String, AccountSessionRecord>>>,
    oidc_transactions: Arc<Mutex<HashMap<String, PendingOidcTransaction>>>,
    saml_transactions: Arc<Mutex<HashMap<String, vsn_saml::SamlLoginTransaction>>>,
    webauthn: Arc<Option<Webauthn>>,
    passkey_registrations: Arc<Mutex<HashMap<String, PendingPasskeyRegistration>>>,
    passkey_authentications: Arc<Mutex<HashMap<String, PendingPasskeyAuthentication>>>,
    auth_encryption_key: Arc<Option<[u8; 32]>>,
    team_vault_keys: Arc<TeamVaultKeyRing>,
    rate_limits: Arc<Mutex<HashMap<String, VecDeque<u128>>>>,
    state_path: Arc<PathBuf>,
    state_postgres: Arc<Option<vsn_control_store::PostgresSnapshotStore>>,
    persist_lock: Arc<Mutex<()>>,
    state_generation: Arc<Mutex<u64>>,
    agent_stream_peers: Arc<tokio::sync::Mutex<HashMap<String, AgentStreamPeer>>>,
    stream_relays: Arc<tokio::sync::Mutex<HashMap<String, StreamRelayRecord>>>,
    remote_stream_homes: Arc<tokio::sync::Mutex<HashMap<String, RemoteRelayHome>>>,
    instance_id: Arc<String>,
    public_endpoint: Arc<String>,
    started_at_unix_ms: u128,
}

struct ReplayWindow {
    capacity: usize,
    order: VecDeque<String>,
    seen: HashSet<String>,
}
impl ReplayWindow {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            order: VecDeque::new(),
            seen: HashSet::new(),
        }
    }
    fn insert(&mut self, id: String) -> bool {
        if self.seen.contains(&id) {
            return false;
        }
        self.seen.insert(id.clone());
        self.order.push_back(id);
        while self.order.len() > self.capacity {
            if let Some(old) = self.order.pop_front() {
                self.seen.remove(&old);
            }
        }
        true
    }
}

#[derive(Clone)]
struct AgentStreamPeer {
    connection_id: String,
    tx: mpsc::Sender<vsn_remote::AgentStreamServerMessageV1>,
}
#[derive(Clone)]
struct RemoteRelayHome {
    home_instance_id: String,
    device_id: String,
    last_activity_unix_ms: u128,
}
#[derive(Clone)]
struct StreamRelayRecord {
    device_id: String,
    principal_id: String,
    permission: String,
    request: vsn_remote::RelayStreamOpenV1,
    browser_tx: Option<mpsc::Sender<vsn_remote::BrowserStreamServerMessageV1>>,
    resume_token_hash: String,
    pending_resume_token: Option<String>,
    resource_id: Option<String>,
    created_at_unix_ms: u128,
    last_activity_unix_ms: u128,
    detached_until_unix_ms: Option<u128>,
    next_input_seq: u64,
    acked_input_seq: u64,
    committed_bytes: Option<u64>,
    resource_progress_bytes: u64,
    history: VecDeque<vsn_remote::RelayStreamFrameV1>,
    history_bytes: usize,
    agent_instance_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
enum ClusterStreamBusV1 {
    ToAgent {
        home_instance_id: String,
        device_id: String,
        relay_id: String,
        message: vsn_remote::AgentStreamServerMessageV1,
    },
    ToBrowser {
        relay_id: String,
        message: vsn_remote::BrowserStreamServerMessageV1,
    },
}
const MAX_ACTIVE_STREAM_RELAYS: usize = 4096;
const STREAM_RELAY_IDLE_MS: u128 = 15 * 60 * 1000;
const STREAM_RELAY_RESUME_MS: u128 = vsn_remote::STREAM_RELAY_RESUME_TTL_MS;
const STREAM_RELAY_HISTORY_BYTES: usize = 4 * 1024 * 1024;
const STREAM_RELAY_HISTORY_FRAMES: usize = 256;
const STREAM_RELAY_SHARED_TTL_MS: u128 = 15 * 60 * 1000;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeviceRecord {
    device_id: String,
    public_key: String,
    display_name: String,
    os: String,
    enrolled_at_unix_ms: u128,
    last_seen_unix_ms: u128,
    #[serde(default)]
    labels: BTreeMap<String, String>,
    #[serde(default)]
    groups: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum DeliveryState {
    Queued,
    Inflight,
    Completed,
    Failed,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeliveryMeta {
    state: DeliveryState,
    attempts: u32,
    leased_until_unix_ms: Option<u128>,
    completed_at_unix_ms: Option<u128>,
    last_error: Option<String>,
}
impl Default for DeliveryMeta {
    fn default() -> Self {
        Self {
            state: DeliveryState::Queued,
            attempts: 0,
            leased_until_unix_ms: None,
            completed_at_unix_ms: None,
            last_error: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct IamRole {
    id: String,
    name: String,
    permissions: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApiTokenRecord {
    id: String,
    principal_id: String,
    role_id: String,
    token_hash: String,
    created_at_unix_ms: u128,
    revoked: bool,
}
#[derive(Debug, Clone)]
struct AuthPrincipal {
    id: String,
    permissions: HashSet<String>,
    bootstrap: bool,
}
impl AuthPrincipal {
    fn allows(&self, permission: &str) -> bool {
        self.bootstrap || self.permissions.contains("*") || self.permissions.contains(permission)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PersistentState {
    #[serde(default)]
    pairings: HashMap<String, u128>,
    #[serde(default)]
    devices: HashMap<String, DeviceRecord>,
    #[serde(default)]
    queues: HashMap<String, VecDeque<RemoteCommandV1>>,
    #[serde(default)]
    deliveries: HashMap<String, DeliveryMeta>,
    #[serde(default)]
    results: Vec<AgentCommandResultV1>,
    #[serde(default)]
    roles: HashMap<String, IamRole>,
    #[serde(default)]
    tokens: HashMap<String, ApiTokenRecord>,
    #[serde(default)]
    fleet_groups: HashMap<String, FleetGroup>,
    #[serde(default)]
    environments: HashMap<String, EnvironmentRecord>,
    #[serde(default)]
    approvals: HashMap<String, ApprovalRecord>,
    #[serde(default)]
    central_audit: Vec<vsn_audit::AuditEvent>,
    #[serde(default)]
    auth_policy: vsn_auth::EnterpriseAuthPolicy,
    #[serde(default)]
    accounts: HashMap<String, AccountRecord>,
    #[serde(default)]
    scim_groups: HashMap<String, ScimGroupRecord>,
    #[serde(default)]
    sessions: HashMap<String, AccountSessionRecord>,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    ok: bool,
    version: &'static str,
    public_key: String,
}
#[derive(Debug, Serialize)]
struct PairingResponse {
    pairing_nonce: String,
    expires_at_unix_ms: u128,
    control_plane_public_key: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CommandRequest {
    device_id: String,
    permission: String,
    command: String,
    #[serde(default)]
    params: Value,
    #[serde(default = "default_ttl")]
    ttl_ms: u128,
}
fn default_ttl() -> u128 {
    60_000
}
#[derive(Debug, Deserialize)]
struct RoleRequest {
    id: String,
    name: String,
    permissions: Vec<String>,
}
#[derive(Debug, Deserialize)]
struct TokenRequest {
    principal_id: String,
    role_id: String,
}
#[derive(Debug, Deserialize)]
struct RevokeTokenRequest {
    token_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FleetGroup {
    id: String,
    name: String,
    #[serde(default)]
    device_ids: Vec<String>,
    #[serde(default)]
    labels: BTreeMap<String, String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EnvironmentRecord {
    id: String,
    name: String,
    #[serde(default)]
    bindings: BTreeMap<String, String>,
    #[serde(default)]
    labels: BTreeMap<String, String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ApprovalState {
    Pending,
    Approved,
    Rejected,
    Expired,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApprovalRecord {
    id: String,
    requester_id: String,
    requested_at_unix_ms: u128,
    expires_at_unix_ms: u128,
    state: ApprovalState,
    request: CommandRequest,
    approver_id: Option<String>,
    decided_at_unix_ms: Option<u128>,
}
#[derive(Debug, Deserialize)]
struct FleetGroupRequest {
    id: String,
    name: String,
    #[serde(default)]
    device_ids: Vec<String>,
    #[serde(default)]
    labels: BTreeMap<String, String>,
}
#[derive(Debug, Deserialize)]
struct DeviceFleetUpdate {
    device_id: String,
    #[serde(default)]
    labels: BTreeMap<String, String>,
    #[serde(default)]
    groups: Vec<String>,
}
#[derive(Debug, Deserialize)]
struct EnvironmentRequest {
    id: String,
    name: String,
    #[serde(default)]
    bindings: BTreeMap<String, String>,
    #[serde(default)]
    labels: BTreeMap<String, String>,
}
#[derive(Debug, Deserialize)]
struct ApprovalDecision {
    approval_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EncryptedAuthSecret {
    nonce_b64: String,
    ciphertext_b64: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OidcIdentity {
    provider_id: String,
    subject: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SamlIdentity {
    provider_id: String,
    subject: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AccountRecord {
    id: String,
    email: String,
    password_hash: String,
    role_id: String,
    created_at_unix_ms: u128,
    disabled: bool,
    #[serde(default)]
    totp_secret: Option<EncryptedAuthSecret>,
    #[serde(default)]
    last_totp_step: Option<u64>,
    #[serde(default)]
    recovery_code_hashes: Vec<String>,
    #[serde(default)]
    passkeys: Vec<Passkey>,
    #[serde(default)]
    oidc_identities: Vec<OidcIdentity>,
    #[serde(default)]
    saml_identities: Vec<SamlIdentity>,
    #[serde(default)]
    managed_by_scim: bool,
    #[serde(default)]
    scim_external_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FederatedSessionContext {
    kind: String,
    provider_id: String,
    subject: String,
    #[serde(default)]
    session_index: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AccountSessionRecord {
    id: String,
    account_id: String,
    token_hash: String,
    created_at_unix_ms: u128,
    expires_at_unix_ms: u128,
    last_activity_unix_ms: u128,
    mfa_verified: bool,
    passkey_verified: bool,
    revoked: bool,
    #[serde(default)]
    federated: Option<FederatedSessionContext>,
}
#[derive(Debug, Deserialize)]
struct CreateAccountRequest {
    id: String,
    email: String,
    password: String,
    role_id: String,
}
#[derive(Debug, Deserialize)]
struct TotpEnrollmentRequest {
    account_id: String,
}
#[derive(Debug, Deserialize)]
struct UpdateAccountRequest {
    account_id: String,
    #[serde(default)]
    disabled: Option<bool>,
    #[serde(default)]
    password: Option<String>,
    #[serde(default)]
    role_id: Option<String>,
    #[serde(default)]
    clear_totp: bool,
}
#[derive(Debug, Deserialize)]
struct LoginRequest {
    email: String,
    password: String,
    #[serde(default)]
    totp_code: Option<String>,
    #[serde(default)]
    recovery_code: Option<String>,
}
#[derive(Debug, Deserialize)]
struct RecoveryCodesRequest {
    account_id: String,
}
#[derive(Debug, Deserialize)]
struct LogoutRequest {
    #[serde(default)]
    session_id: Option<String>,
}
#[derive(Debug, Deserialize)]
struct ScimListQuery {
    #[serde(default)]
    filter: Option<String>,
    #[serde(rename = "startIndex", default)]
    start_index: Option<usize>,
    #[serde(default)]
    count: Option<usize>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScimRoleValue {
    value: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScimUserInput {
    #[serde(default)]
    schemas: Vec<String>,
    #[serde(rename = "userName")]
    user_name: String,
    #[serde(rename = "externalId", default)]
    external_id: Option<String>,
    #[serde(default = "default_true")]
    active: bool,
    #[serde(default)]
    roles: Vec<ScimRoleValue>,
}
fn default_true() -> bool {
    true
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScimGroupRecord {
    id: String,
    display_name: String,
    #[serde(default)]
    external_id: Option<String>,
    #[serde(default)]
    members: Vec<String>,
    created_at_unix_ms: u128,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScimMemberValue {
    value: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScimGroupInput {
    #[serde(default)]
    schemas: Vec<String>,
    #[serde(rename = "displayName")]
    display_name: String,
    #[serde(rename = "externalId", default)]
    external_id: Option<String>,
    #[serde(default)]
    members: Vec<ScimMemberValue>,
}
#[derive(Debug, Clone, Deserialize)]
struct ScimPatchRequest {
    #[serde(default)]
    schemas: Vec<String>,
    #[serde(rename = "Operations")]
    operations: Vec<ScimPatchOperation>,
}
#[derive(Debug, Clone, Deserialize)]
struct ScimPatchOperation {
    op: String,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    value: Value,
}
#[derive(Debug, Clone, Deserialize)]
struct ScimBulkRequest {
    #[serde(default)]
    schemas: Vec<String>,
    #[serde(rename = "failOnErrors", default)]
    fail_on_errors: Option<u32>,
    #[serde(rename = "Operations")]
    operations: Vec<ScimBulkOperation>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScimBulkOperation {
    method: String,
    path: String,
    #[serde(rename = "bulkId", default)]
    bulk_id: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    data: Value,
}
#[derive(Debug, Deserialize)]
struct ScimReconcileRequest {
    #[serde(default)]
    repair: bool,
}
#[derive(Debug, Deserialize)]
struct OidcBeginRequest {
    provider_id: String,
}
#[derive(Debug, Deserialize)]
struct OidcCallbackQuery {
    state: String,
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    error_description: Option<String>,
}
#[derive(Debug, Deserialize)]
struct OidcLinkRequest {
    account_id: String,
    provider_id: String,
    subject: String,
}
#[derive(Debug, Deserialize)]
struct FederationUnlinkRequest {
    account_id: String,
    provider_id: String,
    subject: String,
}
#[derive(Debug, Deserialize)]
struct FederatedLogoutRequest {
    #[serde(default)]
    session_id: Option<String>,
}
#[derive(Debug, Deserialize)]
struct SamlBeginRequest {
    provider_id: String,
}
#[derive(Debug, Deserialize)]
struct SamlLinkRequest {
    account_id: String,
    provider_id: String,
    subject: String,
}
#[derive(Debug, Deserialize)]
struct SamlAcsForm {
    #[serde(rename = "SAMLResponse")]
    saml_response: String,
    #[serde(rename = "RelayState")]
    relay_state: String,
}
#[derive(Debug, Clone)]
struct PendingOidcTransaction {
    provider_id: String,
    transaction: vsn_auth::OidcPkceTransaction,
}
struct PendingPasskeyRegistration {
    account_id: String,
    state: PasskeyRegistration,
    expires_at_unix_ms: u128,
}
struct PendingPasskeyAuthentication {
    account_id: String,
    state: PasskeyAuthentication,
    expires_at_unix_ms: u128,
}
#[derive(Debug, Deserialize)]
struct PasskeyRegisterFinishRequest {
    transaction_id: String,
    credential: RegisterPublicKeyCredential,
}
#[derive(Debug, Deserialize)]
struct PasskeyLoginBeginRequest {
    email: String,
}
#[derive(Debug, Deserialize)]
struct PasskeyLoginFinishRequest {
    transaction_id: String,
    credential: PublicKeyCredential,
}
#[derive(Debug, Deserialize)]
struct PasskeyOwnerQuery {
    kind: String,
}
#[derive(Debug, Deserialize)]
struct TeamSecretSetRequest {
    name: String,
    value: String,
}
#[derive(Debug, Deserialize)]
struct TeamVaultRotateRequest {
    new_key_id: String,
    confirm: bool,
}
#[derive(Debug, Serialize)]
struct TeamSecretMetadata {
    name: String,
    key_id: String,
    created_by: String,
    updated_at_unix_ms: u128,
}
#[derive(Debug, Clone)]
struct TeamVaultKeyRing {
    keys: BTreeMap<String, [u8; 32]>,
    initial_active: Option<String>,
}

#[tokio::main]
async fn main() {
    if std::env::args().any(|arg| arg == "--generate-key") {
        let pair = vsn_remote::generate_control_plane_keypair();
        println!(
            "{}",
            serde_json::to_string_pretty(&pair).expect("serialize keypair")
        );
        return;
    }
    let admin_token =
        std::env::var("VSN_CONTROL_ADMIN_TOKEN").unwrap_or_else(|_| random_id("admin"));
    let private_key = std::env::var("VSN_CONTROL_PRIVATE_KEY_B64")
        .unwrap_or_else(|_| vsn_remote::generate_control_plane_keypair().private_key);
    let public_key =
        vsn_remote::control_plane_public_key(&private_key).expect("valid control plane key");
    let state_path = PathBuf::from(
        std::env::var("VSN_CONTROL_STATE_PATH")
            .unwrap_or_else(|_| "cloud/control-plane/data/state.db".into()),
    );
    let auth_encryption_key = load_auth_encryption_key()
        .unwrap_or_else(|e| panic!("control plane auth key configuration failed: {e}"));
    let team_vault_keys = load_team_vault_keyring()
        .unwrap_or_else(|e| panic!("control plane team-vault keyring configuration failed: {e}"));
    let webauthn = load_webauthn()
        .unwrap_or_else(|e| panic!("control plane WebAuthn configuration failed: {e}"));
    let state_postgres = load_postgres_state_store().unwrap_or_else(|e| {
        panic!("control plane PostgreSQL state-store configuration failed: {e}")
    });
    let (stored, state_generation) = if let Some(store) = state_postgres.as_ref() {
        load_persistent_state_postgres(store, &state_path)
            .unwrap_or_else(|e| panic!("control plane PostgreSQL state load failed: {e}"))
    } else {
        load_persistent_state(&state_path).unwrap_or_else(|e| {
            panic!(
                "control plane state load failed at {}: {e}",
                state_path.display()
            )
        })
    };
    let bind = std::env::var("VSN_CONTROL_BIND").unwrap_or_else(|_| "127.0.0.1:9070".into());
    let instance_id = std::env::var("VSN_CONTROL_INSTANCE_ID")
        .ok()
        .filter(|v| safe_identifier(v))
        .unwrap_or_else(|| random_id("cp"));
    let public_endpoint = std::env::var("VSN_CONTROL_PUBLIC_ENDPOINT")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| format!("local://{instance_id}"));
    if !is_loopback_bind(&bind) && !public_endpoint.starts_with("https://") {
        panic!("VSN_CONTROL_PUBLIC_ENDPOINT=https://... is required when VSN_CONTROL_BIND is non-loopback");
    }
    let state = AppState {
        admin_token: Arc::new(admin_token.clone()),
        private_key: Arc::new(private_key),
        public_key: Arc::new(public_key.clone()),
        pairings: Arc::new(Mutex::new(stored.pairings)),
        devices: Arc::new(Mutex::new(stored.devices)),
        queues: Arc::new(Mutex::new(stored.queues)),
        deliveries: Arc::new(Mutex::new(stored.deliveries)),
        results: Arc::new(Mutex::new(stored.results)),
        roles: Arc::new(Mutex::new(stored.roles)),
        tokens: Arc::new(Mutex::new(stored.tokens)),
        poll_replay: Arc::new(Mutex::new(ReplayWindow::new(16_384))),
        result_replay: Arc::new(Mutex::new(ReplayWindow::new(16_384))),
        fleet_groups: Arc::new(Mutex::new(stored.fleet_groups)),
        environments: Arc::new(Mutex::new(stored.environments)),
        approvals: Arc::new(Mutex::new(stored.approvals)),
        approval_decision_lock: Arc::new(Mutex::new(())),
        central_audit: Arc::new(Mutex::new(stored.central_audit)),
        auth_policy: Arc::new(Mutex::new(stored.auth_policy)),
        accounts: Arc::new(Mutex::new(stored.accounts)),
        scim_groups: Arc::new(Mutex::new(stored.scim_groups)),
        sessions: Arc::new(Mutex::new(stored.sessions)),
        oidc_transactions: Arc::new(Mutex::new(HashMap::new())),
        saml_transactions: Arc::new(Mutex::new(HashMap::new())),
        webauthn: Arc::new(webauthn),
        passkey_registrations: Arc::new(Mutex::new(HashMap::new())),
        passkey_authentications: Arc::new(Mutex::new(HashMap::new())),
        auth_encryption_key: Arc::new(auth_encryption_key),
        team_vault_keys: Arc::new(team_vault_keys),
        rate_limits: Arc::new(Mutex::new(HashMap::new())),
        state_path: Arc::new(state_path),
        state_postgres: Arc::new(state_postgres),
        persist_lock: Arc::new(Mutex::new(())),
        state_generation: Arc::new(Mutex::new(state_generation)),
        agent_stream_peers: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        stream_relays: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        remote_stream_homes: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        instance_id: Arc::new(instance_id),
        public_endpoint: Arc::new(public_endpoint),
        started_at_unix_ms: vsn_remote::now_ms(),
    };
    if state.state_postgres.is_some() {
        if let Err(e) = backfill_shared_sessions_once(&state) {
            panic!("control plane shared-session migration failed: {e:?}");
        }
        if let Err(e) = backfill_shared_auth_once(&state) {
            panic!("control plane shared-auth migration failed: {e:?}");
        }
        if let Err(e) = backfill_shared_iam_fleet_once(&state) {
            panic!("control plane shared-IAM/fleet migration failed: {e:?}");
        }
        if let Err(e) = refresh_shared_auth_state(&state) {
            panic!("control plane shared-auth refresh failed: {e:?}");
        }
        if let Err(e) = refresh_shared_iam_fleet_state(&state) {
            panic!("control plane shared-IAM/fleet refresh failed: {e:?}");
        }
        let cluster_state = state.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(std::time::Duration::from_secs(10));
            loop {
                tick.tick().await;
                if let Some(store) = cluster_state.state_postgres.as_ref() {
                    let _ = store.heartbeat_instance(
                        cluster_state.instance_id.as_str(),
                        cluster_state.public_endpoint.as_str(),
                        30_000,
                    );
                }
            }
        });
        let bus_state = state.clone();
        tokio::spawn(async move {
            run_cluster_stream_bus(bus_state).await;
        });
    }
    {
        let cleanup_state = state.clone();
        tokio::spawn(async move {
            run_stream_relay_cleanup(cleanup_state).await;
        });
    }
    let app = Router::new()
        .route("/health", get(health))
        .route("/ready", get(readiness))
        .route("/v1/admin/ops", get(ops_status))
        .route("/v1/admin/control/validate", get(validate_control_plane))
        .route("/v1/admin/iam/validate", get(validate_iam))
        .route("/v1/admin/security/validate", get(validate_security))
        .route(
            "/v1/admin/vault/secrets",
            get(team_vault_list).post(team_vault_set),
        )
        .route(
            "/v1/admin/vault/secrets/{name}",
            get(team_vault_reveal).delete(team_vault_delete),
        )
        .route("/v1/admin/vault/rotate", post(team_vault_rotate))
        .route("/v1/admin/pairings", post(create_pairing))
        .route(
            "/v1/admin/commands",
            post(queue_command).get(list_deliveries),
        )
        .route("/v1/admin/devices", get(list_devices))
        .route("/v1/admin/results", get(list_results))
        .route("/v1/admin/iam/roles", get(list_roles).post(create_role))
        .route("/v1/admin/iam/tokens", get(list_tokens).post(create_token))
        .route("/v1/admin/iam/tokens/revoke", post(revoke_token))
        .route("/v1/admin/fleet", get(fleet_overview))
        .route("/v1/admin/fleet/groups", post(upsert_fleet_group))
        .route(
            "/v1/admin/fleet/groups/{id}",
            axum::routing::delete(delete_fleet_group),
        )
        .route("/v1/admin/fleet/validate", get(validate_fleet))
        .route("/v1/admin/fleet/devices", post(update_device_fleet))
        .route(
            "/v1/admin/environments",
            get(list_environments).post(upsert_environment),
        )
        .route(
            "/v1/admin/environments/{id}",
            axum::routing::delete(delete_environment),
        )
        .route("/v1/admin/approvals", get(list_approvals))
        .route("/v1/admin/approvals/approve", post(approve_command))
        .route("/v1/admin/approvals/reject", post(reject_command))
        .route("/v1/admin/audit", get(list_central_audit))
        .route("/v1/admin/cluster", get(cluster_status))
        .route("/v1/admin/gateway/validate", get(validate_gateway))
        .route(
            "/v1/admin/auth/policy",
            get(get_auth_policy).post(set_auth_policy),
        )
        .route(
            "/v1/admin/auth/federation/validate",
            get(validate_federation),
        )
        .route(
            "/v1/admin/auth/accounts",
            get(list_accounts).post(create_account),
        )
        .route(
            "/scim/v2/Users",
            get(scim_list_users).post(scim_create_user),
        )
        .route(
            "/scim/v2/Users/{id}",
            get(scim_get_user)
                .put(scim_replace_user)
                .patch(scim_patch_user)
                .delete(scim_delete_user),
        )
        .route(
            "/scim/v2/Groups",
            get(scim_list_groups).post(scim_create_group),
        )
        .route(
            "/scim/v2/Groups/{id}",
            get(scim_get_group)
                .put(scim_replace_group)
                .patch(scim_patch_group)
                .delete(scim_delete_group),
        )
        .route(
            "/scim/v2/ServiceProviderConfig",
            get(scim_service_provider_config),
        )
        .route("/scim/v2/Bulk", post(scim_bulk))
        .route("/v1/admin/scim/reconcile", post(scim_reconcile))
        .route("/v1/admin/auth/totp-enroll", post(enroll_account_totp))
        .route(
            "/v1/admin/auth/recovery-codes",
            post(regenerate_account_recovery_codes),
        )
        .route("/v1/admin/auth/accounts/update", post(update_account))
        .route("/v1/auth/oidc/begin", post(oidc_begin))
        .route("/v1/auth/oidc/callback", get(oidc_callback))
        .route("/v1/admin/auth/oidc/link", post(link_oidc_identity))
        .route("/v1/admin/auth/oidc/unlink", post(unlink_oidc_identity))
        .route("/v1/auth/saml/begin", post(saml_begin))
        .route("/v1/auth/saml/acs", post(saml_acs))
        .route("/v1/admin/auth/saml/link", post(link_saml_identity))
        .route("/v1/admin/auth/saml/unlink", post(unlink_saml_identity))
        .route("/v1/auth/federated/logout", post(federated_logout))
        .route(
            "/v1/auth/passkey/register/begin",
            post(passkey_register_begin),
        )
        .route(
            "/v1/auth/passkey/register/finish",
            post(passkey_register_finish),
        )
        .route("/v1/auth/passkey/login/begin", post(passkey_login_begin))
        .route("/v1/auth/passkey/login/finish", post(passkey_login_finish))
        .route(
            "/v1/auth/passkey/owner/{transaction_id}",
            get(passkey_owner_lookup),
        )
        .route("/v1/auth/login", post(account_login))
        .route("/v1/auth/me", get(account_me))
        .route("/v1/auth/logout", post(account_logout))
        .route("/v1/devices/enroll", post(enroll_device))
        .route("/v1/agent/ws", get(agent_gateway))
        .route("/v1/agent/streams/ws", get(agent_stream_gateway))
        .route("/v1/streams/ws", get(browser_stream_gateway))
        .route("/v1/agent/poll", post(agent_poll))
        .route("/v1/agent/result", post(agent_result))
        .route("/v1/agent/audit", post(agent_audit))
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024))
        .with_state(state)
        .fallback_service(
            ServeDir::new(
                std::env::var("VSN_DASHBOARD_DIR")
                    .unwrap_or_else(|_| "cloud/dashboard/dist".into()),
            )
            .append_index_html_on_directories(true),
        );
    eprintln!("vsn-control-plane {VERSION} listening={bind}");
    eprintln!("control_plane_public_key={public_key}");
    if std::env::var("VSN_CONTROL_ADMIN_TOKEN").is_err() {
        eprintln!("development_admin_token={admin_token}");
        eprintln!("warning=ephemeral bootstrap admin token; set VSN_CONTROL_ADMIN_TOKEN and create scoped API tokens");
    }
    let listener = tokio::net::TcpListener::bind(&bind)
        .await
        .expect("bind control plane");
    axum::serve(listener, app)
        .await
        .expect("control plane server");
}

async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        ok: true,
        version: VERSION,
        public_key: (*state.public_key).clone(),
    })
}
async fn readiness(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let now = vsn_remote::now_ms();
    let mut shared_instances = 0usize;
    if let Some(store) = state.state_postgres.as_ref() {
        store
            .heartbeat_instance(
                state.instance_id.as_str(),
                state.public_endpoint.as_str(),
                30_000,
            )
            .map_err(|e| {
                api_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    &format!("shared state heartbeat failed: {e}"),
                )
            })?;
        let instances = store.live_instances().map_err(|e| {
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!("shared state readiness failed: {e}"),
            )
        })?;
        shared_instances = instances.len();
        if !instances
            .iter()
            .any(|i| i.instance_id == state.instance_id.as_str())
        {
            return Err(api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "current Control Plane instance is absent from shared cluster registry",
            ));
        }
    }
    Ok(Json(
        json!({"ready":true,"version":VERSION,"instance_id":state.instance_id.as_str(),"shared_postgres":state.state_postgres.is_some(),"shared_instances":shared_instances,"uptime_ms":now.saturating_sub(state.started_at_unix_ms)}),
    ))
}
async fn ops_status(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    require_permission(&state, &headers, "control.audit.view")?;
    let now = vsn_remote::now_ms();
    let devices = all_device_records(&state)?.len();
    let accounts = state.accounts.lock().map_err(lock_error)?.len();
    let sessions = state
        .sessions
        .lock()
        .map_err(lock_error)?
        .values()
        .filter(|s| !s.revoked && s.expires_at_unix_ms >= now)
        .count();
    let approvals = if let Some(store) = state.state_postgres.as_ref() {
        store
            .recent_approvals(1000)
            .map_err(|e| {
                api_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    &format!("shared approval status failed: {e}"),
                )
            })?
            .into_iter()
            .filter(|a| a.state == "pending")
            .count()
    } else {
        state
            .approvals
            .lock()
            .map_err(lock_error)?
            .values()
            .filter(|a| a.state == ApprovalState::Pending)
            .count()
    };
    let local_streams = state.stream_relays.lock().await.len();
    let (local_bus_depth, instances) = if let Some(store) = state.state_postgres.as_ref() {
        (
            store.bus_depth(state.instance_id.as_str()).map_err(|e| {
                api_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    &format!("cluster bus status failed: {e}"),
                )
            })?,
            store
                .live_instances()
                .map_err(|e| {
                    api_error(
                        StatusCode::SERVICE_UNAVAILABLE,
                        &format!("cluster instance status failed: {e}"),
                    )
                })?
                .len(),
        )
    } else {
        (0, 1)
    };
    let slo_p95_ms = std::env::var("VSN_SLO_CONTROL_P95_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(750)
        .clamp(50, 60_000);
    let slo_error_bps = std::env::var("VSN_SLO_ERROR_RATE_BPS")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(100)
        .clamp(1, 10_000);
    let team_vault_secrets = if let Some(store) = state.state_postgres.as_ref() {
        store.list_team_secrets().map(|v| v.len()).unwrap_or(0)
    } else {
        0
    };
    Ok(Json(
        json!({"version":VERSION,"instance_id":state.instance_id.as_str(),"uptime_ms":now.saturating_sub(state.started_at_unix_ms),"shared_postgres":state.state_postgres.is_some(),"cluster_instances":instances,"devices":devices,"active_sessions":sessions,"accounts":accounts,"pending_approvals":approvals,"local_stream_relays":local_streams,"local_bus_depth":local_bus_depth,"team_vault":{"configured":!state.team_vault_keys.keys.is_empty(),"secret_count":team_vault_secrets,"loaded_key_ids":state.team_vault_keys.keys.keys().cloned().collect::<Vec<_>>(),"active_key_id":state.state_postgres.as_ref().as_ref().and_then(|s|s.team_vault_active_key().ok()).flatten().or_else(||state.team_vault_keys.initial_active.clone())},"slo_targets":{"control_p95_ms":slo_p95_ms,"error_rate_basis_points":slo_error_bps},"note":"This endpoint exposes bounded operational state and configured SLO targets; latency/error observations are collected by external probes/telemetry rather than fabricated in-process metrics."}),
    ))
}

async fn validate_control_plane(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    require_permission(&state, &headers, "control.audit.view")?;
    refresh_shared_auth_state(&state)?;
    refresh_shared_iam_fleet_state(&state)?;
    let mut issues = Vec::new();
    let roles = state.roles.lock().map_err(lock_error)?.clone();
    let accounts = state.accounts.lock().map_err(lock_error)?.clone();
    let sessions = state.sessions.lock().map_err(lock_error)?.clone();
    let tokens = state.tokens.lock().map_err(lock_error)?.clone();
    for account in accounts.values() {
        if !roles.contains_key(&account.role_id) {
            issues.push(json!({"severity":"error","kind":"account_missing_role","account_id":account.id,"role_id":account.role_id}));
        }
    }
    for session in sessions.values() {
        if !accounts.contains_key(&session.account_id) {
            issues.push(json!({"severity":"error","kind":"session_missing_account","session_id":session.id,"account_id":session.account_id}));
        }
    }
    for token in tokens.values() {
        if !roles.contains_key(&token.role_id) {
            issues.push(json!({"severity":"error","kind":"token_missing_role","token_id":token.id,"role_id":token.role_id}));
        }
    }
    let devices = all_device_records(&state)?;
    let device_ids = devices
        .iter()
        .map(|d| d.device_id.clone())
        .collect::<HashSet<_>>();
    let groups = state.fleet_groups.lock().map_err(lock_error)?.clone();
    let group_ids = groups.keys().cloned().collect::<HashSet<_>>();
    let envs = state.environments.lock().map_err(lock_error)?.clone();
    for d in &devices {
        for g in &d.groups {
            if !group_ids.contains(g) {
                issues.push(json!({"severity":"error","kind":"device_missing_group","device_id":d.device_id,"group_id":g}));
            }
        }
    }
    for g in groups.values() {
        for d in &g.device_ids {
            if !device_ids.contains(d) {
                issues.push(json!({"severity":"error","kind":"group_missing_device","group_id":g.id,"device_id":d}));
            }
        }
    }
    for env in envs.values() {
        for (role, device) in &env.bindings {
            if !device_ids.contains(device) {
                issues.push(json!({"severity":"error","kind":"environment_missing_device","environment_id":env.id,"role":role,"device_id":device}));
            }
        }
    }
    if let Err(e) =
        vsn_auth::validate_policy(&state.auth_policy.lock().map_err(lock_error)?.clone())
    {
        issues
            .push(json!({"severity":"error","kind":"auth_policy_invalid","detail":e.to_string()}));
    }
    let approvals = if let Some(store) = state.state_postgres.as_ref() {
        store
            .recent_approvals(1000)
            .map_err(|e| {
                api_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    &format!("shared approval validation failed: {e}"),
                )
            })?
            .into_iter()
            .filter_map(|r| serde_json::from_str::<ApprovalRecord>(&r.payload).ok())
            .collect::<Vec<_>>()
    } else {
        state
            .approvals
            .lock()
            .map_err(lock_error)?
            .values()
            .cloned()
            .collect::<Vec<_>>()
    };
    for approval in approvals {
        if !device_ids.contains(&approval.request.device_id) {
            issues.push(json!({"severity":"error","kind":"approval_missing_device","approval_id":approval.id,"device_id":approval.request.device_id}));
        }
        if validate_permission_string(&approval.request.permission).is_err() {
            issues.push(json!({"severity":"error","kind":"approval_invalid_permission","approval_id":approval.id,"permission":approval.request.permission}));
        }
    }
    let mut cluster_instances = 1usize;
    if let Some(store) = state.state_postgres.as_ref() {
        let live = store.live_instances().map_err(|e| {
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!("shared cluster validation failed: {e}"),
            )
        })?;
        cluster_instances = live.len();
        if !live
            .iter()
            .any(|i| i.instance_id == state.instance_id.as_str())
        {
            issues.push(json!({"severity":"error","kind":"current_instance_missing_from_cluster","instance_id":state.instance_id.as_str()}));
        }
        for secret in store.list_team_secrets().map_err(|e| {
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!("team Vault validation failed: {e}"),
            )
        })? {
            if !state.team_vault_keys.keys.contains_key(&secret.key_id) {
                issues.push(json!({"severity":"error","kind":"team_vault_key_unavailable","secret":secret.name,"key_id":secret.key_id}));
            }
        }
    }
    Ok(Json(
        json!({"ok":issues.is_empty(),"issues":issues,"counts":{"roles":roles.len(),"accounts":accounts.len(),"sessions":sessions.len(),"tokens":tokens.len(),"devices":devices.len(),"groups":groups.len(),"environments":envs.len(),"cluster_instances":cluster_instances},"shared_postgres":state.state_postgres.is_some()}),
    ))
}

async fn validate_iam(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let principal = require_permission(&state, &headers, "control.iam.manage")?;
    refresh_shared_auth_state(&state)?;
    refresh_shared_iam_fleet_state(&state)?;
    let roles = state.roles.lock().map_err(lock_error)?.clone();
    let accounts = state.accounts.lock().map_err(lock_error)?.clone();
    let tokens = state.tokens.lock().map_err(lock_error)?.clone();
    let mut issues = Vec::new();
    for role in roles.values() {
        if role.permissions.len() > 256 {
            issues.push(json!({"kind":"role_permission_limit","role_id":role.id}));
        }
        for permission in &role.permissions {
            if validate_permission_string(permission).is_err() {
                issues.push(json!({"kind":"invalid_role_permission","role_id":role.id,"permission":permission}));
            }
        }
    }
    for account in accounts.values() {
        if !roles.contains_key(&account.role_id) {
            issues.push(json!({"kind":"account_missing_role","account_id":account.id,"role_id":account.role_id}));
        }
    }
    for token in tokens.values() {
        if !roles.contains_key(&token.role_id) {
            issues.push(
                json!({"kind":"token_missing_role","token_id":token.id,"role_id":token.role_id}),
            );
        }
    }
    Ok(Json(
        json!({"ok":issues.is_empty(),"issues":issues,"roles":roles.len(),"accounts":accounts.len(),"tokens":tokens.len(),"checked_by":principal.id,"delegation":"role/token creation remains bounded by the acting principal permission set"}),
    ))
}

async fn validate_security(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    require_permission(&state, &headers, "control.audit.view")?;
    let mut issues = Vec::new();
    let mut verified = 0usize;
    if let Some(store) = state.state_postgres.as_ref() {
        let mut rows = store.recent_audit(5000).map_err(|e| {
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!("shared audit validation failed: {e}"),
            )
        })?;
        rows.sort_by_key(|r| r.seq);
        let mut last = HashMap::<String, String>::new();
        for row in rows {
            let event: vsn_audit::AuditEvent = serde_json::from_str(&row.payload).map_err(|e| {
                api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("shared audit payload invalid: {e}"),
                )
            })?;
            if let Err(e) = vsn_audit::verify_event(&event) {
                issues.push(json!({"kind":"invalid_audit_signature","event_id":event.event_id,"detail":e.to_string()}));
                continue;
            }
            if let Some(prev) = last.get(&event.device_id) {
                if event.previous_hash != *prev {
                    issues.push(json!({"kind":"audit_chain_discontinuity","device_id":event.device_id,"event_id":event.event_id}));
                }
            }
            last.insert(event.device_id.clone(), event.event_hash.clone());
            verified += 1;
        }
    } else {
        for event in state.central_audit.lock().map_err(lock_error)?.iter() {
            if let Err(e) = vsn_audit::verify_event(event) {
                issues.push(json!({"kind":"invalid_audit_signature","event_id":event.event_id,"detail":e.to_string()}));
            } else {
                verified += 1;
            }
        }
    }
    if let Err(e) =
        vsn_auth::validate_policy(&state.auth_policy.lock().map_err(lock_error)?.clone())
    {
        issues.push(json!({"kind":"invalid_auth_policy","detail":e.to_string()}));
    }
    Ok(Json(
        json!({"ok":issues.is_empty(),"issues":issues,"verified_audit_events":verified,"shared_postgres":state.state_postgres.is_some(),"security_boundary":{"signed_remote_commands":true,"approval_gates":true,"team_vault_encrypted":!state.team_vault_keys.keys.is_empty(),"external_certification":"P30"}}),
    ))
}

async fn team_vault_list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    require_permission(&state, &headers, "control.vault.use")?;
    let store = state.state_postgres.as_ref().as_ref().ok_or_else(|| {
        api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "team Vault requires shared PostgreSQL state",
        )
    })?;
    let secrets = store
        .list_team_secrets()
        .map_err(|e| {
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!("team Vault list failed: {e}"),
            )
        })?
        .into_iter()
        .map(|s| TeamSecretMetadata {
            name: s.name,
            key_id: s.key_id,
            created_by: s.created_by,
            updated_at_unix_ms: s.updated_at_unix_ms,
        })
        .collect::<Vec<_>>();
    let active = store
        .team_vault_active_key()
        .map_err(|e| {
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!("team Vault metadata failed: {e}"),
            )
        })?
        .or_else(|| state.team_vault_keys.initial_active.clone());
    Ok(Json(
        json!({"secrets":secrets,"shared":true,"active_key_id":active,"loaded_key_ids":state.team_vault_keys.keys.keys().cloned().collect::<Vec<_>>()}),
    ))
}
async fn team_vault_set(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<TeamSecretSetRequest>,
) -> Result<Json<Value>, ApiError> {
    let principal = require_permission(&state, &headers, "control.vault.manage")?;
    validate_team_secret_name(&input.name)?;
    if input.value.len() > 1024 * 1024 {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "team secret exceeds 1 MiB",
        ));
    }
    let store = state.state_postgres.as_ref().as_ref().ok_or_else(|| {
        api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "team Vault requires shared PostgreSQL state",
        )
    })?;
    let (key_id, key) = team_vault_active_key(&state, store)?;
    let encrypted = encrypt_secret_bytes(&key, input.value.as_bytes())?;
    let record = vsn_control_store::SharedTeamSecretRecord {
        name: input.name.clone(),
        key_id: key_id.clone(),
        nonce_b64: encrypted.nonce_b64,
        ciphertext_b64: encrypted.ciphertext_b64,
        created_by: principal.id,
        updated_at_unix_ms: vsn_remote::now_ms(),
    };
    store.upsert_team_secret(&record).map_err(|e| {
        api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            &format!("team Vault write failed: {e}"),
        )
    })?;
    Ok(Json(
        json!({"ok":true,"name":record.name,"key_id":key_id,"updated_at_unix_ms":record.updated_at_unix_ms}),
    ))
}
async fn team_vault_reveal(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(name): AxumPath<String>,
) -> Result<Json<Value>, ApiError> {
    require_permission(&state, &headers, "control.vault.reveal")?;
    validate_team_secret_name(&name)?;
    let store = state.state_postgres.as_ref().as_ref().ok_or_else(|| {
        api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "team Vault requires shared PostgreSQL state",
        )
    })?;
    let record = store
        .team_secret(&name)
        .map_err(|e| {
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!("team Vault read failed: {e}"),
            )
        })?
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "team secret not found"))?;
    let key = *state
        .team_vault_keys
        .keys
        .get(&record.key_id)
        .ok_or_else(|| {
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!(
                    "team Vault key {} is not loaded on this Control Plane",
                    record.key_id
                ),
            )
        })?;
    let value = decrypt_secret_bytes(
        &key,
        &EncryptedAuthSecret {
            nonce_b64: record.nonce_b64,
            ciphertext_b64: record.ciphertext_b64,
        },
    )?;
    let value = String::from_utf8(value).map_err(|_| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "stored team secret is not UTF-8",
        )
    })?;
    Ok(Json(
        json!({"name":name,"key_id":record.key_id,"value":value,"updated_at_unix_ms":record.updated_at_unix_ms}),
    ))
}
async fn team_vault_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(name): AxumPath<String>,
) -> Result<Json<Value>, ApiError> {
    require_permission(&state, &headers, "control.vault.manage")?;
    validate_team_secret_name(&name)?;
    let store = state.state_postgres.as_ref().as_ref().ok_or_else(|| {
        api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "team Vault requires shared PostgreSQL state",
        )
    })?;
    let removed = store.delete_team_secret(&name).map_err(|e| {
        api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            &format!("team Vault delete failed: {e}"),
        )
    })?;
    Ok(Json(json!({"ok":true,"name":name,"removed":removed})))
}
async fn team_vault_rotate(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<TeamVaultRotateRequest>,
) -> Result<Json<Value>, ApiError> {
    let principal = require_permission(&state, &headers, "control.vault.manage")?;
    if !input.confirm {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "team Vault rotation requires confirm=true",
        ));
    }
    validate_team_vault_key_id(&input.new_key_id)?;
    let new_key = *state
        .team_vault_keys
        .keys
        .get(&input.new_key_id)
        .ok_or_else(|| {
            api_error(
                StatusCode::BAD_REQUEST,
                "new team Vault key id is not loaded on this Control Plane",
            )
        })?;
    let store = state.state_postgres.as_ref().as_ref().ok_or_else(|| {
        api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "team Vault rotation requires shared PostgreSQL state",
        )
    })?;
    let existing = store.list_team_secrets().map_err(|e| {
        api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            &format!("team Vault rotation read failed: {e}"),
        )
    })?;
    let now = vsn_remote::now_ms();
    let mut rotated = Vec::with_capacity(existing.len());
    for record in existing {
        let old_key = *state
            .team_vault_keys
            .keys
            .get(&record.key_id)
            .ok_or_else(|| {
                api_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    &format!(
                        "cannot rotate: required old team Vault key {} is not loaded",
                        record.key_id
                    ),
                )
            })?;
        let plain = decrypt_secret_bytes(
            &old_key,
            &EncryptedAuthSecret {
                nonce_b64: record.nonce_b64,
                ciphertext_b64: record.ciphertext_b64,
            },
        )?;
        let encrypted = encrypt_secret_bytes(&new_key, &plain)?;
        rotated.push(vsn_control_store::SharedTeamSecretRecord {
            name: record.name,
            key_id: input.new_key_id.clone(),
            nonce_b64: encrypted.nonce_b64,
            ciphertext_b64: encrypted.ciphertext_b64,
            created_by: record.created_by,
            updated_at_unix_ms: now,
        });
    }
    let count = store
        .rotate_team_secrets(&rotated, &input.new_key_id)
        .map_err(|e| {
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!("team Vault atomic rotation failed: {e}"),
            )
        })?;
    Ok(Json(
        json!({"ok":true,"active_key_id":input.new_key_id,"rotated_secrets":count,"rotated_by":principal.id,"updated_at_unix_ms":now}),
    ))
}
fn validate_team_secret_name(value: &str) -> Result<(), ApiError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
    {
        Err(api_error(
            StatusCode::BAD_REQUEST,
            "team secret name is invalid",
        ))
    } else {
        Ok(())
    }
}

async fn create_pairing(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<PairingResponse>, ApiError> {
    require_permission(&state, &headers, "control.pairings.create")?;
    let nonce = random_id("pair");
    let now = vsn_remote::now_ms();
    let expires = now + 10 * 60 * 1000;
    if let Some(store) = state.state_postgres.as_ref() {
        store.create_pairing(&nonce, expires).map_err(|e| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("shared pairing create failed: {e}"),
            )
        })?;
    }
    {
        let mut pairings = state.pairings.lock().map_err(lock_error)?;
        pairings.retain(|_, expiry| *expiry >= now);
        pairings.insert(nonce.clone(), expires);
    }
    if state.state_postgres.is_none() {
        persist_state(&state)?;
    }
    Ok(Json(PairingResponse {
        pairing_nonce: nonce,
        expires_at_unix_ms: expires,
        control_plane_public_key: (*state.public_key).clone(),
    }))
}

async fn enroll_device(
    State(state): State<AppState>,
    Json(enrollment): Json<DeviceEnrollmentV1>,
) -> Result<Json<Value>, ApiError> {
    vsn_remote::verify_device_enrollment(&enrollment).map_err(remote_error)?;
    let now = vsn_remote::now_ms();
    if let Some(store) = state.state_postgres.as_ref() {
        if !store
            .consume_pairing(&enrollment.pairing_nonce)
            .map_err(|e| {
                api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("shared pairing consume failed: {e}"),
                )
            })?
        {
            return Err(api_error(
                StatusCode::UNAUTHORIZED,
                "pairing nonce is unknown, expired, or already consumed",
            ));
        }
    } else {
        let mut pairings = state.pairings.lock().map_err(lock_error)?;
        let expiry = pairings.remove(&enrollment.pairing_nonce).ok_or_else(|| {
            api_error(
                StatusCode::UNAUTHORIZED,
                "pairing nonce is unknown or already consumed",
            )
        })?;
        if expiry < now {
            return Err(api_error(StatusCode::UNAUTHORIZED, "pairing nonce expired"));
        }
    }
    let record = DeviceRecord {
        device_id: enrollment.device_id.clone(),
        public_key: enrollment.public_key.clone(),
        display_name: enrollment.display_name,
        os: enrollment.os,
        enrolled_at_unix_ms: now,
        last_seen_unix_ms: now,
        labels: BTreeMap::new(),
        groups: Vec::new(),
    };
    if let Some(store) = state.state_postgres.as_ref() {
        store
            .upsert_device(&vsn_control_store::SharedDeviceRecord {
                device_id: record.device_id.clone(),
                public_key: record.public_key.clone(),
                display_name: record.display_name.clone(),
                os: record.os.clone(),
                enrolled_at_unix_ms: record.enrolled_at_unix_ms,
                last_seen_unix_ms: record.last_seen_unix_ms,
            })
            .map_err(|e| {
                api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("shared device registration failed: {e}"),
                )
            })?;
    }
    state
        .devices
        .lock()
        .map_err(lock_error)?
        .insert(record.device_id.clone(), record);
    if state.state_postgres.is_none() {
        persist_state(&state)?;
    }
    Ok(Json(
        json!({"ok":true,"device_id":enrollment.device_id,"control_plane_public_key":state.public_key.as_str()}),
    ))
}

fn enrolled_device_record(
    state: &AppState,
    device_id: &str,
) -> Result<Option<DeviceRecord>, ApiError> {
    if let Some(device) = state
        .devices
        .lock()
        .map_err(lock_error)?
        .get(device_id)
        .cloned()
    {
        return Ok(Some(device));
    }
    let Some(store) = state.state_postgres.as_ref() else {
        return Ok(None);
    };
    let shared = store.shared_device(device_id).map_err(|e| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("shared device lookup failed: {e}"),
        )
    })?;
    let Some(shared) = shared else {
        return Ok(None);
    };
    let record = DeviceRecord {
        device_id: shared.device_id,
        public_key: shared.public_key,
        display_name: shared.display_name,
        os: shared.os,
        enrolled_at_unix_ms: shared.enrolled_at_unix_ms,
        last_seen_unix_ms: shared.last_seen_unix_ms,
        labels: BTreeMap::new(),
        groups: Vec::new(),
    };
    state
        .devices
        .lock()
        .map_err(lock_error)?
        .insert(record.device_id.clone(), record.clone());
    Ok(Some(record))
}

fn all_device_records(state: &AppState) -> Result<Vec<DeviceRecord>, ApiError> {
    let mut local = state.devices.lock().map_err(lock_error)?.clone();
    if let Some(store) = state.state_postgres.as_ref() {
        for shared in store.shared_devices(5000).map_err(|e| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("shared device list failed: {e}"),
            )
        })? {
            local
                .entry(shared.device_id.clone())
                .and_modify(|d| {
                    d.public_key = shared.public_key.clone();
                    d.display_name = shared.display_name.clone();
                    d.os = shared.os.clone();
                    d.enrolled_at_unix_ms = shared.enrolled_at_unix_ms;
                    d.last_seen_unix_ms = shared.last_seen_unix_ms;
                })
                .or_insert(DeviceRecord {
                    device_id: shared.device_id,
                    public_key: shared.public_key,
                    display_name: shared.display_name,
                    os: shared.os,
                    enrolled_at_unix_ms: shared.enrolled_at_unix_ms,
                    last_seen_unix_ms: shared.last_seen_unix_ms,
                    labels: BTreeMap::new(),
                    groups: Vec::new(),
                });
        }
    }
    let mut out = local.into_values().collect::<Vec<_>>();
    out.sort_by(|a, b| b.last_seen_unix_ms.cmp(&a.last_seen_unix_ms));
    Ok(out)
}

fn process_agent_poll(
    state: &AppState,
    poll: AgentPollV1,
) -> Result<AgentPollResponseV1, ApiError> {
    vsn_remote::verify_agent_poll(&poll).map_err(remote_error)?;
    let now = vsn_remote::now_ms();
    let replay_key = format!("{}:{}", poll.device_id, poll.nonce);
    if !state
        .poll_replay
        .lock()
        .map_err(lock_error)?
        .insert(replay_key)
    {
        return Err(api_error(
            StatusCode::CONFLICT,
            "agent poll replay detected",
        ));
    }
    if let Some(store) = state.state_postgres.as_ref() {
        let shared = store
            .shared_device(&poll.device_id)
            .map_err(|e| {
                api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("shared device lookup failed: {e}"),
                )
            })?
            .ok_or_else(|| api_error(StatusCode::UNAUTHORIZED, "device is not enrolled"))?;
        if shared.public_key != poll.public_key {
            return Err(api_error(
                StatusCode::UNAUTHORIZED,
                "device public key mismatch",
            ));
        }
        store.touch_device(&poll.device_id, now).map_err(|e| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("shared device heartbeat failed: {e}"),
            )
        })?;
        {
            let mut devices = state.devices.lock().map_err(lock_error)?;
            devices
                .entry(shared.device_id.clone())
                .and_modify(|d| d.last_seen_unix_ms = now)
                .or_insert(DeviceRecord {
                    device_id: shared.device_id,
                    public_key: shared.public_key,
                    display_name: shared.display_name,
                    os: shared.os,
                    enrolled_at_unix_ms: shared.enrolled_at_unix_ms,
                    last_seen_unix_ms: now,
                    labels: BTreeMap::new(),
                    groups: Vec::new(),
                });
        }
        let leased = store
            .lease_command(
                &poll.device_id,
                state.instance_id.as_str(),
                DELIVERY_LEASE_MS as u64,
                MAX_DELIVERY_ATTEMPTS,
            )
            .map_err(|e| {
                api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("shared command lease failed: {e}"),
                )
            })?;
        let selected = if let Some(record) = leased {
            let command: RemoteCommandV1 = serde_json::from_str(&record.payload).map_err(|e| {
                api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("shared command payload is invalid: {e}"),
                )
            })?;
            if command.command_id != record.command_id || command.device_id != poll.device_id {
                return Err(api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "shared command identity mismatch",
                ));
            }
            {
                let mut deliveries = state.deliveries.lock().map_err(lock_error)?;
                deliveries.insert(
                    command.command_id.clone(),
                    DeliveryMeta {
                        state: DeliveryState::Inflight,
                        attempts: record.attempts,
                        leased_until_unix_ms: record.lease_until_unix_ms,
                        completed_at_unix_ms: record.completed_at_unix_ms,
                        last_error: record.last_error,
                    },
                );
            }
            Some(command)
        } else {
            None
        };
        return Ok(AgentPollResponseV1 {
            command: selected,
            server_time_unix_ms: now,
        });
    }
    {
        let mut devices = state.devices.lock().map_err(lock_error)?;
        let device = devices
            .get_mut(&poll.device_id)
            .ok_or_else(|| api_error(StatusCode::UNAUTHORIZED, "device is not enrolled"))?;
        if device.public_key != poll.public_key {
            return Err(api_error(
                StatusCode::UNAUTHORIZED,
                "device public key mismatch",
            ));
        }
        device.last_seen_unix_ms = now;
    }
    let mut selected = None;
    let mut changed = false;
    {
        let queues = state.queues.lock().map_err(lock_error)?;
        let mut deliveries = state.deliveries.lock().map_err(lock_error)?;
        if let Some(queue) = queues.get(&poll.device_id) {
            for command in queue.iter() {
                let meta = deliveries.entry(command.command_id.clone()).or_default();
                if command.expires_at_unix_ms < now {
                    if meta.state != DeliveryState::Completed {
                        meta.state = DeliveryState::Failed;
                        meta.last_error = Some("command expired before completion".into());
                        changed = true;
                    }
                    continue;
                }
                if matches!(meta.state, DeliveryState::Completed | DeliveryState::Failed) {
                    continue;
                }
                if meta.state == DeliveryState::Inflight
                    && meta.leased_until_unix_ms.unwrap_or(0) > now
                {
                    continue;
                }
                if meta.attempts >= MAX_DELIVERY_ATTEMPTS {
                    meta.state = DeliveryState::Failed;
                    meta.last_error = Some("delivery attempt limit reached".into());
                    changed = true;
                    continue;
                }
                meta.state = DeliveryState::Inflight;
                meta.attempts += 1;
                meta.leased_until_unix_ms = Some(now + DELIVERY_LEASE_MS);
                meta.last_error = None;
                selected = Some(command.clone());
                changed = true;
                break;
            }
        }
    }
    if changed {
        persist_state(state)?;
    }
    Ok(AgentPollResponseV1 {
        command: selected,
        server_time_unix_ms: now,
    })
}

fn process_agent_result(
    state: &AppState,
    result: AgentCommandResultV1,
) -> Result<(bool, bool), ApiError> {
    let key = if let Some(store) = state.state_postgres.as_ref() {
        store
            .shared_device(&result.device_id)
            .map_err(|e| {
                api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("shared device lookup failed: {e}"),
                )
            })?
            .map(|d| d.public_key)
            .ok_or_else(|| api_error(StatusCode::UNAUTHORIZED, "device is not enrolled"))?
    } else {
        let devices = state.devices.lock().map_err(lock_error)?;
        devices
            .get(&result.device_id)
            .map(|d| d.public_key.clone())
            .ok_or_else(|| api_error(StatusCode::UNAUTHORIZED, "device is not enrolled"))?
    };
    vsn_remote::verify_agent_result(&result, &key).map_err(remote_error)?;
    let replay_key = format!("{}:{}", result.device_id, result.nonce);
    if !state
        .result_replay
        .lock()
        .map_err(lock_error)?
        .insert(replay_key)
    {
        let already = state
            .results
            .lock()
            .map_err(lock_error)?
            .iter()
            .any(|existing| existing == &result);
        if already {
            return Ok((true, true));
        }
    }
    if let Some(store) = state.state_postgres.as_ref() {
        let record = store
            .command(&result.command_id)
            .map_err(|e| {
                api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("shared command lookup failed: {e}"),
                )
            })?
            .ok_or_else(|| {
                api_error(
                    StatusCode::CONFLICT,
                    "result does not match a shared command",
                )
            })?;
        if record.device_id != result.device_id {
            return Err(api_error(StatusCode::CONFLICT, "result device mismatch"));
        }
        let command: RemoteCommandV1 = serde_json::from_str(&record.payload).map_err(|e| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("shared command payload is invalid: {e}"),
            )
        })?;
        if command.session_id != result.session_id {
            return Err(api_error(StatusCode::CONFLICT, "result session mismatch"));
        }
        if record.state == "completed" {
            let same_session = state
                .results
                .lock()
                .map_err(lock_error)?
                .iter()
                .any(|existing| {
                    existing.device_id == result.device_id
                        && existing.command_id == result.command_id
                        && existing.session_id == result.session_id
                });
            return Ok((true, same_session));
        }
        if !matches!(record.state.as_str(), "queued" | "inflight") {
            return Err(api_error(
                StatusCode::CONFLICT,
                "shared command is not active",
            ));
        }
        let result_payload = serde_json::to_string(&result).map_err(|e| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("shared result serialization failed: {e}"),
            )
        })?;
        if !store
            .complete_command(&result.command_id, &result.device_id, &result_payload)
            .map_err(|e| {
                api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("shared command completion failed: {e}"),
                )
            })?
        {
            return Err(api_error(
                StatusCode::CONFLICT,
                "shared command completion was not accepted",
            ));
        }
        {
            let mut deliveries = state.deliveries.lock().map_err(lock_error)?;
            deliveries.insert(
                result.command_id.clone(),
                DeliveryMeta {
                    state: DeliveryState::Completed,
                    attempts: record.attempts,
                    leased_until_unix_ms: None,
                    completed_at_unix_ms: Some(vsn_remote::now_ms()),
                    last_error: None,
                },
            );
        }
        {
            let mut results = state.results.lock().map_err(lock_error)?;
            if results.len() >= 100 {
                results.remove(0);
            }
            results.push(result);
        }
        let _ = store.cleanup_commands(7 * 24 * 60 * 60 * 1000);
        return Ok((true, false));
    }
    let command = {
        let queues = state.queues.lock().map_err(lock_error)?;
        queues
            .get(&result.device_id)
            .and_then(|q| q.iter().find(|c| c.command_id == result.command_id))
            .cloned()
    };
    if command.is_none() {
        let completed = state
            .deliveries
            .lock()
            .map_err(lock_error)?
            .get(&result.command_id)
            .map(|m| m.state == DeliveryState::Completed)
            .unwrap_or(false);
        let same_session = state
            .results
            .lock()
            .map_err(lock_error)?
            .iter()
            .any(|existing| {
                existing.device_id == result.device_id
                    && existing.command_id == result.command_id
                    && existing.session_id == result.session_id
            });
        if completed && same_session {
            return Ok((true, true));
        }
        return Err(api_error(
            StatusCode::CONFLICT,
            "result does not match an active command",
        ));
    }
    let command = command.expect("checked above");
    if command.session_id != result.session_id {
        return Err(api_error(StatusCode::CONFLICT, "result session mismatch"));
    }
    {
        let mut deliveries = state.deliveries.lock().map_err(lock_error)?;
        let meta = deliveries.entry(result.command_id.clone()).or_default();
        meta.state = DeliveryState::Completed;
        meta.completed_at_unix_ms = Some(vsn_remote::now_ms());
        meta.leased_until_unix_ms = None;
        meta.last_error = None;
    }
    {
        let mut queues = state.queues.lock().map_err(lock_error)?;
        if let Some(q) = queues.get_mut(&result.device_id) {
            q.retain(|c| c.command_id != result.command_id);
        }
    }
    {
        let mut results = state.results.lock().map_err(lock_error)?;
        if results.len() >= 100 {
            results.remove(0);
        }
        results.push(result);
    }
    persist_state(state)?;
    Ok((true, false))
}

async fn agent_poll(
    State(state): State<AppState>,
    Json(poll): Json<AgentPollV1>,
) -> Result<Json<AgentPollResponseV1>, ApiError> {
    Ok(Json(process_agent_poll(&state, poll)?))
}
async fn agent_result(
    State(state): State<AppState>,
    Json(result): Json<AgentCommandResultV1>,
) -> Result<Json<Value>, ApiError> {
    let (ok, duplicate) = process_agent_result(&state, result)?;
    Ok(Json(json!({"ok":ok,"duplicate":duplicate})))
}

async fn agent_gateway(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.max_message_size(2 * 1024 * 1024)
        .on_upgrade(move |socket| handle_agent_gateway(socket, state))
}
async fn handle_agent_gateway(mut socket: WebSocket, state: AppState) {
    while let Some(message) = socket.recv().await {
        let message = match message {
            Ok(v) => v,
            Err(_) => break,
        };
        let bytes = match message {
            WsMessage::Text(v) => v.as_bytes().to_vec(),
            WsMessage::Binary(v) => v.to_vec(),
            WsMessage::Close(_) => break,
            WsMessage::Ping(_) | WsMessage::Pong(_) => continue,
        };
        if bytes.len() > 2 * 1024 * 1024 {
            let _ = send_gateway_response(
                &mut socket,
                &vsn_remote::AgentGatewayResponseV1::Error {
                    message: "gateway frame exceeds 2 MiB safety limit".into(),
                },
            )
            .await;
            break;
        }
        let request = match serde_json::from_slice::<vsn_remote::AgentGatewayRequestV1>(&bytes) {
            Ok(v) => v,
            Err(_) => {
                let _ = send_gateway_response(
                    &mut socket,
                    &vsn_remote::AgentGatewayResponseV1::Error {
                        message: "invalid gateway frame".into(),
                    },
                )
                .await;
                continue;
            }
        };
        let response = match request {
            vsn_remote::AgentGatewayRequestV1::Poll(poll) => match process_agent_poll(&state, poll)
            {
                Ok(v) => vsn_remote::AgentGatewayResponseV1::Poll(Box::new(v)),
                Err(e) => vsn_remote::AgentGatewayResponseV1::Error {
                    message: api_error_message(&e),
                },
            },
            vsn_remote::AgentGatewayRequestV1::Result(result) => {
                match process_agent_result(&state, result) {
                    Ok((ok, duplicate)) => {
                        vsn_remote::AgentGatewayResponseV1::Ack { ok, duplicate }
                    }
                    Err(e) => vsn_remote::AgentGatewayResponseV1::Error {
                        message: api_error_message(&e),
                    },
                }
            }
        };
        if send_gateway_response(&mut socket, &response).await.is_err() {
            break;
        }
    }
}
async fn send_gateway_response(
    socket: &mut WebSocket,
    response: &vsn_remote::AgentGatewayResponseV1,
) -> Result<(), ()> {
    let text = serde_json::to_string(response).map_err(|_| ())?;
    socket
        .send(WsMessage::Text(text.into()))
        .await
        .map_err(|_| ())
}

async fn agent_stream_gateway(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.max_message_size(2 * 1024 * 1024)
        .on_upgrade(move |socket| handle_agent_stream_gateway(socket, state))
}

async fn handle_agent_stream_gateway(mut socket: WebSocket, state: AppState) {
    let first = match tokio::time::timeout(std::time::Duration::from_secs(10), socket.recv()).await
    {
        Ok(Some(Ok(v))) => v,
        _ => return,
    };
    let hello = match parse_ws_json::<vsn_remote::AgentStreamClientMessageV1>(first) {
        Ok(vsn_remote::AgentStreamClientMessageV1::Hello(v)) => v,
        _ => {
            let _ = send_ws_json(
                &mut socket,
                &vsn_remote::AgentStreamServerMessageV1::Close {
                    relay_id: "connection".into(),
                    reason: Some("signed agent stream hello required".into()),
                },
            )
            .await;
            return;
        }
    };
    if vsn_remote::verify_agent_stream_hello(&hello).is_err() {
        return;
    }
    let enrolled = match enrolled_device_record(&state, &hello.device_id) {
        Ok(v) => v,
        Err(_) => None,
    };
    let Some(device) = enrolled else { return };
    if device.public_key != hello.public_key {
        return;
    }
    let connection_id = random_id("agentstream");
    let (tx, mut rx) = mpsc::channel::<vsn_remote::AgentStreamServerMessageV1>(128);
    state.agent_stream_peers.lock().await.insert(
        hello.device_id.clone(),
        AgentStreamPeer {
            connection_id: connection_id.clone(),
            tx,
        },
    );
    if let Some(store) = state.state_postgres.as_ref() {
        let _ = store.upsert_route(
            "agent_stream",
            &hello.device_id,
            state.instance_id.as_str(),
            120_000,
        );
    }
    let reopened =
        reopen_recoverable_relays(&state, &hello.device_id, state.instance_id.as_str()).await;
    if !reopened.is_empty() {
        eprintln!(
            "agent_stream_reopened={} device={}",
            reopened.len(),
            hello.device_id
        );
    }
    let device_id = hello.device_id.clone();
    let (mut sender, mut receiver) = socket.split();
    let mut heartbeat = tokio::time::interval(std::time::Duration::from_secs(20));
    loop {
        tokio::select! {
            inbound=receiver.next()=>{
                let Some(Ok(message))=inbound else{break};let parsed=match parse_ws_json::<vsn_remote::AgentStreamClientMessageV1>(message){Ok(v)=>v,Err(_)=>continue};
                if let Some(store)=state.state_postgres.as_ref(){let _=store.upsert_route("agent_stream",&device_id,state.instance_id.as_str(),120_000);}
                match parsed{
                    vsn_remote::AgentStreamClientMessageV1::Opened{relay_id,ok,resource_id,error,..}=>route_agent_stream_message(&state,&relay_id,vsn_remote::BrowserStreamServerMessageV1::Opened{relay_id:relay_id.clone(),ok,resource_id,error,resume_token:None,resumed:false,next_input_seq:0}).await,
                    vsn_remote::AgentStreamClientMessageV1::Output{relay_id,frame}=>{if frame.decoded_len().is_ok(){route_agent_stream_message(&state,&relay_id,vsn_remote::BrowserStreamServerMessageV1::Output{frame}).await;}},
                    vsn_remote::AgentStreamClientMessageV1::InputAck{relay_id,next_input_seq,committed_bytes,digest_sha256}=>route_agent_stream_message(&state,&relay_id,vsn_remote::BrowserStreamServerMessageV1::InputAck{next_input_seq,committed_bytes,digest_sha256}).await,
                    vsn_remote::AgentStreamClientMessageV1::Closed{relay_id,reason}=>{route_agent_stream_message(&state,&relay_id,vsn_remote::BrowserStreamServerMessageV1::Closed{reason:reason.clone()}).await;state.stream_relays.lock().await.remove(&relay_id);},
                    vsn_remote::AgentStreamClientMessageV1::Pong{timestamp_unix_ms}=>{let _=timestamp_unix_ms;},
                    vsn_remote::AgentStreamClientMessageV1::Error{relay_id,message}=>{if let Some(id)=relay_id{route_agent_stream_message(&state,&id,vsn_remote::BrowserStreamServerMessageV1::Error{message}).await;}},
                    vsn_remote::AgentStreamClientMessageV1::Hello(_)=>{},
                }
            },
            outbound=rx.recv()=>{
                let Some(message)=outbound else{break};let text=match serde_json::to_string(&message){Ok(v)=>v,Err(_)=>continue};if sender.send(WsMessage::Text(text.into())).await.is_err(){break;}
            },
            _=heartbeat.tick()=>{let ping=vsn_remote::AgentStreamServerMessageV1::Ping{timestamp_unix_ms:vsn_remote::now_ms()};let text=match serde_json::to_string(&ping){Ok(v)=>v,Err(_)=>continue};if sender.send(WsMessage::Text(text.into())).await.is_err(){break;}}
        }
    }
    {
        let mut peers = state.agent_stream_peers.lock().await;
        if peers.get(&device_id).map(|p| p.connection_id.as_str()) == Some(connection_id.as_str()) {
            peers.remove(&device_id);
        }
    }
    if let Some(store) = state.state_postgres.as_ref() {
        let _ = store.remove_route_if_owner("agent_stream", &device_id, state.instance_id.as_str());
    }
    let (now, to_close, to_notify) = {
        let now = vsn_remote::now_ms();
        let mut all = state.stream_relays.lock().await;
        let ids = all
            .iter()
            .filter(|(_, r)| {
                r.device_id == device_id && r.agent_instance_id == state.instance_id.as_str()
            })
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        let mut close = Vec::new();
        let mut notify = Vec::new();
        for id in ids {
            let recover = all
                .get(&id)
                .map(recoverable_after_agent_disconnect)
                .unwrap_or(false);
            if recover {
                if let Some(r) = all.get_mut(&id) {
                    r.agent_instance_id = "reconnecting".into();
                    r.last_activity_unix_ms = now;
                    r.detached_until_unix_ms = Some(now + STREAM_RELAY_RESUME_MS);
                    persist_shared_relay(&state, &id, r);
                    if let Some(tx) = r.browser_tx.clone() {
                        notify.push(tx);
                    }
                }
            } else if let Some(r) = all.remove(&id) {
                close.push((id, r));
            }
        }
        (now, close, notify)
    };
    let _ = now;
    for tx in to_notify {
        let _ = tx.try_send(vsn_remote::BrowserStreamServerMessageV1::Error {
            message:
                "Agent stream disconnected; recoverable resource is held for the resume window"
                    .into(),
        });
    }
    for (id, relay) in to_close {
        delete_shared_relay(&state, &id);
        if let Some(tx) = relay.browser_tx {
            let _ = tx.try_send(vsn_remote::BrowserStreamServerMessageV1::Closed {
                reason: Some(
                    "agent stream disconnected; terminal/non-recoverable resource closed fail-safe"
                        .into(),
                ),
            });
        }
    }
    let remote_relays = {
        let homes = state.remote_stream_homes.lock().await;
        homes
            .iter()
            .filter(|(_, r)| r.device_id == device_id)
            .map(|(id, h)| (id.clone(), h.clone()))
            .collect::<Vec<_>>()
    };
    for (relay_id, home) in remote_relays {
        let _ = publish_browser_bus(
            &state,
            &home.home_instance_id,
            &relay_id,
            vsn_remote::BrowserStreamServerMessageV1::Error {
                message:
                    "Agent stream disconnected; reconnect the browser relay after the Agent returns"
                        .into(),
            },
        );
    }
}

async fn browser_stream_gateway(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Response {
    let expected = state.public_endpoint.trim_end_matches('/');
    if expected.starts_with("https://") || expected.starts_with("http://") {
        let origin = headers
            .get("origin")
            .and_then(|v| v.to_str().ok())
            .map(str::trim)
            .unwrap_or("");
        if origin != expected {
            return (StatusCode::FORBIDDEN, "browser stream origin rejected").into_response();
        }
    }
    ws.max_message_size(2 * 1024 * 1024)
        .on_upgrade(move |socket| handle_browser_stream_gateway(socket, state))
}

async fn handle_browser_stream_gateway(mut socket: WebSocket, state: AppState) {
    let first = match tokio::time::timeout(std::time::Duration::from_secs(10), socket.recv()).await
    {
        Ok(Some(Ok(v))) => v,
        _ => return,
    };
    let (token, device_id, request, resume) =
        match parse_ws_json::<vsn_remote::BrowserStreamClientMessageV1>(first) {
            Ok(vsn_remote::BrowserStreamClientMessageV1::Hello {
                token,
                device_id,
                request,
                resume,
            }) => (token, device_id, request, resume),
            _ => {
                let _ = send_browser_stream(
                    &mut socket,
                    &vsn_remote::BrowserStreamServerMessageV1::Error {
                        message: "stream hello required".into(),
                    },
                )
                .await;
                return;
            }
        };
    let principal = match authenticate_token(&state, &token, true) {
        Ok(v) => v,
        Err(_) => {
            let _ = send_browser_stream(
                &mut socket,
                &vsn_remote::BrowserStreamServerMessageV1::Error {
                    message: "stream authentication failed".into(),
                },
            )
            .await;
            return;
        }
    };
    let permission = match relay_permission(&request.kind, &request.direction) {
        Ok(v) => v,
        Err(message) => {
            let _ = send_browser_stream(
                &mut socket,
                &vsn_remote::BrowserStreamServerMessageV1::Error { message },
            )
            .await;
            return;
        }
    };
    if !principal.allows(permission) {
        let _ = send_browser_stream(
            &mut socket,
            &vsn_remote::BrowserStreamServerMessageV1::Error {
                message: "stream permission denied".into(),
            },
        )
        .await;
        return;
    }
    if check_rate_limit(&state, &format!("stream:{}", principal.id), 60, 60_000).is_err() {
        let _ = send_browser_stream(
            &mut socket,
            &vsn_remote::BrowserStreamServerMessageV1::Error {
                message: "stream rate limit exceeded".into(),
            },
        )
        .await;
        return;
    }
    let now = vsn_remote::now_ms();
    let (browser_tx, mut browser_rx) = mpsc::channel(128);
    let mut replay_frames = Vec::new();
    if let Some(r) = resume.as_ref() {
        if ensure_shared_relay_loaded(&state, &r.relay_id)
            .await
            .is_err()
        {
            let _ = send_browser_stream(
                &mut socket,
                &vsn_remote::BrowserStreamServerMessageV1::Error {
                    message: "resume relay not found or expired".into(),
                },
            )
            .await;
            return;
        }
    }
    let (relay_id, agent_instance_id, resume_token, resumed, next_input_seq, needs_reopen) =
        if let Some(resume) = resume {
            let current_owner = if state
                .agent_stream_peers
                .lock()
                .await
                .contains_key(&device_id)
            {
                Some(state.instance_id.as_str().to_string())
            } else if let Some(store) = state.state_postgres.as_ref() {
                store.route_owner("agent_stream", &device_id).ok().flatten()
            } else {
                None
            };
            let mut relays = state.stream_relays.lock().await;
            let Some(relay) = relays.get_mut(&resume.relay_id) else {
                let _ = send_browser_stream(
                    &mut socket,
                    &vsn_remote::BrowserStreamServerMessageV1::Error {
                        message: "resume relay not found or expired".into(),
                    },
                )
                .await;
                return;
            };
            let token_hash = hash_token(&resume.resume_token);
            let detached_ok = relay
                .detached_until_unix_ms
                .map(|v| v >= now)
                .unwrap_or(false);
            if relay.device_id != device_id
                || relay.principal_id != principal.id
                || relay.permission != permission
                || relay.request != request
                || !detached_ok
                || relay.browser_tx.is_some()
                || !constant_time_eq(relay.resume_token_hash.as_bytes(), token_hash.as_bytes())
            {
                let _ = send_browser_stream(
                    &mut socket,
                    &vsn_remote::BrowserStreamServerMessageV1::Error {
                        message: "resume authorization rejected".into(),
                    },
                )
                .await;
                return;
            }
            let current_owner = match current_owner {
                Some(v) => v,
                None => {
                    let _ = send_browser_stream(
                        &mut socket,
                        &vsn_remote::BrowserStreamServerMessageV1::Error {
                            message: "device stream channel is offline".into(),
                        },
                    )
                    .await;
                    return;
                }
            };
            let needs_reopen = relay.agent_instance_id != current_owner;
            if needs_reopen && !recoverable_after_agent_disconnect(relay) {
                let _=send_browser_stream(&mut socket,&vsn_remote::BrowserStreamServerMessageV1::Error{message:"this resource cannot be reconstructed after Agent/Control Plane reconnect; open a new session".into()}).await;
                return;
            }
            let rotated = random_id("resume");
            relay.resume_token_hash = hash_token(&rotated);
            relay.pending_resume_token = None;
            relay.browser_tx = Some(browser_tx.clone());
            relay.detached_until_unix_ms = None;
            relay.last_activity_unix_ms = now;
            relay.next_input_seq = relay.acked_input_seq;
            relay.agent_instance_id = current_owner.clone();
            let last = resume.last_output_seq;
            replay_frames = relay
                .history
                .iter()
                .filter(|frame| last.map(|seq| frame.seq > seq).unwrap_or(true))
                .cloned()
                .collect();
            persist_shared_relay(&state, &resume.relay_id, relay);
            (
                resume.relay_id,
                current_owner,
                rotated,
                true,
                relay.acked_input_seq,
                needs_reopen,
            )
        } else {
            let local_peer = state
                .agent_stream_peers
                .lock()
                .await
                .get(&device_id)
                .cloned();
            let agent_instance_id = if local_peer.is_some() {
                state.instance_id.as_str().to_string()
            } else if let Some(store) = state.state_postgres.as_ref() {
                match store.route_owner("agent_stream", &device_id) {
                    Ok(Some(owner)) => owner,
                    _ => {
                        let _ = send_browser_stream(
                            &mut socket,
                            &vsn_remote::BrowserStreamServerMessageV1::Error {
                                message: "device stream channel is offline".into(),
                            },
                        )
                        .await;
                        return;
                    }
                }
            } else {
                let _ = send_browser_stream(
                    &mut socket,
                    &vsn_remote::BrowserStreamServerMessageV1::Error {
                        message: "device stream channel is offline".into(),
                    },
                )
                .await;
                return;
            };
            let relay_id = random_id("relay");
            let raw_resume = random_id("resume");
            {
                let mut relays = state.stream_relays.lock().await;
                relays.retain(|_, r| {
                    now.saturating_sub(r.last_activity_unix_ms) <= STREAM_RELAY_IDLE_MS
                });
                if relays.len() >= MAX_ACTIVE_STREAM_RELAYS {
                    let _ = send_browser_stream(
                        &mut socket,
                        &vsn_remote::BrowserStreamServerMessageV1::Error {
                            message: "stream relay capacity reached".into(),
                        },
                    )
                    .await;
                    return;
                }
                relays.insert(
                    relay_id.clone(),
                    StreamRelayRecord {
                        device_id: device_id.clone(),
                        principal_id: principal.id.clone(),
                        permission: permission.into(),
                        request: request.clone(),
                        browser_tx: Some(browser_tx.clone()),
                        resume_token_hash: hash_token(&raw_resume),
                        pending_resume_token: Some(raw_resume.clone()),
                        resource_id: None,
                        created_at_unix_ms: now,
                        last_activity_unix_ms: now,
                        detached_until_unix_ms: None,
                        next_input_seq: 0,
                        acked_input_seq: 0,
                        committed_bytes: None,
                        resource_progress_bytes: 0,
                        history: VecDeque::new(),
                        history_bytes: 0,
                        agent_instance_id: agent_instance_id.clone(),
                    },
                );
            }
            if let Some(snapshot) = state.stream_relays.lock().await.get(&relay_id).cloned() {
                persist_shared_relay(&state, &relay_id, &snapshot);
            }
            let mut authorization = RemoteCommandV1 {
                version: vsn_remote::REMOTE_PROTOCOL_VERSION,
                command_id: random_id("streamcmd"),
                device_id: device_id.clone(),
                principal_id: principal.id.clone(),
                issued_at_unix_ms: now,
                expires_at_unix_ms: now + 30_000,
                permission: permission.into(),
                command: "stream.relay.open".into(),
                params: serde_json::to_value(&request).unwrap_or(Value::Null),
                session_id: relay_id.clone(),
                signature: String::new(),
            };
            if vsn_remote::sign_remote_command(&state.private_key, &mut authorization).is_err()
                || send_agent_stream_message(
                    &state,
                    &agent_instance_id,
                    &device_id,
                    &relay_id,
                    vsn_remote::AgentStreamServerMessageV1::Open {
                        relay_id: relay_id.clone(),
                        authorization: Box::new(authorization),
                        request: request.clone(),
                    },
                )
                .await
                .is_err()
            {
                state.stream_relays.lock().await.remove(&relay_id);
                delete_shared_relay(&state, &relay_id);
                let _ = send_browser_stream(
                    &mut socket,
                    &vsn_remote::BrowserStreamServerMessageV1::Error {
                        message: "device stream channel unavailable".into(),
                    },
                )
                .await;
                return;
            }
            (relay_id, agent_instance_id, raw_resume, false, 0, false)
        };
    if resumed && needs_reopen {
        let snapshot = { state.stream_relays.lock().await.get(&relay_id).cloned() };
        let Some(relay) = snapshot else {
            let _ = send_browser_stream(
                &mut socket,
                &vsn_remote::BrowserStreamServerMessageV1::Error {
                    message: "resume relay disappeared".into(),
                },
            )
            .await;
            return;
        };
        let reopen_request = build_reopen_request(&relay);
        let mut authorization = RemoteCommandV1 {
            version: vsn_remote::REMOTE_PROTOCOL_VERSION,
            command_id: random_id("streamcmd"),
            device_id: device_id.clone(),
            principal_id: principal.id.clone(),
            issued_at_unix_ms: now,
            expires_at_unix_ms: now + 30_000,
            permission: permission.into(),
            command: "stream.relay.open".into(),
            params: serde_json::to_value(&reopen_request).unwrap_or(Value::Null),
            session_id: relay_id.clone(),
            signature: String::new(),
        };
        if vsn_remote::sign_remote_command(&state.private_key, &mut authorization).is_err()
            || send_agent_stream_message(
                &state,
                &agent_instance_id,
                &device_id,
                &relay_id,
                vsn_remote::AgentStreamServerMessageV1::Open {
                    relay_id: relay_id.clone(),
                    authorization: Box::new(authorization),
                    request: reopen_request,
                },
            )
            .await
            .is_err()
        {
            let _ = send_browser_stream(
                &mut socket,
                &vsn_remote::BrowserStreamServerMessageV1::Error {
                    message: "recoverable resource could not be reopened on the Agent".into(),
                },
            )
            .await;
            detach_browser_relay(&state, &relay_id).await;
            return;
        }
    }
    if resumed {
        let resource_id = {
            state
                .stream_relays
                .lock()
                .await
                .get(&relay_id)
                .and_then(|r| r.resource_id.clone())
        };
        let opened = vsn_remote::BrowserStreamServerMessageV1::Opened {
            relay_id: relay_id.clone(),
            ok: true,
            resource_id,
            error: None,
            resume_token: Some(resume_token.clone()),
            resumed: true,
            next_input_seq,
        };
        if send_browser_stream(&mut socket, &opened).await.is_err() {
            detach_browser_relay(&state, &relay_id).await;
            return;
        }
        for frame in replay_frames {
            if send_browser_stream(
                &mut socket,
                &vsn_remote::BrowserStreamServerMessageV1::Output { frame },
            )
            .await
            .is_err()
            {
                detach_browser_relay(&state, &relay_id).await;
                return;
            }
        }
    }
    let (mut sender, mut receiver) = socket.split();
    let mut explicit_close = false;
    loop {
        tokio::select! {
            incoming=receiver.next()=>{
                let Some(Ok(message))=incoming else{break};let parsed=match parse_ws_json::<vsn_remote::BrowserStreamClientMessageV1>(message){Ok(v)=>v,Err(_)=>continue};
                match parsed{
                    vsn_remote::BrowserStreamClientMessageV1::Input{frame}=>{
                        if frame.decoded_len().is_err(){break;}
                        let accepted={let relays=state.stream_relays.lock().await;matches!(relays.get(&relay_id),Some(r) if frame.seq==r.next_input_seq)};
                        if !accepted{let msg=vsn_remote::BrowserStreamServerMessageV1::Error{message:"stream input sequence mismatch; reconnect using the advertised next_input_seq".into()};let text=serde_json::to_string(&msg).unwrap_or_default();let _=sender.send(WsMessage::Text(text.into())).await;continue;}
                        let sent_seq=frame.seq;if send_agent_stream_message(&state,&agent_instance_id,&device_id,&relay_id,vsn_remote::AgentStreamServerMessageV1::Input{relay_id:relay_id.clone(),frame}).await.is_err(){break;}
                        if let Some(r)=state.stream_relays.lock().await.get_mut(&relay_id){if r.next_input_seq==sent_seq{r.next_input_seq=r.next_input_seq.saturating_add(1);r.last_activity_unix_ms=vsn_remote::now_ms();persist_shared_relay(&state,&relay_id,r);}}
                    },
                    vsn_remote::BrowserStreamClientMessageV1::Close{reason}=>{explicit_close=true;let _=send_agent_stream_message(&state,&agent_instance_id,&device_id,&relay_id,vsn_remote::AgentStreamServerMessageV1::Close{relay_id:relay_id.clone(),reason}).await;break;},
                    vsn_remote::BrowserStreamClientMessageV1::Ping{timestamp_unix_ms}=>{let _=send_agent_stream_message(&state,&agent_instance_id,&device_id,&relay_id,vsn_remote::AgentStreamServerMessageV1::Ping{timestamp_unix_ms}).await;},
                    vsn_remote::BrowserStreamClientMessageV1::Hello{..}=>{},
                }
            },
            outbound=browser_rx.recv()=>{let Some(message)=outbound else{break};let text=match serde_json::to_string(&message){Ok(v)=>v,Err(_)=>continue};if sender.send(WsMessage::Text(text.into())).await.is_err(){break;}}
        }
    }
    if explicit_close {
        state.stream_relays.lock().await.remove(&relay_id);
        delete_shared_relay(&state, &relay_id);
    } else {
        detach_browser_relay(&state, &relay_id).await;
    }
}

async fn detach_browser_relay(state: &AppState, relay_id: &str) {
    let now = vsn_remote::now_ms();
    if let Some(r) = state.stream_relays.lock().await.get_mut(relay_id) {
        r.browser_tx = None;
        r.detached_until_unix_ms = Some(now + STREAM_RELAY_RESUME_MS);
        r.last_activity_unix_ms = now;
        persist_shared_relay(state, relay_id, r);
    }
}

fn relay_permission(
    kind: &vsn_stream::StreamKind,
    direction: &vsn_stream::StreamDirection,
) -> Result<&'static str, String> {
    use vsn_stream::{StreamDirection::*, StreamKind::*};
    match (kind, direction) {
        (Terminal, Bidirectional) => Ok("terminal.execute"),
        (FileUpload, ClientToAgent) => Ok("files.write"),
        (FileDownload, AgentToClient) => Ok("files.read"),
        (Preview, AgentToClient) => Ok("project.view"),
        (Preview, Bidirectional) => Ok("project.edit"),
        (Database, AgentToClient) => Ok("database.query"),
        _ => Err("stream kind/direction combination is not remotely supported".into()),
    }
}
async fn touch_relay(state: &AppState, relay_id: &str) {
    if let Some(r) = state.stream_relays.lock().await.get_mut(relay_id) {
        r.last_activity_unix_ms = vsn_remote::now_ms();
    }
}

fn relay_history_push(relay: &mut StreamRelayRecord, frame: &vsn_remote::RelayStreamFrameV1) {
    let len = frame.decoded_len().unwrap_or(0);
    relay.history.push_back(frame.clone());
    relay.history_bytes = relay.history_bytes.saturating_add(len);
    while relay.history.len() > STREAM_RELAY_HISTORY_FRAMES
        || relay.history_bytes > STREAM_RELAY_HISTORY_BYTES
    {
        if let Some(old) = relay.history.pop_front() {
            relay.history_bytes = relay
                .history_bytes
                .saturating_sub(old.decoded_len().unwrap_or(0));
        } else {
            break;
        }
    }
}

fn shared_relay_record(
    relay_id: &str,
    relay: &StreamRelayRecord,
) -> Result<vsn_control_store::SharedStreamCheckpoint, String> {
    Ok(vsn_control_store::SharedStreamCheckpoint {
        relay_id: relay_id.into(),
        device_id: relay.device_id.clone(),
        principal_id: relay.principal_id.clone(),
        permission: relay.permission.clone(),
        request_json: serde_json::to_string(&relay.request).map_err(|e| e.to_string())?,
        agent_instance_id: relay.agent_instance_id.clone(),
        resume_token_hash: relay.resume_token_hash.clone(),
        resource_id: relay.resource_id.clone(),
        next_input_seq: relay.next_input_seq,
        acked_input_seq: relay.acked_input_seq,
        committed_bytes: relay.committed_bytes,
        resource_progress_bytes: relay.resource_progress_bytes,
        created_at_unix_ms: relay.created_at_unix_ms,
        last_activity_unix_ms: relay.last_activity_unix_ms,
        detached_until_unix_ms: relay.detached_until_unix_ms,
        expires_at_unix_ms: relay
            .last_activity_unix_ms
            .saturating_add(STREAM_RELAY_SHARED_TTL_MS),
    })
}
fn persist_shared_relay(state: &AppState, relay_id: &str, relay: &StreamRelayRecord) {
    if let Some(store) = state.state_postgres.as_ref() {
        if let Ok(record) = shared_relay_record(relay_id, relay) {
            let _ = store.upsert_stream_checkpoint(&record);
        }
    }
}
fn delete_shared_relay(state: &AppState, relay_id: &str) {
    if let Some(store) = state.state_postgres.as_ref() {
        let _ = store.delete_stream_checkpoint(relay_id);
    }
}

async fn ensure_shared_relay_loaded(state: &AppState, relay_id: &str) -> Result<(), String> {
    if state.stream_relays.lock().await.contains_key(relay_id) {
        return Ok(());
    }
    let Some(store) = state.state_postgres.as_ref() else {
        return Err("resume relay not found or expired".into());
    };
    let checkpoint = store
        .stream_checkpoint(relay_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "resume relay not found or expired".to_string())?;
    let request: vsn_remote::RelayStreamOpenV1 = serde_json::from_str(&checkpoint.request_json)
        .map_err(|_| "shared relay request is invalid".to_string())?;
    let mut history = VecDeque::new();
    let mut history_bytes = 0usize;
    for item in store
        .stream_frames_after(relay_id, None, STREAM_RELAY_HISTORY_FRAMES as u32)
        .map_err(|e| e.to_string())?
    {
        let frame: vsn_remote::RelayStreamFrameV1 = serde_json::from_str(&item.frame_json)
            .map_err(|_| "shared relay frame is invalid".to_string())?;
        history_bytes = history_bytes.saturating_add(frame.decoded_len().unwrap_or(0));
        history.push_back(frame);
        while history.len() > STREAM_RELAY_HISTORY_FRAMES
            || history_bytes > STREAM_RELAY_HISTORY_BYTES
        {
            if let Some(old) = history.pop_front() {
                history_bytes = history_bytes.saturating_sub(old.decoded_len().unwrap_or(0));
            } else {
                break;
            }
        }
    }
    let relay = StreamRelayRecord {
        device_id: checkpoint.device_id,
        principal_id: checkpoint.principal_id,
        permission: checkpoint.permission,
        request,
        browser_tx: None,
        resume_token_hash: checkpoint.resume_token_hash,
        pending_resume_token: None,
        resource_id: checkpoint.resource_id,
        created_at_unix_ms: checkpoint.created_at_unix_ms,
        last_activity_unix_ms: checkpoint.last_activity_unix_ms,
        detached_until_unix_ms: checkpoint.detached_until_unix_ms,
        next_input_seq: checkpoint.next_input_seq,
        acked_input_seq: checkpoint.acked_input_seq,
        committed_bytes: checkpoint.committed_bytes,
        resource_progress_bytes: checkpoint.resource_progress_bytes,
        history,
        history_bytes,
        agent_instance_id: checkpoint.agent_instance_id,
    };
    state
        .stream_relays
        .lock()
        .await
        .entry(relay_id.into())
        .or_insert(relay);
    Ok(())
}

fn recoverable_after_agent_disconnect(relay: &StreamRelayRecord) -> bool {
    match relay.request.kind {
        vsn_stream::StreamKind::FileUpload | vsn_stream::StreamKind::FileDownload => true,
        vsn_stream::StreamKind::Preview | vsn_stream::StreamKind::Database => {
            relay.resource_progress_bytes == 0
        }
        _ => false,
    }
}
fn build_reopen_request(relay: &StreamRelayRecord) -> vsn_remote::RelayStreamOpenV1 {
    let mut request = relay.request.clone();
    request.metadata.insert(
        "vsn_resume_input_seq".into(),
        relay.acked_input_seq.to_string(),
    );
    let next_out = relay
        .history
        .back()
        .map(|f| f.seq.saturating_add(1))
        .unwrap_or(0);
    request
        .metadata
        .insert("vsn_resume_output_seq".into(), next_out.to_string());
    match request.kind {
        vsn_stream::StreamKind::FileUpload => {
            if let Some(bytes) = relay.committed_bytes {
                request.metadata.insert("offset".into(), bytes.to_string());
            }
        }
        vsn_stream::StreamKind::FileDownload => {
            let base = relay
                .request
                .metadata
                .get("offset")
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(0);
            request.metadata.insert(
                "offset".into(),
                base.saturating_add(relay.resource_progress_bytes)
                    .to_string(),
            );
        }
        _ => {}
    }
    request
}
async fn reopen_recoverable_relays(
    state: &AppState,
    device_id: &str,
    new_instance_id: &str,
) -> Vec<String> {
    let ids = {
        let relays = state.stream_relays.lock().await;
        relays
            .iter()
            .filter(|(_, r)| {
                r.device_id == device_id
                    && r.agent_instance_id == "reconnecting"
                    && r.browser_tx.is_some()
                    && recoverable_after_agent_disconnect(r)
            })
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>()
    };
    let mut reopened = Vec::new();
    for relay_id in ids {
        let snapshot = { state.stream_relays.lock().await.get(&relay_id).cloned() };
        let Some(mut relay) = snapshot else { continue };
        let request = build_reopen_request(&relay);
        let now = vsn_remote::now_ms();
        let mut authorization = RemoteCommandV1 {
            version: vsn_remote::REMOTE_PROTOCOL_VERSION,
            command_id: random_id("streamcmd"),
            device_id: device_id.into(),
            principal_id: relay.principal_id.clone(),
            issued_at_unix_ms: now,
            expires_at_unix_ms: now + 30_000,
            permission: relay.permission.clone(),
            command: "stream.relay.open".into(),
            params: serde_json::to_value(&request).unwrap_or(Value::Null),
            session_id: relay_id.clone(),
            signature: String::new(),
        };
        if vsn_remote::sign_remote_command(&state.private_key, &mut authorization).is_err() {
            continue;
        }
        relay.agent_instance_id = new_instance_id.into();
        relay.last_activity_unix_ms = now;
        {
            let mut relays = state.stream_relays.lock().await;
            if let Some(current) = relays.get_mut(&relay_id) {
                current.agent_instance_id = relay.agent_instance_id.clone();
                current.last_activity_unix_ms = now;
                current.detached_until_unix_ms = None;
                persist_shared_relay(state, &relay_id, current);
            }
        }
        if send_agent_stream_message(
            state,
            new_instance_id,
            device_id,
            &relay_id,
            vsn_remote::AgentStreamServerMessageV1::Open {
                relay_id: relay_id.clone(),
                authorization: Box::new(authorization),
                request,
            },
        )
        .await
        .is_ok()
        {
            reopened.push(relay_id);
        } else {
            if let Some(r) = state.stream_relays.lock().await.get_mut(&relay_id) {
                r.agent_instance_id = "reconnecting".into();
                persist_shared_relay(state, &relay_id, r);
            }
        }
    }
    reopened
}

async fn route_agent_stream_message(
    state: &AppState,
    relay_id: &str,
    mut message: vsn_remote::BrowserStreamServerMessageV1,
) {
    let local_target = {
        let mut relays = state.stream_relays.lock().await;
        if let Some(r) = relays.get_mut(relay_id) {
            r.last_activity_unix_ms = vsn_remote::now_ms();
            match &mut message {
                vsn_remote::BrowserStreamServerMessageV1::Opened {
                    ok,
                    resource_id,
                    resume_token,
                    resumed,
                    next_input_seq,
                    ..
                } => {
                    if *ok {
                        r.resource_id = resource_id.clone();
                    }
                    *resume_token = r.pending_resume_token.take();
                    *resumed = false;
                    *next_input_seq = r.next_input_seq;
                }
                vsn_remote::BrowserStreamServerMessageV1::Output { frame } => {
                    let bytes = frame.decoded_len().unwrap_or(0) as u64;
                    r.resource_progress_bytes = r.resource_progress_bytes.saturating_add(bytes);
                    relay_history_push(r, frame);
                    if let Some(store) = state.state_postgres.as_ref() {
                        if let Ok(payload) = serde_json::to_string(frame) {
                            let _ = store.append_stream_frame(
                                relay_id,
                                frame.seq,
                                &payload,
                                vsn_remote::now_ms(),
                                STREAM_RELAY_HISTORY_FRAMES as u32,
                            );
                        }
                    }
                }
                vsn_remote::BrowserStreamServerMessageV1::InputAck {
                    next_input_seq,
                    committed_bytes,
                    ..
                } => {
                    r.acked_input_seq = r.acked_input_seq.max(*next_input_seq);
                    if committed_bytes.is_some() {
                        r.committed_bytes = *committed_bytes;
                    }
                }
                _ => {}
            }
            persist_shared_relay(state, relay_id, r);
            Some((
                r.browser_tx.clone(),
                r.device_id.clone(),
                r.agent_instance_id.clone(),
            ))
        } else {
            None
        }
    };
    if let Some((tx, device_id, agent_instance_id)) = local_target {
        let closing = matches!(
            message,
            vsn_remote::BrowserStreamServerMessageV1::Closed { .. }
        );
        if let Some(tx) = tx {
            if tx.try_send(message).is_err() {
                detach_browser_relay(state, relay_id).await;
                let _ = send_agent_stream_message(
                    state,
                    &agent_instance_id,
                    &device_id,
                    relay_id,
                    vsn_remote::AgentStreamServerMessageV1::Close {
                        relay_id: relay_id.into(),
                        reason: Some("browser_backpressure".into()),
                    },
                )
                .await;
            }
        }
        if closing {
            state.stream_relays.lock().await.remove(relay_id);
            delete_shared_relay(state, relay_id);
        }
        return;
    }
    let remote_home = {
        let mut homes = state.remote_stream_homes.lock().await;
        if let Some(home) = homes.get_mut(relay_id) {
            home.last_activity_unix_ms = vsn_remote::now_ms();
            Some(home.clone())
        } else {
            None
        }
    };
    if let Some(home) = remote_home {
        let closing = matches!(
            message,
            vsn_remote::BrowserStreamServerMessageV1::Closed { .. }
        );
        let _ = publish_browser_bus(state, &home.home_instance_id, relay_id, message);
        if closing {
            state.remote_stream_homes.lock().await.remove(relay_id);
        }
    }
}

async fn send_agent_stream_message(
    state: &AppState,
    agent_instance_id: &str,
    device_id: &str,
    relay_id: &str,
    message: vsn_remote::AgentStreamServerMessageV1,
) -> Result<(), String> {
    if agent_instance_id == state.instance_id.as_str() {
        let peer = state
            .agent_stream_peers
            .lock()
            .await
            .get(device_id)
            .cloned()
            .ok_or_else(|| "local agent stream peer unavailable".to_string())?;
        peer.tx
            .send(message)
            .await
            .map_err(|_| "local agent stream channel closed".to_string())
    } else {
        let store =
            state.state_postgres.as_ref().as_ref().ok_or_else(|| {
                "cross-instance stream relay requires shared PostgreSQL".to_string()
            })?;
        let envelope = ClusterStreamBusV1::ToAgent {
            home_instance_id: state.instance_id.as_str().to_string(),
            device_id: device_id.to_string(),
            relay_id: relay_id.to_string(),
            message,
        };
        let payload = serde_json::to_string(&envelope).map_err(|e| e.to_string())?;
        store
            .publish_bus(
                state.instance_id.as_str(),
                agent_instance_id,
                "stream_relay",
                &payload,
                60_000,
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

fn publish_browser_bus(
    state: &AppState,
    home_instance_id: &str,
    relay_id: &str,
    message: vsn_remote::BrowserStreamServerMessageV1,
) -> Result<(), String> {
    if home_instance_id == state.instance_id.as_str() {
        return Err("browser relay unexpectedly targeted local instance".into());
    }
    let store = state
        .state_postgres
        .as_ref()
        .as_ref()
        .ok_or_else(|| "cross-instance browser relay requires shared PostgreSQL".to_string())?;
    let envelope = ClusterStreamBusV1::ToBrowser {
        relay_id: relay_id.to_string(),
        message,
    };
    let payload = serde_json::to_string(&envelope).map_err(|e| e.to_string())?;
    store
        .publish_bus(
            state.instance_id.as_str(),
            home_instance_id,
            "stream_relay",
            &payload,
            60_000,
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

async fn run_cluster_stream_bus(state: AppState) {
    let mut tick = tokio::time::interval(std::time::Duration::from_millis(100));
    let mut after_id = 0i64;
    loop {
        tick.tick().await;
        let Some(store) = state.state_postgres.as_ref() else {
            return;
        };
        let messages = match store.poll_bus(state.instance_id.as_str(), after_id, 128) {
            Ok(v) => v,
            Err(_) => continue,
        };
        for item in messages {
            let parsed = serde_json::from_str::<ClusterStreamBusV1>(&item.payload);
            let processed = match parsed {
                Ok(envelope) => handle_cluster_stream_bus_message(&state, envelope).await,
                Err(_) => true,
            };
            if processed {
                let _ = store.ack_bus(state.instance_id.as_str(), item.id);
                after_id = item.id;
            } else {
                break;
            }
        }
    }
}

async fn handle_cluster_stream_bus_message(state: &AppState, envelope: ClusterStreamBusV1) -> bool {
    match envelope {
        ClusterStreamBusV1::ToAgent {
            home_instance_id,
            device_id,
            relay_id,
            message,
        } => {
            let peer = state
                .agent_stream_peers
                .lock()
                .await
                .get(&device_id)
                .cloned();
            let Some(peer) = peer else {
                let _ = publish_browser_bus(
                    state,
                    &home_instance_id,
                    &relay_id,
                    vsn_remote::BrowserStreamServerMessageV1::Error {
                        message: "device stream peer left the owning Control Plane instance".into(),
                    },
                );
                return true;
            };
            if matches!(message, vsn_remote::AgentStreamServerMessageV1::Open { .. }) {
                state.remote_stream_homes.lock().await.insert(
                    relay_id.clone(),
                    RemoteRelayHome {
                        home_instance_id: home_instance_id.clone(),
                        device_id: device_id.clone(),
                        last_activity_unix_ms: vsn_remote::now_ms(),
                    },
                );
            }
            let closing = matches!(
                message,
                vsn_remote::AgentStreamServerMessageV1::Close { .. }
            );
            if peer.tx.send(message).await.is_err() {
                let _ = publish_browser_bus(
                    state,
                    &home_instance_id,
                    &relay_id,
                    vsn_remote::BrowserStreamServerMessageV1::Closed {
                        reason: Some("agent stream channel unavailable".into()),
                    },
                );
            }
            if closing {
                state.remote_stream_homes.lock().await.remove(&relay_id);
            }
            true
        }
        ClusterStreamBusV1::ToBrowser { relay_id, message } => {
            route_agent_stream_message(state, &relay_id, message).await;
            true
        }
    }
}

async fn run_stream_relay_cleanup(state: AppState) {
    let mut tick = tokio::time::interval(std::time::Duration::from_secs(5));
    loop {
        tick.tick().await;
        let now = vsn_remote::now_ms();
        let expired = {
            let mut relays = state.stream_relays.lock().await;
            let ids = relays
                .iter()
                .filter(|(_, r)| {
                    r.detached_until_unix_ms.map(|v| v < now).unwrap_or(false)
                        || now.saturating_sub(r.last_activity_unix_ms) > STREAM_RELAY_IDLE_MS
                })
                .map(|(id, _)| id.clone())
                .collect::<Vec<_>>();
            let mut out = Vec::new();
            for id in ids {
                if let Some(r) = relays.remove(&id) {
                    out.push((id, r));
                }
            }
            out
        };
        for (relay_id, relay) in expired {
            if relay.agent_instance_id != "reconnecting" {
                let _ = send_agent_stream_message(
                    &state,
                    &relay.agent_instance_id,
                    &relay.device_id,
                    &relay_id,
                    vsn_remote::AgentStreamServerMessageV1::Close {
                        relay_id: relay_id.clone(),
                        reason: Some("relay_resume_window_expired".into()),
                    },
                )
                .await;
            }
            delete_shared_relay(&state, &relay_id);
        }
        if let Some(store) = state.state_postgres.as_ref() {
            let _ = store.cleanup_stream_checkpoints();
        }
        let mut homes = state.remote_stream_homes.lock().await;
        homes.retain(|_, h| now.saturating_sub(h.last_activity_unix_ms) <= STREAM_RELAY_IDLE_MS);
    }
}

fn parse_ws_json<T: serde::de::DeserializeOwned>(message: WsMessage) -> Result<T, ()> {
    let bytes = match message {
        WsMessage::Text(v) => v.as_bytes().to_vec(),
        WsMessage::Binary(v) => v.to_vec(),
        _ => return Err(()),
    };
    if bytes.len() > 2 * 1024 * 1024 {
        return Err(());
    }
    serde_json::from_slice(&bytes).map_err(|_| ())
}
async fn send_ws_json<T: Serialize>(socket: &mut WebSocket, value: &T) -> Result<(), ()> {
    let text = serde_json::to_string(value).map_err(|_| ())?;
    socket
        .send(WsMessage::Text(text.into()))
        .await
        .map_err(|_| ())
}
async fn send_browser_stream(
    socket: &mut WebSocket,
    value: &vsn_remote::BrowserStreamServerMessageV1,
) -> Result<(), ()> {
    send_ws_json(socket, value).await
}
fn api_error_message(error: &ApiError) -> String {
    error
        .1
         .0
        .get("error")
        .and_then(Value::as_str)
        .unwrap_or("gateway request rejected")
        .to_string()
}

async fn queue_command(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<CommandRequest>,
) -> Result<Json<Value>, ApiError> {
    let principal = require_permission(&state, &headers, "control.commands.queue")?;
    validate_command_request(&state, &principal, &input)?;
    if approval_required(&input.permission, &input.command) {
        let now = vsn_remote::now_ms();
        let approval = ApprovalRecord {
            id: random_id("approval"),
            requester_id: principal.id,
            requested_at_unix_ms: now,
            expires_at_unix_ms: now + 10 * 60 * 1000,
            state: ApprovalState::Pending,
            request: input,
            approver_id: None,
            decided_at_unix_ms: None,
        };
        if let Some(store) = state.state_postgres.as_ref() {
            let payload = serde_json::to_string(&approval).map_err(|e| {
                api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("approval serialization failed: {e}"),
                )
            })?;
            store
                .create_approval(
                    &approval.id,
                    &payload,
                    approval.requested_at_unix_ms,
                    approval.expires_at_unix_ms,
                )
                .map_err(|e| {
                    api_error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        &format!("shared approval create failed: {e}"),
                    )
                })?;
        } else {
            state
                .approvals
                .lock()
                .map_err(lock_error)?
                .insert(approval.id.clone(), approval.clone());
            persist_state(&state)?;
        }
        return Ok(Json(
            json!({"status":"pending_approval","approval":approval,"shared":state.state_postgres.is_some()}),
        ));
    }
    let command = queue_signed_command(&state, &principal.id, input)?;
    if state.state_postgres.is_none() {
        persist_state(&state)?;
    }
    Ok(Json(json!({"status":"queued","command":command})))
}

fn validate_command_request(
    state: &AppState,
    principal: &AuthPrincipal,
    input: &CommandRequest,
) -> Result<(), ApiError> {
    if !principal.allows(&input.permission) {
        return Err(api_error(
            StatusCode::FORBIDDEN,
            "token role may not delegate the requested agent permission",
        ));
    }
    if !(1_000..=vsn_remote::MAX_REMOTE_COMMAND_TTL_MS).contains(&input.ttl_ms) {
        return Err(api_error(StatusCode::BAD_REQUEST, "ttl_ms outside policy"));
    }
    if input.command.trim().is_empty() || input.command.len() > 128 {
        return Err(api_error(StatusCode::BAD_REQUEST, "invalid command"));
    }
    if enrolled_device_record(state, &input.device_id)?.is_none() {
        return Err(api_error(StatusCode::NOT_FOUND, "device is not enrolled"));
    }
    Ok(())
}
fn build_signed_command(
    state: &AppState,
    principal_id: &str,
    input: CommandRequest,
) -> Result<RemoteCommandV1, ApiError> {
    let now = vsn_remote::now_ms();
    let mut command = RemoteCommandV1 {
        version: vsn_remote::REMOTE_PROTOCOL_VERSION,
        command_id: random_id("cmd"),
        device_id: input.device_id,
        principal_id: principal_id.into(),
        issued_at_unix_ms: now,
        expires_at_unix_ms: now + input.ttl_ms,
        permission: input.permission,
        command: input.command,
        params: input.params,
        session_id: random_id("session"),
        signature: String::new(),
    };
    vsn_remote::sign_remote_command(&state.private_key, &mut command).map_err(remote_error)?;
    Ok(command)
}
fn queue_signed_command(
    state: &AppState,
    principal_id: &str,
    input: CommandRequest,
) -> Result<RemoteCommandV1, ApiError> {
    let command = build_signed_command(state, principal_id, input)?;
    let device_id = command.device_id.clone();
    if let Some(store) = state.state_postgres.as_ref() {
        let payload = serde_json::to_string(&command).map_err(|e| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("shared command serialization failed: {e}"),
            )
        })?;
        store
            .enqueue_command(
                &command.command_id,
                &device_id,
                &payload,
                command.expires_at_unix_ms,
            )
            .map_err(|e| {
                api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("shared command enqueue failed: {e}"),
                )
            })?;
    } else {
        state
            .queues
            .lock()
            .map_err(lock_error)?
            .entry(device_id)
            .or_default()
            .push_back(command.clone());
    }
    state
        .deliveries
        .lock()
        .map_err(lock_error)?
        .insert(command.command_id.clone(), DeliveryMeta::default());
    Ok(command)
}
fn approval_required(permission: &str, command: &str) -> bool {
    matches!(
        permission,
        "terminal.execute"
            | "files.write"
            | "database.query"
            | "service.manage"
            | "runtime.manage"
            | "project.edit"
            | "network.manage"
            | "secrets.manage"
            | "secrets.reveal"
    ) || matches!(command, "files.binary.write" | "files.binary.abort")
}

async fn agent_audit(
    State(state): State<AppState>,
    Json(batch): Json<AgentAuditBatchV1>,
) -> Result<Json<Value>, ApiError> {
    if batch.version != vsn_remote::REMOTE_PROTOCOL_VERSION {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "unsupported audit batch version",
        ));
    }
    if batch.events.is_empty() || batch.events.len() > 256 {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "audit batch must contain 1..256 events",
        ));
    }
    let device = enrolled_device_record(&state, &batch.device_id)?
        .ok_or_else(|| api_error(StatusCode::UNAUTHORIZED, "device is not enrolled"))?;
    for event in &batch.events {
        if event.device_id != batch.device_id || event.signer_public_key != device.public_key {
            return Err(api_error(
                StatusCode::UNAUTHORIZED,
                "audit event device identity mismatch",
            ));
        }
        vsn_audit::verify_event(event).map_err(|e| {
            api_error(
                StatusCode::UNAUTHORIZED,
                &format!("audit signature/hash invalid: {e}"),
            )
        })?;
    }
    if let Some(store) = state.state_postgres.as_ref() {
        let shared = batch
            .events
            .iter()
            .map(|event| {
                Ok(vsn_control_store::SharedAuditInput {
                    event_id: event.event_id.clone(),
                    previous_hash: event.previous_hash.clone(),
                    event_hash: event.event_hash.clone(),
                    timestamp_unix_ms: event.timestamp_unix_ms,
                    payload: serde_json::to_string(event).map_err(|e| {
                        api_error(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            &format!("audit serialization failed: {e}"),
                        )
                    })?,
                })
            })
            .collect::<Result<Vec<_>, ApiError>>()?;
        let appended = store
            .append_audit_batch(&batch.device_id, &shared)
            .map_err(|e| {
                api_error(
                    StatusCode::CONFLICT,
                    &format!("shared audit append failed: {e}"),
                )
            })?;
        return Ok(Json(
            json!({"ok":true,"accepted":appended.accepted,"duplicates":appended.duplicates,"last_hash":appended.last_hash,"shared":true}),
        ));
    }
    let mut expected_previous = {
        let events = state.central_audit.lock().map_err(lock_error)?;
        events
            .iter()
            .rev()
            .find(|e| e.device_id == batch.device_id)
            .map(|e| e.event_hash.clone())
            .unwrap_or_else(|| "GENESIS".into())
    };
    let mut seen_ids = HashSet::new();
    {
        let events = state.central_audit.lock().map_err(lock_error)?;
        for event in events.iter().filter(|e| e.device_id == batch.device_id) {
            seen_ids.insert(event.event_id.clone());
        }
    }
    let mut accepted = Vec::new();
    let mut duplicates = 0usize;
    for event in batch.events {
        if seen_ids.contains(&event.event_id) {
            duplicates += 1;
            continue;
        }
        if event.previous_hash != expected_previous {
            return Err(api_error(
                StatusCode::CONFLICT,
                "central audit chain continuity mismatch",
            ));
        }
        expected_previous = event.event_hash.clone();
        seen_ids.insert(event.event_id.clone());
        accepted.push(event);
    }
    let accepted_count = accepted.len();
    if !accepted.is_empty() {
        let mut central = state.central_audit.lock().map_err(lock_error)?;
        central.extend(accepted);
        if central.len() > 50_000 {
            let drain = central.len() - 50_000;
            central.drain(0..drain);
        }
        drop(central);
        persist_state(&state)?;
    }
    Ok(Json(
        json!({"ok":true,"accepted":accepted_count,"duplicates":duplicates,"last_hash":expected_previous,"shared":false}),
    ))
}

async fn fleet_overview(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    require_permission(&state, &headers, "control.fleet.view")?;
    refresh_shared_iam_fleet_state(&state)?;
    let now = vsn_remote::now_ms();
    let devices = all_device_records(&state)?
        .into_iter()
        .map(|d| {
            let online = now.saturating_sub(d.last_seen_unix_ms) < 30_000;
            json!({"device":d,"online":online})
        })
        .collect::<Vec<_>>();
    let groups = state
        .fleet_groups
        .lock()
        .map_err(lock_error)?
        .values()
        .cloned()
        .collect::<Vec<_>>();
    let environments = state
        .environments
        .lock()
        .map_err(lock_error)?
        .values()
        .cloned()
        .collect::<Vec<_>>();
    Ok(Json(
        json!({"devices":devices,"groups":groups,"environments":environments,"shared":state.state_postgres.is_some()}),
    ))
}
async fn upsert_fleet_group(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<FleetGroupRequest>,
) -> Result<Json<FleetGroup>, ApiError> {
    require_permission(&state, &headers, "control.fleet.manage")?;
    refresh_shared_iam_fleet_state(&state)?;
    validate_id(&input.id)?;
    if input.name.trim().is_empty()
        || input.name.len() > 128
        || input.device_ids.len() > 512
        || input.labels.len() > 64
    {
        return Err(api_error(StatusCode::BAD_REQUEST, "invalid fleet group"));
    }
    for id in &input.device_ids {
        if enrolled_device_record(&state, id)?.is_none() {
            return Err(api_error(
                StatusCode::NOT_FOUND,
                "fleet group references unknown device",
            ));
        }
    }
    let group = FleetGroup {
        id: input.id,
        name: input.name,
        device_ids: input.device_ids,
        labels: input.labels,
    };
    state
        .fleet_groups
        .lock()
        .map_err(lock_error)?
        .insert(group.id.clone(), group.clone());
    sync_shared_fleet_group(&state, &group)?;
    persist_operational_state(&state)?;
    Ok(Json(group))
}
async fn update_device_fleet(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<DeviceFleetUpdate>,
) -> Result<Json<DeviceRecord>, ApiError> {
    require_permission(&state, &headers, "control.fleet.manage")?;
    refresh_shared_iam_fleet_state(&state)?;
    if input.labels.len() > 64 || input.groups.len() > 64 {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "too many labels or groups",
        ));
    }
    {
        let groups = state.fleet_groups.lock().map_err(lock_error)?;
        for id in &input.groups {
            if !groups.contains_key(id) {
                return Err(api_error(StatusCode::NOT_FOUND, "unknown fleet group"));
            }
        }
    }
    if enrolled_device_record(&state, &input.device_id)?.is_none() {
        return Err(api_error(StatusCode::NOT_FOUND, "device not found"));
    }
    let record = {
        let mut devices = state.devices.lock().map_err(lock_error)?;
        let device = devices
            .get_mut(&input.device_id)
            .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "device not found"))?;
        device.labels = input.labels;
        device.groups = input.groups;
        device.clone()
    };
    sync_shared_device_fleet(&state, &record)?;
    persist_operational_state(&state)?;
    Ok(Json(record))
}
async fn delete_fleet_group(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<Value>, ApiError> {
    require_permission(&state, &headers, "control.fleet.manage")?;
    validate_id(&id)?;
    refresh_shared_iam_fleet_state(&state)?;
    let removed = state
        .fleet_groups
        .lock()
        .map_err(lock_error)?
        .remove(&id)
        .is_some();
    let mut changed = Vec::new();
    {
        let mut devices = state.devices.lock().map_err(lock_error)?;
        for device in devices.values_mut() {
            let before = device.groups.len();
            device.groups.retain(|g| g != &id);
            if device.groups.len() != before {
                changed.push(device.clone());
            }
        }
    }
    for device in &changed {
        sync_shared_device_fleet(&state, device)?;
    }
    if let Some(store) = state.state_postgres.as_ref() {
        store.delete_fleet_group(&id).map_err(|e| {
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!("shared fleet group delete failed: {e}"),
            )
        })?;
    }
    persist_operational_state(&state)?;
    Ok(Json(
        json!({"ok":true,"removed":removed,"group_id":id,"devices_updated":changed.len()}),
    ))
}
async fn validate_fleet(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    require_permission(&state, &headers, "control.fleet.view")?;
    refresh_shared_iam_fleet_state(&state)?;
    let devices = all_device_records(&state)?;
    let device_ids = devices
        .iter()
        .map(|d| d.device_id.clone())
        .collect::<HashSet<_>>();
    let groups = state
        .fleet_groups
        .lock()
        .map_err(lock_error)?
        .values()
        .cloned()
        .collect::<Vec<_>>();
    let group_ids = groups.iter().map(|g| g.id.clone()).collect::<HashSet<_>>();
    let envs = state
        .environments
        .lock()
        .map_err(lock_error)?
        .values()
        .cloned()
        .collect::<Vec<_>>();
    let mut issues = Vec::new();
    for d in &devices {
        for g in &d.groups {
            if !group_ids.contains(g) {
                issues.push(json!({"severity":"error","kind":"unknown_device_group","device_id":d.device_id,"group_id":g}));
            }
        }
    }
    for g in &groups {
        for d in &g.device_ids {
            if !device_ids.contains(d) {
                issues.push(json!({"severity":"error","kind":"unknown_group_device","group_id":g.id,"device_id":d}));
            }
        }
    }
    for env in &envs {
        for (role, d) in &env.bindings {
            if !device_ids.contains(d) {
                issues.push(json!({"severity":"error","kind":"unknown_environment_device","environment_id":env.id,"role":role,"device_id":d}));
            }
        }
    }
    Ok(Json(
        json!({"ok":issues.is_empty(),"issues":issues,"devices":devices.len(),"groups":groups.len(),"environments":envs.len()}),
    ))
}
async fn list_environments(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    require_permission(&state, &headers, "control.fleet.view")?;
    refresh_shared_iam_fleet_state(&state)?;
    let environments = state
        .environments
        .lock()
        .map_err(lock_error)?
        .values()
        .cloned()
        .collect::<Vec<_>>();
    Ok(Json(
        json!({"environments":environments,"shared":state.state_postgres.is_some()}),
    ))
}
async fn delete_environment(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<Value>, ApiError> {
    require_permission(&state, &headers, "control.fleet.manage")?;
    validate_id(&id)?;
    refresh_shared_iam_fleet_state(&state)?;
    let removed = state
        .environments
        .lock()
        .map_err(lock_error)?
        .remove(&id)
        .is_some();
    if let Some(store) = state.state_postgres.as_ref() {
        store.delete_environment(&id).map_err(|e| {
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!("shared environment delete failed: {e}"),
            )
        })?;
    }
    persist_operational_state(&state)?;
    Ok(Json(
        json!({"ok":true,"removed":removed,"environment_id":id}),
    ))
}
async fn upsert_environment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<EnvironmentRequest>,
) -> Result<Json<EnvironmentRecord>, ApiError> {
    require_permission(&state, &headers, "control.fleet.manage")?;
    refresh_shared_iam_fleet_state(&state)?;
    validate_id(&input.id)?;
    if input.name.trim().is_empty() || input.bindings.len() > 64 || input.labels.len() > 64 {
        return Err(api_error(StatusCode::BAD_REQUEST, "invalid environment"));
    }
    for (role, id) in &input.bindings {
        validate_id(role)?;
        if enrolled_device_record(&state, id)?.is_none() {
            return Err(api_error(
                StatusCode::NOT_FOUND,
                "environment references unknown device",
            ));
        }
    }
    let env = EnvironmentRecord {
        id: input.id,
        name: input.name,
        bindings: input.bindings,
        labels: input.labels,
    };
    state
        .environments
        .lock()
        .map_err(lock_error)?
        .insert(env.id.clone(), env.clone());
    sync_shared_environment(&state, &env)?;
    persist_operational_state(&state)?;
    Ok(Json(env))
}
fn shared_approval_record(
    record: vsn_control_store::SharedApprovalRecord,
) -> Result<ApprovalRecord, ApiError> {
    let mut approval: ApprovalRecord = serde_json::from_str(&record.payload).map_err(|e| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("shared approval payload invalid: {e}"),
        )
    })?;
    approval.state = match record.state.as_str() {
        "pending" => ApprovalState::Pending,
        "approved" => ApprovalState::Approved,
        "rejected" => ApprovalState::Rejected,
        "expired" => ApprovalState::Expired,
        _ => {
            return Err(api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "shared approval has invalid state",
            ))
        }
    };
    approval.approver_id = record.approver_id;
    approval.decided_at_unix_ms = record.decided_at_unix_ms;
    Ok(approval)
}
async fn list_approvals(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    require_permission(&state, &headers, "control.approvals.view")?;
    if let Some(store) = state.state_postgres.as_ref() {
        let records = store.recent_approvals(500).map_err(|e| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("shared approval list failed: {e}"),
            )
        })?;
        let approvals = records
            .into_iter()
            .map(shared_approval_record)
            .collect::<Result<Vec<_>, ApiError>>()?;
        return Ok(Json(json!({"approvals":approvals,"shared":true})));
    }
    let now = vsn_remote::now_ms();
    let mut changed = false;
    {
        let mut approvals = state.approvals.lock().map_err(lock_error)?;
        for approval in approvals.values_mut() {
            if approval.state == ApprovalState::Pending && approval.expires_at_unix_ms < now {
                approval.state = ApprovalState::Expired;
                changed = true;
            }
        }
    }
    if changed {
        persist_state(&state)?;
    }
    let approvals = state
        .approvals
        .lock()
        .map_err(lock_error)?
        .values()
        .cloned()
        .collect::<Vec<_>>();
    Ok(Json(json!({"approvals":approvals,"shared":false})))
}
async fn approve_command(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<ApprovalDecision>,
) -> Result<Json<Value>, ApiError> {
    let principal = require_permission(&state, &headers, "control.approvals.approve")?;
    if let Some(store) = state.state_postgres.as_ref() {
        let record = store
            .approval(&input.approval_id)
            .map_err(|e| {
                api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("shared approval lookup failed: {e}"),
                )
            })?
            .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "approval not found"))?;
        let approval = shared_approval_record(record)?;
        if approval.state != ApprovalState::Pending {
            return Err(api_error(StatusCode::CONFLICT, "approval is not pending"));
        }
        if !principal.allows(&approval.request.permission) {
            return Err(api_error(
                StatusCode::FORBIDDEN,
                "approver may not approve a permission outside their own scope",
            ));
        }
        let command = build_signed_command(&state, &approval.requester_id, approval.request)?;
        let payload = serde_json::to_string(&command).map_err(|e| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("shared command serialization failed: {e}"),
            )
        })?;
        let committed = store
            .approve_and_enqueue(
                &input.approval_id,
                &principal.id,
                &command.command_id,
                &command.device_id,
                &payload,
                command.expires_at_unix_ms,
            )
            .map_err(|e| {
                api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("shared approval decision failed: {e}"),
                )
            })?;
        if !committed {
            return Err(api_error(
                StatusCode::CONFLICT,
                "approval changed or expired during decision",
            ));
        }
        state
            .deliveries
            .lock()
            .map_err(lock_error)?
            .insert(command.command_id.clone(), DeliveryMeta::default());
        return Ok(Json(json!({"ok":true,"command":command,"shared":true})));
    }
    let _decision_guard = state.approval_decision_lock.lock().map_err(lock_error)?;
    let now = vsn_remote::now_ms();
    let (request, requester) = {
        let mut approvals = state.approvals.lock().map_err(lock_error)?;
        let approval = approvals
            .get_mut(&input.approval_id)
            .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "approval not found"))?;
        if approval.state != ApprovalState::Pending {
            return Err(api_error(StatusCode::CONFLICT, "approval is not pending"));
        }
        if approval.expires_at_unix_ms < now {
            approval.state = ApprovalState::Expired;
            return Err(api_error(StatusCode::CONFLICT, "approval expired"));
        }
        if !principal.allows(&approval.request.permission) {
            return Err(api_error(
                StatusCode::FORBIDDEN,
                "approver may not approve a permission outside their own scope",
            ));
        }
        (approval.request.clone(), approval.requester_id.clone())
    };
    let command = queue_signed_command(&state, &requester, request)?;
    {
        let mut approvals = state.approvals.lock().map_err(lock_error)?;
        let approval = approvals.get_mut(&input.approval_id).ok_or_else(|| {
            api_error(
                StatusCode::NOT_FOUND,
                "approval disappeared during decision",
            )
        })?;
        if approval.state != ApprovalState::Pending {
            return Err(api_error(
                StatusCode::CONFLICT,
                "approval changed during decision",
            ));
        }
        approval.state = ApprovalState::Approved;
        approval.approver_id = Some(principal.id.clone());
        approval.decided_at_unix_ms = Some(now);
    }
    persist_state(&state)?;
    Ok(Json(json!({"ok":true,"command":command,"shared":false})))
}
async fn reject_command(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<ApprovalDecision>,
) -> Result<Json<Value>, ApiError> {
    let principal = require_permission(&state, &headers, "control.approvals.approve")?;
    if let Some(store) = state.state_postgres.as_ref() {
        let changed = store
            .reject_approval(&input.approval_id, &principal.id)
            .map_err(|e| {
                api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("shared approval rejection failed: {e}"),
                )
            })?;
        if !changed {
            return Err(api_error(
                StatusCode::CONFLICT,
                "approval is not pending, is expired, or does not exist",
            ));
        }
        return Ok(Json(json!({"ok":true,"shared":true})));
    }
    let mut approvals = state.approvals.lock().map_err(lock_error)?;
    let approval = approvals
        .get_mut(&input.approval_id)
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "approval not found"))?;
    if approval.state != ApprovalState::Pending {
        return Err(api_error(StatusCode::CONFLICT, "approval is not pending"));
    }
    approval.state = ApprovalState::Rejected;
    approval.approver_id = Some(principal.id);
    approval.decided_at_unix_ms = Some(vsn_remote::now_ms());
    drop(approvals);
    persist_state(&state)?;
    Ok(Json(json!({"ok":true,"shared":false})))
}

async fn list_central_audit(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    require_permission(&state, &headers, "control.audit.view")?;
    if let Some(store) = state.state_postgres.as_ref() {
        let rows = store.recent_audit(500).map_err(|e| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("shared audit list failed: {e}"),
            )
        })?;
        let mut events = Vec::new();
        for row in rows {
            let event: vsn_audit::AuditEvent = serde_json::from_str(&row.payload).map_err(|e| {
                api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("shared audit payload invalid: {e}"),
                )
            })?;
            events.push(event);
        }
        return Ok(Json(json!({"events":events,"shared":true})));
    }
    let events = state
        .central_audit
        .lock()
        .map_err(lock_error)?
        .iter()
        .rev()
        .take(500)
        .cloned()
        .collect::<Vec<_>>();
    Ok(Json(json!({"events":events,"shared":false})))
}

async fn validate_gateway(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    require_permission(&state, &headers, "control.audit.view")?;
    let mut issues = Vec::<String>::new();
    let public_endpoint = state.public_endpoint.as_str();
    if !public_endpoint.starts_with("https://")
        && !public_endpoint.starts_with("http://127.0.0.1")
        && !public_endpoint.starts_with("http://localhost")
    {
        issues.push("gateway public endpoint must use HTTPS outside loopback development".into());
    }
    let local_agents = state.agent_stream_peers.lock().await.len();
    let relays = state.stream_relays.lock().await.len();
    let shared = state.state_postgres.is_some();
    if shared {
        if let Some(store) = state.state_postgres.as_ref() {
            if store
                .route_owner("agent_stream", "__vsn_probe_missing__")
                .is_err()
            {
                issues.push("shared gateway route store is unavailable".into());
            }
        }
    }
    Ok(Json(
        json!({"ok":issues.is_empty(),"issues":issues,"protocol_version":vsn_remote::STREAM_RELAY_PROTOCOL_VERSION,"shared_postgres":shared,"local_agent_stream_peers":local_agents,"active_relays":relays,"cross_instance_bus":shared,"resume_checkpoints":shared,"supported_streams":["terminal","file_upload","file_download","database","preview_snapshot","preview_sse","preview_websocket"],"security":{"signed_open_authorization":true,"bounded_frames":true,"origin_validation":true,"device_local_opt_in":true,"backpressure":true,"replay_protection":true}}),
    ))
}

async fn cluster_status(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    require_permission(&state, &headers, "control.devices.view")?;
    let local_stream_devices = state
        .agent_stream_peers
        .lock()
        .await
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    let instances = if let Some(store) = state.state_postgres.as_ref() {
        store.live_instances().map_err(|e| {
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!("cluster store unavailable: {e}"),
            )
        })?
    } else {
        Vec::new()
    };
    Ok(Json(
        json!({"instance_id":state.instance_id.as_str(),"public_endpoint":state.public_endpoint.as_str(),"shared_postgres":state.state_postgres.is_some(),"instances":instances,"local_stream_devices":local_stream_devices,"note":"Agent and active browser sockets remain instance-local, while shared PostgreSQL provides route ownership, bounded cross-instance relay/command buses, and relay resume checkpoints/history. A reconnect can reload bounded resume metadata on another Control Plane node; live terminal process reconstruction after Agent loss is intentionally not automatic."}),
    ))
}

async fn validate_federation(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    refresh_shared_auth_state(&state)?;
    require_permission(&state, &headers, "control.auth.manage")?;
    let policy = state.auth_policy.lock().map_err(lock_error)?.clone();
    let mut issues = Vec::<String>::new();
    if let Err(e) = vsn_auth::validate_policy(&policy) {
        issues.push(e.to_string());
    }
    let accounts = state
        .accounts
        .lock()
        .map_err(lock_error)?
        .values()
        .cloned()
        .collect::<Vec<_>>();
    let mut oidc = std::collections::BTreeSet::new();
    let mut saml = std::collections::BTreeSet::new();
    for account in &accounts {
        for identity in &account.oidc_identities {
            if !oidc.insert((identity.provider_id.clone(), identity.subject.clone())) {
                issues.push(format!(
                    "duplicate OIDC subject mapping: {}:{}",
                    identity.provider_id, identity.subject
                ));
            }
            if !policy
                .oidc_providers
                .iter()
                .any(|p| p.id == identity.provider_id)
            {
                issues.push(format!(
                    "account {} references missing OIDC provider {}",
                    account.id, identity.provider_id
                ));
            }
        }
        for identity in &account.saml_identities {
            if !saml.insert((identity.provider_id.clone(), identity.subject.clone())) {
                issues.push(format!(
                    "duplicate SAML subject mapping: {}:{}",
                    identity.provider_id, identity.subject
                ));
            }
            if !policy
                .saml_providers
                .iter()
                .any(|p| p.id == identity.provider_id)
            {
                issues.push(format!(
                    "account {} references missing SAML provider {}",
                    account.id, identity.provider_id
                ));
            }
        }
    }
    let providers=policy.oidc_providers.iter().map(|p|json!({"id":p.id,"kind":"oidc","login":true,"explicit_mapping":true,"unlink":true,"local_logout":true,"provider_logout":p.end_session_endpoint.is_some()})).chain(policy.saml_providers.iter().map(|p|json!({"id":p.id,"kind":"saml","login":true,"explicit_mapping":true,"unlink":true,"local_logout":true,"provider_logout":p.slo_url.is_some()}))).collect::<Vec<_>>();
    Ok(Json(
        json!({"ok":issues.is_empty(),"issues":issues,"providers":providers,"oidc_mappings":oidc.len(),"saml_mappings":saml.len(),"passkeys_supported":state.webauthn.as_ref().is_some(),"scim_users_groups_bulk_patch_etag":true}),
    ))
}

async fn get_auth_policy(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<vsn_auth::EnterpriseAuthPolicy>, ApiError> {
    require_permission(&state, &headers, "control.auth.view")?;
    Ok(Json(state.auth_policy.lock().map_err(lock_error)?.clone()))
}
async fn set_auth_policy(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(policy): Json<vsn_auth::EnterpriseAuthPolicy>,
) -> Result<Json<vsn_auth::EnterpriseAuthPolicy>, ApiError> {
    require_permission(&state, &headers, "control.auth.manage")?;
    vsn_auth::validate_policy(&policy)
        .map_err(|e| api_error(StatusCode::BAD_REQUEST, &e.to_string()))?;
    *state.auth_policy.lock().map_err(lock_error)? = policy.clone();
    sync_shared_auth_policy(&state, &policy)?;
    persist_auth_state(&state)?;
    Ok(Json(policy))
}

const SCIM_USER_SCHEMA: &str = "urn:ietf:params:scim:schemas:core:2.0:User";
const SCIM_LIST_SCHEMA: &str = "urn:ietf:params:scim:api:messages:2.0:ListResponse";
const SCIM_BULK_SCHEMA: &str = "urn:ietf:params:scim:api:messages:2.0:BulkRequest";
const SCIM_BULK_RESPONSE_SCHEMA: &str = "urn:ietf:params:scim:api:messages:2.0:BulkResponse";
fn scim_version(value: &Value) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    let digest = Sha256::digest(bytes);
    format!(
        "\"{}\"",
        digest
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    )
}
fn scim_etag_headers(value: &Value) -> Result<HeaderMap, ApiError> {
    let version = value
        .get("meta")
        .and_then(|m| m.get("version"))
        .and_then(Value::as_str)
        .ok_or_else(|| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "SCIM resource version missing",
            )
        })?;
    let mut headers = HeaderMap::new();
    headers.insert(
        "etag",
        HeaderValue::from_str(version).map_err(|_| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "SCIM resource version is not a valid ETag",
            )
        })?,
    );
    Ok(headers)
}
fn scim_require_if_match(headers: &HeaderMap, current: &Value) -> Result<(), ApiError> {
    let Some(raw) = headers.get("if-match") else {
        return Ok(());
    };
    let actual = current
        .get("meta")
        .and_then(|m| m.get("version"))
        .and_then(Value::as_str)
        .ok_or_else(|| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "SCIM resource version missing",
            )
        })?;
    let supplied = raw
        .to_str()
        .map_err(|_| api_error(StatusCode::BAD_REQUEST, "invalid If-Match header"))?;
    if supplied != "*" && supplied != actual {
        return Err(api_error(
            StatusCode::PRECONDITION_FAILED,
            "SCIM resource version mismatch",
        ));
    }
    Ok(())
}
fn scim_user_json(state: &AppState, account: &AccountRecord) -> Value {
    let mut value = json!({"schemas":[SCIM_USER_SCHEMA],"id":account.id,"externalId":account.scim_external_id,"userName":account.email,"active":!account.disabled,"roles":[{"value":account.role_id}],"meta":{"resourceType":"User","created":account.created_at_unix_ms,"location":format!("{}/scim/v2/Users/{}",state.public_endpoint.trim_end_matches('/'),account.id)}});
    let version = scim_version(&value);
    if let Some(meta) = value.get_mut("meta").and_then(Value::as_object_mut) {
        meta.insert("version".into(), Value::String(version));
    }
    value
}
fn scim_role_for_request(
    state: &AppState,
    principal: &AuthPrincipal,
    input: &ScimUserInput,
) -> Result<IamRole, ApiError> {
    let role_id = input
        .roles
        .first()
        .map(|r| r.value.clone())
        .or_else(|| std::env::var("VSN_SCIM_DEFAULT_ROLE").ok())
        .ok_or_else(|| {
            api_error(
                StatusCode::BAD_REQUEST,
                "SCIM user requires one role or VSN_SCIM_DEFAULT_ROLE",
            )
        })?;
    validate_id(&role_id)?;
    let role = state
        .roles
        .lock()
        .map_err(lock_error)?
        .get(&role_id)
        .cloned()
        .ok_or_else(|| api_error(StatusCode::BAD_REQUEST, "SCIM role not found"))?;
    if !principal.bootstrap && role.permissions.iter().any(|p| !principal.allows(p)) {
        return Err(api_error(
            StatusCode::FORBIDDEN,
            "SCIM principal cannot assign a role broader than its delegated permissions",
        ));
    }
    Ok(role)
}
fn scim_parse_filter(filter: Option<&str>) -> Result<Option<String>, ApiError> {
    let Some(raw) = filter else { return Ok(None) };
    let raw = raw.trim();
    let prefix = "userName eq \"";
    if !raw.starts_with(prefix) || !raw.ends_with('"') {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "0.13 SCIM filter supports only userName eq \"value\"",
        ));
    }
    let value = &raw[prefix.len()..raw.len() - 1];
    Ok(Some(normalize_email(value)?))
}
async fn scim_service_provider_config(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    require_permission(&state, &headers, "control.scim.manage")?;
    Ok(Json(
        json!({"schemas":["urn:ietf:params:scim:schemas:core:2.0:ServiceProviderConfig"],"patch":{"supported":true},"bulk":{"supported":true,"maxOperations":100,"maxPayloadSize":1048576},"filter":{"supported":true,"maxResults":200},"changePassword":{"supported":false},"sort":{"supported":false},"etag":{"supported":true}}),
    ))
}
async fn scim_list_users(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ScimListQuery>,
) -> Result<Json<Value>, ApiError> {
    refresh_shared_auth_state(&state)?;
    require_permission(&state, &headers, "control.scim.manage")?;
    let filter = scim_parse_filter(q.filter.as_deref())?;
    let start = q.start_index.unwrap_or(1).max(1);
    let count = q.count.unwrap_or(100).clamp(1, 200);
    let mut users = state
        .accounts
        .lock()
        .map_err(lock_error)?
        .values()
        .filter(|a| filter.as_ref().map(|f| &a.email == f).unwrap_or(true))
        .cloned()
        .collect::<Vec<_>>();
    users.sort_by(|a, b| a.email.cmp(&b.email));
    let total = users.len();
    let resources = users
        .into_iter()
        .skip(start - 1)
        .take(count)
        .map(|a| scim_user_json(&state, &a))
        .collect::<Vec<_>>();
    Ok(Json(
        json!({"schemas":[SCIM_LIST_SCHEMA],"totalResults":total,"startIndex":start,"itemsPerPage":resources.len(),"Resources":resources}),
    ))
}
async fn scim_get_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
) -> Result<(HeaderMap, Json<Value>), ApiError> {
    refresh_shared_auth_state(&state)?;
    require_permission(&state, &headers, "control.scim.manage")?;
    validate_id(&id)?;
    let account = state
        .accounts
        .lock()
        .map_err(lock_error)?
        .get(&id)
        .cloned()
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "SCIM user not found"))?;
    let value = scim_user_json(&state, &account);
    Ok((scim_etag_headers(&value)?, Json(value)))
}
async fn scim_create_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<ScimUserInput>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    refresh_shared_auth_state(&state)?;
    let principal = require_permission(&state, &headers, "control.scim.manage")?;
    if !input.schemas.is_empty() && !input.schemas.iter().any(|s| s == SCIM_USER_SCHEMA) {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "SCIM User schema is required",
        ));
    }
    let email = normalize_email(&input.user_name)?;
    if input
        .external_id
        .as_deref()
        .map(|v| v.len() > 256 || v.chars().any(char::is_control))
        .unwrap_or(false)
    {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "invalid SCIM externalId",
        ));
    }
    let role = scim_role_for_request(&state, &principal, &input)?;
    let id = random_id("scimusr");
    let secret = random_id("scim-disabled-password");
    let password_hash = vsn_auth::hash_password(&secret)
        .map_err(|e| api_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;
    let record = AccountRecord {
        id: id.clone(),
        email: email.clone(),
        password_hash,
        role_id: role.id,
        created_at_unix_ms: vsn_remote::now_ms(),
        disabled: !input.active,
        totp_secret: None,
        last_totp_step: None,
        recovery_code_hashes: Vec::new(),
        passkeys: Vec::new(),
        oidc_identities: Vec::new(),
        saml_identities: Vec::new(),
        managed_by_scim: true,
        scim_external_id: input.external_id,
    };
    {
        let mut accounts = state.accounts.lock().map_err(lock_error)?;
        if accounts.values().any(|a| a.email == email) {
            return Err(api_error(
                StatusCode::CONFLICT,
                "SCIM userName already exists",
            ));
        }
        accounts.insert(id.clone(), record.clone());
    }
    sync_shared_account(&state, &record)?;
    persist_auth_state(&state)?;
    Ok((StatusCode::CREATED, Json(scim_user_json(&state, &record))))
}
async fn scim_replace_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
    Json(input): Json<ScimUserInput>,
) -> Result<Json<Value>, ApiError> {
    refresh_shared_auth_state(&state)?;
    let principal = require_permission(&state, &headers, "control.scim.manage")?;
    validate_id(&id)?;
    {
        let current = state
            .accounts
            .lock()
            .map_err(lock_error)?
            .get(&id)
            .cloned()
            .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "SCIM user not found"))?;
        scim_require_if_match(&headers, &scim_user_json(&state, &current))?;
    }
    let email = normalize_email(&input.user_name)?;
    let role = scim_role_for_request(&state, &principal, &input)?;
    {
        let accounts = state.accounts.lock().map_err(lock_error)?;
        if accounts.values().any(|a| a.id != id && a.email == email) {
            return Err(api_error(
                StatusCode::CONFLICT,
                "SCIM userName already exists",
            ));
        }
    }
    let updated = {
        let mut accounts = state.accounts.lock().map_err(lock_error)?;
        let account = accounts
            .get_mut(&id)
            .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "SCIM user not found"))?;
        account.email = email;
        account.role_id = role.id;
        account.disabled = !input.active;
        account.managed_by_scim = true;
        account.scim_external_id = input.external_id;
        account.clone()
    };
    sync_shared_account(&state, &updated)?;
    revoke_account_sessions_for(&state, &id)?;
    persist_auth_state(&state)?;
    Ok(Json(scim_user_json(&state, &updated)))
}
async fn scim_delete_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
) -> Result<StatusCode, ApiError> {
    refresh_shared_auth_state(&state)?;
    require_permission(&state, &headers, "control.scim.manage")?;
    validate_id(&id)?;
    {
        let current = state
            .accounts
            .lock()
            .map_err(lock_error)?
            .get(&id)
            .cloned()
            .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "SCIM user not found"))?;
        scim_require_if_match(&headers, &scim_user_json(&state, &current))?;
    }
    state
        .accounts
        .lock()
        .map_err(lock_error)?
        .remove(&id)
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "SCIM user not found"))?;
    delete_shared_account(&state, &id)?;
    revoke_account_sessions_for(&state, &id)?;
    state
        .sessions
        .lock()
        .map_err(lock_error)?
        .retain(|_, s| s.account_id != id);
    persist_auth_state(&state)?;
    Ok(StatusCode::NO_CONTENT)
}

const SCIM_GROUP_SCHEMA: &str = "urn:ietf:params:scim:schemas:core:2.0:Group";
const SCIM_PATCH_SCHEMA: &str = "urn:ietf:params:scim:api:messages:2.0:PatchOp";
fn scim_group_json(state: &AppState, group: &ScimGroupRecord) -> Value {
    let mut value = json!({"schemas":[SCIM_GROUP_SCHEMA],"id":group.id,"externalId":group.external_id,"displayName":group.display_name,"members":group.members.iter().map(|id|json!({"value":id,"$ref":format!("{}/scim/v2/Users/{id}",state.public_endpoint.trim_end_matches('/'))})).collect::<Vec<_>>(),"meta":{"resourceType":"Group","created":group.created_at_unix_ms,"location":format!("{}/scim/v2/Groups/{}",state.public_endpoint.trim_end_matches('/'),group.id)}});
    let version = scim_version(&value);
    if let Some(meta) = value.get_mut("meta").and_then(Value::as_object_mut) {
        meta.insert("version".into(), Value::String(version));
    }
    value
}
fn validate_scim_external_id(value: Option<&str>) -> Result<(), ApiError> {
    if value
        .map(|v| v.len() > 256 || v.chars().any(char::is_control))
        .unwrap_or(false)
    {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "invalid SCIM externalId",
        ));
    }
    Ok(())
}
fn validate_scim_group_members(
    state: &AppState,
    members: &[ScimMemberValue],
) -> Result<Vec<String>, ApiError> {
    if members.len() > 10_000 {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "SCIM group member limit exceeded",
        ));
    }
    let accounts = state.accounts.lock().map_err(lock_error)?;
    let mut seen = HashSet::new();
    let mut ids = Vec::new();
    for member in members {
        validate_id(&member.value)?;
        if !accounts.contains_key(&member.value) {
            return Err(api_error(
                StatusCode::BAD_REQUEST,
                "SCIM group references unknown user",
            ));
        }
        if seen.insert(member.value.clone()) {
            ids.push(member.value.clone());
        }
    }
    Ok(ids)
}
fn scim_group_input(state: &AppState, input: ScimGroupInput) -> Result<ScimGroupRecord, ApiError> {
    if !input.schemas.is_empty() && !input.schemas.iter().any(|s| s == SCIM_GROUP_SCHEMA) {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "SCIM Group schema is required",
        ));
    }
    let name = input.display_name.trim().to_string();
    if name.is_empty() || name.len() > 256 || name.chars().any(char::is_control) {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "invalid SCIM group displayName",
        ));
    }
    validate_scim_external_id(input.external_id.as_deref())?;
    let members = validate_scim_group_members(state, &input.members)?;
    Ok(ScimGroupRecord {
        id: random_id("scimgrp"),
        display_name: name,
        external_id: input.external_id,
        members,
        created_at_unix_ms: vsn_remote::now_ms(),
    })
}
async fn scim_list_groups(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ScimListQuery>,
) -> Result<Json<Value>, ApiError> {
    refresh_shared_auth_state(&state)?;
    require_permission(&state, &headers, "control.scim.manage")?;
    let start = q.start_index.unwrap_or(1).max(1);
    let count = q.count.unwrap_or(100).clamp(1, 200);
    let mut groups = state
        .scim_groups
        .lock()
        .map_err(lock_error)?
        .values()
        .cloned()
        .collect::<Vec<_>>();
    groups.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    let total = groups.len();
    let resources = groups
        .into_iter()
        .skip(start - 1)
        .take(count)
        .map(|g| scim_group_json(&state, &g))
        .collect::<Vec<_>>();
    Ok(Json(
        json!({"schemas":[SCIM_LIST_SCHEMA],"totalResults":total,"startIndex":start,"itemsPerPage":resources.len(),"Resources":resources}),
    ))
}
async fn scim_get_group(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
) -> Result<(HeaderMap, Json<Value>), ApiError> {
    refresh_shared_auth_state(&state)?;
    require_permission(&state, &headers, "control.scim.manage")?;
    validate_id(&id)?;
    let group = state
        .scim_groups
        .lock()
        .map_err(lock_error)?
        .get(&id)
        .cloned()
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "SCIM group not found"))?;
    let value = scim_group_json(&state, &group);
    Ok((scim_etag_headers(&value)?, Json(value)))
}
async fn scim_create_group(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<ScimGroupInput>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    refresh_shared_auth_state(&state)?;
    require_permission(&state, &headers, "control.scim.manage")?;
    let group = scim_group_input(&state, input)?;
    {
        let groups = state.scim_groups.lock().map_err(lock_error)?;
        if groups
            .values()
            .any(|g| g.display_name == group.display_name)
        {
            return Err(api_error(
                StatusCode::CONFLICT,
                "SCIM group displayName already exists",
            ));
        }
    }
    state
        .scim_groups
        .lock()
        .map_err(lock_error)?
        .insert(group.id.clone(), group.clone());
    sync_shared_scim_group(&state, &group)?;
    persist_auth_state(&state)?;
    Ok((StatusCode::CREATED, Json(scim_group_json(&state, &group))))
}
async fn scim_replace_group(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
    Json(input): Json<ScimGroupInput>,
) -> Result<Json<Value>, ApiError> {
    refresh_shared_auth_state(&state)?;
    require_permission(&state, &headers, "control.scim.manage")?;
    validate_id(&id)?;
    {
        let current = state
            .scim_groups
            .lock()
            .map_err(lock_error)?
            .get(&id)
            .cloned()
            .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "SCIM group not found"))?;
        scim_require_if_match(&headers, &scim_group_json(&state, &current))?;
    }
    let mut replacement = scim_group_input(&state, input)?;
    {
        let groups = state.scim_groups.lock().map_err(lock_error)?;
        if groups
            .values()
            .any(|g| g.id != id && g.display_name == replacement.display_name)
        {
            return Err(api_error(
                StatusCode::CONFLICT,
                "SCIM group displayName already exists",
            ));
        }
        replacement.created_at_unix_ms = groups
            .get(&id)
            .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "SCIM group not found"))?
            .created_at_unix_ms;
    }
    replacement.id = id.clone();
    state
        .scim_groups
        .lock()
        .map_err(lock_error)?
        .insert(id, replacement.clone());
    sync_shared_scim_group(&state, &replacement)?;
    persist_auth_state(&state)?;
    Ok(Json(scim_group_json(&state, &replacement)))
}
async fn scim_delete_group(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
) -> Result<StatusCode, ApiError> {
    refresh_shared_auth_state(&state)?;
    require_permission(&state, &headers, "control.scim.manage")?;
    validate_id(&id)?;
    {
        let current = state
            .scim_groups
            .lock()
            .map_err(lock_error)?
            .get(&id)
            .cloned()
            .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "SCIM group not found"))?;
        scim_require_if_match(&headers, &scim_group_json(&state, &current))?;
    }
    state
        .scim_groups
        .lock()
        .map_err(lock_error)?
        .remove(&id)
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "SCIM group not found"))?;
    delete_shared_scim_group(&state, &id)?;
    persist_auth_state(&state)?;
    Ok(StatusCode::NO_CONTENT)
}
fn scim_patch_validate(input: &ScimPatchRequest) -> Result<(), ApiError> {
    if !input.schemas.is_empty() && !input.schemas.iter().any(|v| v == SCIM_PATCH_SCHEMA) {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "SCIM PatchOp schema is required",
        ));
    }
    if input.operations.is_empty() || input.operations.len() > 64 {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "SCIM PATCH must contain 1..64 operations",
        ));
    }
    Ok(())
}
fn scim_patch_role_value(value: &Value) -> Option<String> {
    if let Some(s) = value.as_str() {
        return Some(s.to_string());
    }
    if let Some(obj) = value.as_object() {
        if let Some(s) = obj.get("value").and_then(Value::as_str) {
            return Some(s.to_string());
        }
    }
    value
        .as_array()
        .and_then(|a| a.first())
        .and_then(|v| v.get("value"))
        .and_then(Value::as_str)
        .map(str::to_string)
}
async fn scim_patch_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
    Json(input): Json<ScimPatchRequest>,
) -> Result<Json<Value>, ApiError> {
    refresh_shared_auth_state(&state)?;
    let principal = require_permission(&state, &headers, "control.scim.manage")?;
    validate_id(&id)?;
    {
        let current = state
            .accounts
            .lock()
            .map_err(lock_error)?
            .get(&id)
            .cloned()
            .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "SCIM user not found"))?;
        scim_require_if_match(&headers, &scim_user_json(&state, &current))?;
    }
    scim_patch_validate(&input)?;
    let mut account = state
        .accounts
        .lock()
        .map_err(lock_error)?
        .get(&id)
        .cloned()
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "SCIM user not found"))?;
    let mut security_changed = false;
    for op in input.operations {
        let action = op.op.to_ascii_lowercase();
        let path = op.path.unwrap_or_default().to_ascii_lowercase();
        match (action.as_str(), path.as_str()) {
            ("replace" | "add", "active") => {
                account.disabled = !op.value.as_bool().ok_or_else(|| {
                    api_error(StatusCode::BAD_REQUEST, "SCIM active must be boolean")
                })?;
                security_changed = true;
            }
            ("replace" | "add", "username") => {
                account.email = normalize_email(op.value.as_str().ok_or_else(|| {
                    api_error(StatusCode::BAD_REQUEST, "SCIM userName must be string")
                })?)?;
                security_changed = true;
            }
            ("replace" | "add", "externalid") => {
                let v = op.value.as_str().map(str::to_string);
                validate_scim_external_id(v.as_deref())?;
                account.scim_external_id = v;
            }
            ("remove", "externalid") => account.scim_external_id = None,
            ("replace" | "add", "roles") => {
                let role_id = scim_patch_role_value(&op.value).ok_or_else(|| {
                    api_error(
                        StatusCode::BAD_REQUEST,
                        "SCIM roles patch requires a role value",
                    )
                })?;
                validate_id(&role_id)?;
                let role = state
                    .roles
                    .lock()
                    .map_err(lock_error)?
                    .get(&role_id)
                    .cloned()
                    .ok_or_else(|| api_error(StatusCode::BAD_REQUEST, "SCIM role not found"))?;
                if !principal.bootstrap && role.permissions.iter().any(|p| !principal.allows(p)) {
                    return Err(api_error(
                        StatusCode::FORBIDDEN,
                        "SCIM principal cannot assign broader role",
                    ));
                }
                account.role_id = role.id;
                security_changed = true;
            }
            _ => {
                return Err(api_error(
                    StatusCode::BAD_REQUEST,
                    "unsupported SCIM User PATCH operation/path",
                ))
            }
        }
    }
    {
        let accounts = state.accounts.lock().map_err(lock_error)?;
        if accounts
            .values()
            .any(|a| a.id != id && a.email == account.email)
        {
            return Err(api_error(
                StatusCode::CONFLICT,
                "SCIM userName already exists",
            ));
        }
    }
    account.managed_by_scim = true;
    state
        .accounts
        .lock()
        .map_err(lock_error)?
        .insert(id.clone(), account.clone());
    sync_shared_account(&state, &account)?;
    if security_changed {
        revoke_account_sessions_for(&state, &id)?;
    }
    persist_auth_state(&state)?;
    Ok(Json(scim_user_json(&state, &account)))
}
async fn scim_patch_group(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
    Json(input): Json<ScimPatchRequest>,
) -> Result<Json<Value>, ApiError> {
    refresh_shared_auth_state(&state)?;
    require_permission(&state, &headers, "control.scim.manage")?;
    validate_id(&id)?;
    {
        let current = state
            .scim_groups
            .lock()
            .map_err(lock_error)?
            .get(&id)
            .cloned()
            .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "SCIM group not found"))?;
        scim_require_if_match(&headers, &scim_group_json(&state, &current))?;
    }
    scim_patch_validate(&input)?;
    let mut group = state
        .scim_groups
        .lock()
        .map_err(lock_error)?
        .get(&id)
        .cloned()
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "SCIM group not found"))?;
    for op in input.operations {
        let action = op.op.to_ascii_lowercase();
        let path = op.path.unwrap_or_default().to_ascii_lowercase();
        match (action.as_str(), path.as_str()) {
            ("replace" | "add", "displayname") => {
                let name = op
                    .value
                    .as_str()
                    .ok_or_else(|| {
                        api_error(StatusCode::BAD_REQUEST, "SCIM displayName must be string")
                    })?
                    .trim()
                    .to_string();
                if name.is_empty() || name.len() > 256 || name.chars().any(char::is_control) {
                    return Err(api_error(
                        StatusCode::BAD_REQUEST,
                        "invalid SCIM group displayName",
                    ));
                }
                group.display_name = name;
            }
            ("replace" | "add", "externalid") => {
                let v = op.value.as_str().map(str::to_string);
                validate_scim_external_id(v.as_deref())?;
                group.external_id = v;
            }
            ("remove", "externalid") => group.external_id = None,
            ("replace", "members") => {
                let values: Vec<ScimMemberValue> =
                    serde_json::from_value(op.value).map_err(|_| {
                        api_error(
                            StatusCode::BAD_REQUEST,
                            "SCIM members must be an array of {value}",
                        )
                    })?;
                group.members = validate_scim_group_members(&state, &values)?;
            }
            ("add", "members") => {
                let values: Vec<ScimMemberValue> = if op.value.is_array() {
                    serde_json::from_value(op.value).map_err(|_| {
                        api_error(
                            StatusCode::BAD_REQUEST,
                            "SCIM members must be an array of {value}",
                        )
                    })?
                } else {
                    vec![serde_json::from_value(op.value).map_err(|_| {
                        api_error(StatusCode::BAD_REQUEST, "SCIM member must be {value}")
                    })?]
                };
                let additions = validate_scim_group_members(&state, &values)?;
                for member in additions {
                    if !group.members.contains(&member) {
                        group.members.push(member);
                    }
                }
            }
            ("remove", "members") => {
                let values: Vec<ScimMemberValue> = if op.value.is_array() {
                    serde_json::from_value(op.value).map_err(|_| {
                        api_error(StatusCode::BAD_REQUEST, "SCIM members remove value invalid")
                    })?
                } else {
                    vec![serde_json::from_value(op.value).map_err(|_| {
                        api_error(StatusCode::BAD_REQUEST, "SCIM member remove value invalid")
                    })?]
                };
                let remove: HashSet<_> = values.into_iter().map(|v| v.value).collect();
                group.members.retain(|m| !remove.contains(m));
            }
            _ => {
                return Err(api_error(
                    StatusCode::BAD_REQUEST,
                    "unsupported SCIM Group PATCH operation/path",
                ))
            }
        }
    }
    {
        let groups = state.scim_groups.lock().map_err(lock_error)?;
        if groups
            .values()
            .any(|g| g.id != id && g.display_name == group.display_name)
        {
            return Err(api_error(
                StatusCode::CONFLICT,
                "SCIM group displayName already exists",
            ));
        }
    }
    state
        .scim_groups
        .lock()
        .map_err(lock_error)?
        .insert(id.clone(), group.clone());
    sync_shared_scim_group(&state, &group)?;
    persist_auth_state(&state)?;
    Ok(Json(scim_group_json(&state, &group)))
}

fn scim_bulk_path(path: &str) -> Result<(&str, Option<&str>), ApiError> {
    let clean = path.trim().trim_start_matches('/');
    let mut parts = clean.split('/');
    let resource = parts
        .next()
        .ok_or_else(|| api_error(StatusCode::BAD_REQUEST, "SCIM Bulk path missing"))?;
    let id = parts.next();
    if parts.next().is_some() || !matches!(resource, "Users" | "Groups") {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "SCIM Bulk path must be Users[/id] or Groups[/id]",
        ));
    }
    Ok((resource, id))
}
async fn scim_bulk(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<ScimBulkRequest>,
) -> Result<Json<Value>, ApiError> {
    refresh_shared_auth_state(&state)?;
    require_permission(&state, &headers, "control.scim.manage")?;
    if !input.schemas.is_empty() && !input.schemas.iter().any(|s| s == SCIM_BULK_SCHEMA) {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "SCIM BulkRequest schema is required",
        ));
    }
    if input.operations.is_empty() || input.operations.len() > 100 {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "SCIM Bulk requires 1..100 operations",
        ));
    }
    let encoded = serde_json::to_vec(&input.operations)
        .map_err(|e| api_error(StatusCode::BAD_REQUEST, &e.to_string()))?;
    if encoded.len() > 1024 * 1024 {
        return Err(api_error(
            StatusCode::PAYLOAD_TOO_LARGE,
            "SCIM Bulk payload exceeds 1 MiB",
        ));
    }
    let fail_on = input.fail_on_errors.unwrap_or(0);
    let mut failures = 0u32;
    let mut out = Vec::new();
    for op in input.operations {
        let method = op.method.to_ascii_uppercase();
        let bulk_id = op.bulk_id.clone();
        let mut op_headers = headers.clone();
        if let Some(version) = op.version.as_ref() {
            let hv = HeaderValue::from_str(version).map_err(|_| {
                api_error(StatusCode::BAD_REQUEST, "invalid SCIM Bulk version value")
            })?;
            op_headers.insert(axum::http::header::IF_MATCH, hv);
        }
        let result: Result<(StatusCode, Option<Value>, Option<String>), ApiError> = async {
            let (resource, id) = scim_bulk_path(&op.path)?;
            match (method.as_str(), resource, id) {
                ("POST", "Users", None) => {
                    let req: ScimUserInput = serde_json::from_value(op.data).map_err(|_| {
                        api_error(StatusCode::BAD_REQUEST, "invalid SCIM Bulk User payload")
                    })?;
                    let (status, Json(v)) =
                        scim_create_user(State(state.clone()), op_headers.clone(), Json(req))
                            .await?;
                    let location = v
                        .get("meta")
                        .and_then(|m| m.get("location"))
                        .and_then(Value::as_str)
                        .map(str::to_string);
                    Ok((status, Some(v), location))
                }
                ("POST", "Groups", None) => {
                    let req: ScimGroupInput = serde_json::from_value(op.data).map_err(|_| {
                        api_error(StatusCode::BAD_REQUEST, "invalid SCIM Bulk Group payload")
                    })?;
                    let (status, Json(v)) =
                        scim_create_group(State(state.clone()), op_headers.clone(), Json(req))
                            .await?;
                    let location = v
                        .get("meta")
                        .and_then(|m| m.get("location"))
                        .and_then(Value::as_str)
                        .map(str::to_string);
                    Ok((status, Some(v), location))
                }
                ("PUT", "Users", Some(id)) => {
                    let req: ScimUserInput = serde_json::from_value(op.data).map_err(|_| {
                        api_error(StatusCode::BAD_REQUEST, "invalid SCIM Bulk User payload")
                    })?;
                    let Json(v) = scim_replace_user(
                        State(state.clone()),
                        op_headers.clone(),
                        AxumPath(id.to_string()),
                        Json(req),
                    )
                    .await?;
                    Ok((StatusCode::OK, Some(v), None))
                }
                ("PUT", "Groups", Some(id)) => {
                    let req: ScimGroupInput = serde_json::from_value(op.data).map_err(|_| {
                        api_error(StatusCode::BAD_REQUEST, "invalid SCIM Bulk Group payload")
                    })?;
                    let Json(v) = scim_replace_group(
                        State(state.clone()),
                        op_headers.clone(),
                        AxumPath(id.to_string()),
                        Json(req),
                    )
                    .await?;
                    Ok((StatusCode::OK, Some(v), None))
                }
                ("PATCH", "Users", Some(id)) => {
                    let req: ScimPatchRequest = serde_json::from_value(op.data).map_err(|_| {
                        api_error(
                            StatusCode::BAD_REQUEST,
                            "invalid SCIM Bulk User PATCH payload",
                        )
                    })?;
                    let Json(v) = scim_patch_user(
                        State(state.clone()),
                        op_headers.clone(),
                        AxumPath(id.to_string()),
                        Json(req),
                    )
                    .await?;
                    Ok((StatusCode::OK, Some(v), None))
                }
                ("PATCH", "Groups", Some(id)) => {
                    let req: ScimPatchRequest = serde_json::from_value(op.data).map_err(|_| {
                        api_error(
                            StatusCode::BAD_REQUEST,
                            "invalid SCIM Bulk Group PATCH payload",
                        )
                    })?;
                    let Json(v) = scim_patch_group(
                        State(state.clone()),
                        op_headers.clone(),
                        AxumPath(id.to_string()),
                        Json(req),
                    )
                    .await?;
                    Ok((StatusCode::OK, Some(v), None))
                }
                ("DELETE", "Users", Some(id)) => {
                    let status = scim_delete_user(
                        State(state.clone()),
                        op_headers.clone(),
                        AxumPath(id.to_string()),
                    )
                    .await?;
                    Ok((status, None, None))
                }
                ("DELETE", "Groups", Some(id)) => {
                    let status = scim_delete_group(
                        State(state.clone()),
                        op_headers.clone(),
                        AxumPath(id.to_string()),
                    )
                    .await?;
                    Ok((status, None, None))
                }
                _ => Err(api_error(
                    StatusCode::BAD_REQUEST,
                    "unsupported SCIM Bulk method/path",
                )),
            }
        }
        .await;
        match result {
            Ok((status, value, location)) => {
                let version = value
                    .as_ref()
                    .and_then(|v| v.get("meta"))
                    .and_then(|m| m.get("version"))
                    .cloned();
                out.push(json!({"method":method,"bulkId":bulk_id,"status":status.as_u16().to_string(),"location":location,"version":version,"response":value}));
            }
            Err(error) => {
                failures = failures.saturating_add(1);
                out.push(json!({"method":method,"bulkId":bulk_id,"status":error.0.as_u16().to_string(),"response":{"schemas":["urn:ietf:params:scim:api:messages:2.0:Error"],"status":error.0.as_u16().to_string(),"detail":api_error_message(&error)}}));
                if fail_on > 0 && failures >= fail_on {
                    break;
                }
            }
        }
    }
    Ok(Json(
        json!({"schemas":[SCIM_BULK_RESPONSE_SCHEMA],"Operations":out}),
    ))
}
async fn scim_reconcile(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<ScimReconcileRequest>,
) -> Result<Json<Value>, ApiError> {
    refresh_shared_auth_state(&state)?;
    require_permission(&state, &headers, "control.scim.manage")?;
    let account_ids = state
        .accounts
        .lock()
        .map_err(lock_error)?
        .keys()
        .cloned()
        .collect::<HashSet<_>>();
    let mut dangling = Vec::new();
    let mut duplicate_external = Vec::new();
    let mut external_seen: HashMap<String, String> = HashMap::new();
    {
        let accounts = state.accounts.lock().map_err(lock_error)?;
        for a in accounts.values().filter(|a| a.managed_by_scim) {
            if let Some(ext) = a.scim_external_id.as_ref() {
                if let Some(first) = external_seen.insert(ext.clone(), a.id.clone()) {
                    duplicate_external
                        .push(json!({"externalId":ext,"first":first,"duplicate":a.id}));
                }
            }
        }
    }
    {
        let groups = state.scim_groups.lock().map_err(lock_error)?;
        for g in groups.values() {
            for member in &g.members {
                if !account_ids.contains(member) {
                    dangling.push(json!({"group_id":g.id,"member_id":member}));
                }
            }
            if let Some(ext) = g.external_id.as_ref() {
                let key = format!("group:{ext}");
                if let Some(first) = external_seen.insert(key.clone(), g.id.clone()) {
                    duplicate_external.push(
                        json!({"externalId":ext,"first":first,"duplicate":g.id,"resource":"Group"}),
                    );
                }
            }
        }
    }
    let mut repaired = 0usize;
    if input.repair && !dangling.is_empty() {
        let mut groups = state.scim_groups.lock().map_err(lock_error)?;
        for group in groups.values_mut() {
            let before = group.members.len();
            group.members.retain(|m| account_ids.contains(m));
            if group.members.len() != before {
                repaired += before - group.members.len();
                sync_shared_scim_group(&state, group)?;
            }
        }
        drop(groups);
        persist_auth_state(&state)?;
    }
    Ok(Json(
        json!({"dangling_memberships":dangling,"duplicate_external_ids":duplicate_external,"repaired_memberships":repaired,"repair_mode":input.repair,"note":"repair only removes dangling memberships; VSN never guesses identity or role mappings"}),
    ))
}

async fn list_accounts(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    refresh_shared_auth_state(&state)?;
    require_permission(&state, &headers, "control.auth.manage")?;
    let accounts=state.accounts.lock().map_err(lock_error)?.values().map(|a|json!({"id":a.id,"email":a.email,"role_id":a.role_id,"created_at_unix_ms":a.created_at_unix_ms,"disabled":a.disabled,"totp_enabled":a.totp_secret.is_some(),"recovery_codes_remaining":a.recovery_code_hashes.len(),"passkeys":a.passkeys.len(),"oidc_identities":a.oidc_identities.len(),"saml_identities":a.saml_identities.len(),"managed_by_scim":a.managed_by_scim,"scim_external_id":a.scim_external_id})).collect::<Vec<_>>();
    Ok(Json(json!({"accounts":accounts})))
}
async fn create_account(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<CreateAccountRequest>,
) -> Result<Json<Value>, ApiError> {
    let principal = require_permission(&state, &headers, "control.auth.manage")?;
    validate_id(&input.id)?;
    let email = normalize_email(&input.email)?;
    let role = state
        .roles
        .lock()
        .map_err(lock_error)?
        .get(&input.role_id)
        .cloned()
        .ok_or_else(|| api_error(StatusCode::BAD_REQUEST, "role not found"))?;
    if !principal.bootstrap && role.permissions.iter().any(|p| !principal.allows(p)) {
        return Err(api_error(
            StatusCode::FORBIDDEN,
            "scoped principal cannot create an account with broader role permissions",
        ));
    }
    let password_hash = vsn_auth::hash_password(&input.password)
        .map_err(|e| api_error(StatusCode::BAD_REQUEST, &e.to_string()))?;
    let mut accounts = state.accounts.lock().map_err(lock_error)?;
    if accounts.contains_key(&input.id) || accounts.values().any(|a| a.email == email) {
        return Err(api_error(
            StatusCode::CONFLICT,
            "account id or email already exists",
        ));
    }
    let record = AccountRecord {
        id: input.id,
        email,
        password_hash,
        role_id: input.role_id,
        created_at_unix_ms: vsn_remote::now_ms(),
        disabled: false,
        totp_secret: None,
        last_totp_step: None,
        recovery_code_hashes: Vec::new(),
        passkeys: Vec::new(),
        oidc_identities: Vec::new(),
        saml_identities: Vec::new(),
        managed_by_scim: false,
        scim_external_id: None,
    };
    let response = json!({"id":record.id,"email":record.email,"role_id":record.role_id,"created_at_unix_ms":record.created_at_unix_ms,"disabled":record.disabled,"totp_enabled":false,"recovery_codes_remaining":0});
    accounts.insert(record.id.clone(), record.clone());
    drop(accounts);
    sync_shared_account(&state, &record)?;
    persist_auth_state(&state)?;
    Ok(Json(response))
}
async fn update_account(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<UpdateAccountRequest>,
) -> Result<Json<Value>, ApiError> {
    let principal = require_permission(&state, &headers, "control.auth.manage")?;
    validate_id(&input.account_id)?;
    let new_role = if let Some(role_id) = input.role_id.as_ref() {
        let role = state
            .roles
            .lock()
            .map_err(lock_error)?
            .get(role_id)
            .cloned()
            .ok_or_else(|| api_error(StatusCode::BAD_REQUEST, "role not found"))?;
        if !principal.bootstrap && role.permissions.iter().any(|p| !principal.allows(p)) {
            return Err(api_error(
                StatusCode::FORBIDDEN,
                "scoped principal cannot assign broader role permissions",
            ));
        }
        Some(role.id)
    } else {
        None
    };
    let new_password = if let Some(password) = input.password.as_ref() {
        Some(
            vsn_auth::hash_password(password)
                .map_err(|e| api_error(StatusCode::BAD_REQUEST, &e.to_string()))?,
        )
    } else {
        None
    };
    let security_changed = input.disabled.is_some()
        || new_password.is_some()
        || new_role.is_some()
        || input.clear_totp;
    let response = {
        let mut accounts = state.accounts.lock().map_err(lock_error)?;
        let account = accounts
            .get_mut(&input.account_id)
            .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "account not found"))?;
        if let Some(v) = input.disabled {
            account.disabled = v;
        }
        if let Some(v) = new_password {
            account.password_hash = v;
        }
        if let Some(v) = new_role {
            account.role_id = v;
        }
        if input.clear_totp {
            account.totp_secret = None;
            account.last_totp_step = None;
        }
        json!({"id":account.id,"email":account.email,"role_id":account.role_id,"disabled":account.disabled,"totp_enabled":account.totp_secret.is_some()})
    };
    let updated = state
        .accounts
        .lock()
        .map_err(lock_error)?
        .get(&input.account_id)
        .cloned()
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "account not found after update"))?;
    sync_shared_account(&state, &updated)?;
    if security_changed {
        revoke_account_sessions_for(&state, &input.account_id)?;
    }
    persist_auth_state(&state)?;
    Ok(Json(response))
}
async fn enroll_account_totp(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<TotpEnrollmentRequest>,
) -> Result<Json<Value>, ApiError> {
    require_permission(&state, &headers, "control.auth.manage")?;
    let key = state.auth_encryption_key.as_ref().as_ref().ok_or_else(|| {
        api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "VSN_CONTROL_AUTH_KEY_B64 is required for TOTP enrollment",
        )
    })?;
    let (account_id, email) = {
        let accounts = state.accounts.lock().map_err(lock_error)?;
        let a = accounts
            .get(&input.account_id)
            .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "account not found"))?;
        (a.id.clone(), a.email.clone())
    };
    let enrollment = vsn_auth::create_totp_enrollment(&email, "VSN Control Plane")
        .map_err(|e| api_error(StatusCode::BAD_REQUEST, &e.to_string()))?;
    let encrypted = encrypt_auth_secret(key, enrollment.secret_base32.as_bytes())?;
    {
        let mut accounts = state.accounts.lock().map_err(lock_error)?;
        let account = accounts
            .get_mut(&account_id)
            .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "account not found"))?;
        account.totp_secret = Some(encrypted);
        account.last_totp_step = None;
    }
    let updated = state
        .accounts
        .lock()
        .map_err(lock_error)?
        .get(&account_id)
        .cloned()
        .ok_or_else(|| {
            api_error(
                StatusCode::NOT_FOUND,
                "account not found after TOTP enrollment",
            )
        })?;
    sync_shared_account(&state, &updated)?;
    revoke_account_sessions_for(&state, &account_id)?;
    persist_auth_state(&state)?;
    Ok(Json(
        json!({"account_id":account_id,"otpauth_url":enrollment.otpauth_url,"secret_base32":enrollment.secret_base32,"digits":enrollment.digits,"step_seconds":enrollment.step_seconds,"warning":"display once; store in authenticator and do not log"}),
    ))
}
async fn regenerate_account_recovery_codes(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<RecoveryCodesRequest>,
) -> Result<Json<Value>, ApiError> {
    require_permission(&state, &headers, "control.auth.manage")?;
    validate_id(&input.account_id)?;
    {
        let accounts = state.accounts.lock().map_err(lock_error)?;
        if !accounts.contains_key(&input.account_id) {
            return Err(api_error(StatusCode::NOT_FOUND, "account not found"));
        }
    }
    let generated = vsn_auth::create_recovery_codes(10)
        .map_err(|e| api_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;
    {
        let mut accounts = state.accounts.lock().map_err(lock_error)?;
        let account = accounts
            .get_mut(&input.account_id)
            .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "account not found"))?;
        account.recovery_code_hashes = generated.hashes;
    }
    let updated = state
        .accounts
        .lock()
        .map_err(lock_error)?
        .get(&input.account_id)
        .cloned()
        .ok_or_else(|| {
            api_error(
                StatusCode::NOT_FOUND,
                "account not found after recovery-code generation",
            )
        })?;
    sync_shared_account(&state, &updated)?;
    revoke_account_sessions_for(&state, &input.account_id)?;
    persist_auth_state(&state)?;
    Ok(Json(
        json!({"account_id":input.account_id,"recovery_codes":generated.codes,"warning":"display once; each code is single-use and only hashes are stored"}),
    ))
}

fn store_passkey_owner(
    state: &AppState,
    transaction_id: &str,
    kind: &str,
    account_id: &str,
    expires_at_unix_ms: u128,
) -> Result<(), ApiError> {
    if let Some(store) = state.state_postgres.as_ref() {
        let payload=serde_json::to_string(&json!({"account_id":account_id,"owner_instance_id":state.instance_id.as_str(),"owner_endpoint":state.public_endpoint.as_str()})).map_err(|e|api_error(StatusCode::INTERNAL_SERVER_ERROR,&e.to_string()))?;
        store
            .put_auth_transaction(&vsn_control_store::SharedAuthTransaction {
                transaction_id: transaction_id.into(),
                kind: kind.into(),
                payload,
                created_at_unix_ms: vsn_remote::now_ms(),
                expires_at_unix_ms,
                consumed_at_unix_ms: None,
            })
            .map_err(|e| {
                api_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    &format!("shared passkey transaction owner write failed: {e}"),
                )
            })?;
    }
    Ok(())
}
fn passkey_owner_mismatch(
    state: &AppState,
    transaction_id: &str,
    kind: &str,
) -> Result<Option<ApiError>, ApiError> {
    let Some(store) = state.state_postgres.as_ref() else {
        return Ok(None);
    };
    let Some(record) = store.auth_transaction(transaction_id, kind).map_err(|e| {
        api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            &format!("shared passkey owner lookup failed: {e}"),
        )
    })?
    else {
        return Ok(None);
    };
    let payload: Value = serde_json::from_str(&record.payload).map_err(|e| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("shared passkey owner payload invalid: {e}"),
        )
    })?;
    let owner = payload
        .get("owner_instance_id")
        .and_then(Value::as_str)
        .unwrap_or("");
    if owner == state.instance_id.as_str() {
        return Ok(None);
    };
    Ok(Some((
        StatusCode::CONFLICT,
        Json(
            json!({"error":"passkey ceremony belongs to another Control Plane instance","owner_instance_id":owner,"owner_endpoint":payload.get("owner_endpoint"),"retry":"send the finish request to owner_endpoint before the ceremony expires"}),
        ),
    )))
}
fn consume_passkey_owner(state: &AppState, transaction_id: &str, kind: &str) {
    if let Some(store) = state.state_postgres.as_ref() {
        let _ = store.consume_auth_transaction(transaction_id, kind, vsn_remote::now_ms());
    }
}

async fn passkey_owner_lookup(
    State(state): State<AppState>,
    AxumPath(transaction_id): AxumPath<String>,
    Query(query): Query<PasskeyOwnerQuery>,
) -> Result<Json<Value>, ApiError> {
    validate_id(&transaction_id)?;
    let kind = match query.kind.as_str() {
        "registration" => "webauthn_registration",
        "authentication" => "webauthn_authentication",
        _ => {
            return Err(api_error(
                StatusCode::BAD_REQUEST,
                "kind must be registration or authentication",
            ))
        }
    };
    check_rate_limit(
        &state,
        &format!("passkey-owner:{}", hash_token(&transaction_id)),
        30,
        60_000,
    )?;
    let Some(store) = state.state_postgres.as_ref() else {
        return Ok(Json(
            json!({"transaction_id":transaction_id,"owner_instance_id":state.instance_id.as_str(),"owner_endpoint":state.public_endpoint.as_str(),"shared":false}),
        ));
    };
    let record = store
        .auth_transaction(&transaction_id, kind)
        .map_err(|e| {
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!("shared passkey owner lookup failed: {e}"),
            )
        })?
        .ok_or_else(|| {
            api_error(
                StatusCode::NOT_FOUND,
                "passkey transaction not found or expired",
            )
        })?;
    let payload: Value = serde_json::from_str(&record.payload).map_err(|e| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("shared passkey owner payload invalid: {e}"),
        )
    })?;
    Ok(Json(
        json!({"transaction_id":transaction_id,"owner_instance_id":payload.get("owner_instance_id"),"owner_endpoint":payload.get("owner_endpoint"),"expires_at_unix_ms":record.expires_at_unix_ms,"shared":true}),
    ))
}
async fn passkey_register_begin(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let principal = authenticate_account_session(&state, &headers, true)?;
    check_rate_limit(&state, &format!("passkey-reg:{}", principal.id), 20, 60_000)?;
    let webauthn = state.webauthn.as_ref().as_ref().ok_or_else(|| {
        api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "WebAuthn is not configured",
        )
    })?;
    let account = state
        .accounts
        .lock()
        .map_err(lock_error)?
        .get(&principal.id)
        .cloned()
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "account not found"))?;
    if account.disabled {
        return Err(api_error(StatusCode::FORBIDDEN, "account disabled"));
    }
    if account.passkeys.len() >= 16 {
        return Err(api_error(StatusCode::CONFLICT, "passkey limit reached"));
    }
    let exclude = if account.passkeys.is_empty() {
        None
    } else {
        Some(
            account
                .passkeys
                .iter()
                .map(|p| p.cred_id().clone())
                .collect(),
        )
    };
    let user_id = account_webauthn_uuid(&account.id);
    let (challenge, registration) = webauthn
        .start_passkey_registration(user_id, &account.email, &account.email, exclude)
        .map_err(|e| {
            api_error(
                StatusCode::BAD_REQUEST,
                &format!("passkey registration start failed: {e}"),
            )
        })?;
    let tx_id = random_id("pkreg");
    let now = vsn_remote::now_ms();
    {
        let mut pending = state.passkey_registrations.lock().map_err(lock_error)?;
        pending.retain(|_, v| v.expires_at_unix_ms >= now);
        if pending.len() >= 4096 {
            return Err(api_error(
                StatusCode::TOO_MANY_REQUESTS,
                "too many pending passkey registrations",
            ));
        }
        pending.insert(
            tx_id.clone(),
            PendingPasskeyRegistration {
                account_id: account.id.clone(),
                state: registration,
                expires_at_unix_ms: now + 5 * 60 * 1000,
            },
        );
    }
    store_passkey_owner(
        &state,
        &tx_id,
        "webauthn_registration",
        &account.id,
        now + 5 * 60 * 1000,
    )?;
    Ok(Json(
        json!({"transaction_id":tx_id,"challenge":challenge,"expires_at_unix_ms":now+5*60*1000,"owner_instance_id":state.instance_id.as_str(),"owner_endpoint":state.public_endpoint.as_str()}),
    ))
}
async fn passkey_register_finish(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<PasskeyRegisterFinishRequest>,
) -> Result<Json<Value>, ApiError> {
    let principal = authenticate_account_session(&state, &headers, true)?;
    validate_id(&input.transaction_id)?;
    let now = vsn_remote::now_ms();
    let pending = match state
        .passkey_registrations
        .lock()
        .map_err(lock_error)?
        .remove(&input.transaction_id)
    {
        Some(v) => v,
        None => {
            if let Some(error) =
                passkey_owner_mismatch(&state, &input.transaction_id, "webauthn_registration")?
            {
                return Err(error);
            }
            return Err(api_error(
                StatusCode::NOT_FOUND,
                "passkey registration transaction not found or owning instance restarted",
            ));
        }
    };
    if pending.account_id != principal.id {
        return Err(api_error(
            StatusCode::FORBIDDEN,
            "passkey transaction belongs to another account",
        ));
    }
    if pending.expires_at_unix_ms < now {
        return Err(api_error(
            StatusCode::GONE,
            "passkey registration transaction expired",
        ));
    }
    let webauthn = state.webauthn.as_ref().as_ref().ok_or_else(|| {
        api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "WebAuthn is not configured",
        )
    })?;
    let passkey = webauthn
        .finish_passkey_registration(&input.credential, &pending.state)
        .map_err(|e| {
            api_error(
                StatusCode::UNAUTHORIZED,
                &format!("passkey registration failed: {e}"),
            )
        })?;
    {
        let mut accounts = state.accounts.lock().map_err(lock_error)?;
        if accounts
            .values()
            .any(|a| a.passkeys.iter().any(|p| p.cred_id() == passkey.cred_id()))
        {
            return Err(api_error(
                StatusCode::CONFLICT,
                "credential is already registered",
            ));
        }
        let account = accounts
            .get_mut(&principal.id)
            .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "account not found"))?;
        account.passkeys.push(passkey);
    }
    let updated = state
        .accounts
        .lock()
        .map_err(lock_error)?
        .get(&principal.id)
        .cloned()
        .ok_or_else(|| {
            api_error(
                StatusCode::NOT_FOUND,
                "account not found after passkey registration",
            )
        })?;
    consume_passkey_owner(&state, &input.transaction_id, "webauthn_registration");
    sync_shared_account(&state, &updated)?;
    persist_auth_state(&state)?;
    Ok(Json(json!({"ok":true,"account_id":principal.id})))
}
async fn passkey_login_begin(
    State(state): State<AppState>,
    Json(input): Json<PasskeyLoginBeginRequest>,
) -> Result<Json<Value>, ApiError> {
    refresh_shared_auth_state(&state)?;
    let email = normalize_email(&input.email)?;
    check_rate_limit(
        &state,
        &format!("passkey-login:{}", hash_token(&email)),
        20,
        60_000,
    )?;
    let webauthn = state.webauthn.as_ref().as_ref().ok_or_else(|| {
        api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "WebAuthn is not configured",
        )
    })?;
    let account = state
        .accounts
        .lock()
        .map_err(lock_error)?
        .values()
        .find(|a| a.email == email && !a.disabled)
        .cloned()
        .ok_or_else(|| api_error(StatusCode::UNAUTHORIZED, "passkey login unavailable"))?;
    if account.passkeys.is_empty() {
        return Err(api_error(
            StatusCode::UNAUTHORIZED,
            "passkey login unavailable",
        ));
    }
    let (challenge, authentication) = webauthn
        .start_passkey_authentication(&account.passkeys)
        .map_err(|e| {
            api_error(
                StatusCode::BAD_REQUEST,
                &format!("passkey authentication start failed: {e}"),
            )
        })?;
    let tx_id = random_id("pkauth");
    let now = vsn_remote::now_ms();
    {
        let mut pending = state.passkey_authentications.lock().map_err(lock_error)?;
        pending.retain(|_, v| v.expires_at_unix_ms >= now);
        if pending.len() >= 4096 {
            return Err(api_error(
                StatusCode::TOO_MANY_REQUESTS,
                "too many pending passkey authentications",
            ));
        }
        pending.insert(
            tx_id.clone(),
            PendingPasskeyAuthentication {
                account_id: account.id.clone(),
                state: authentication,
                expires_at_unix_ms: now + 5 * 60 * 1000,
            },
        );
    }
    store_passkey_owner(
        &state,
        &tx_id,
        "webauthn_authentication",
        &account.id,
        now + 5 * 60 * 1000,
    )?;
    Ok(Json(
        json!({"transaction_id":tx_id,"challenge":challenge,"expires_at_unix_ms":now+5*60*1000,"owner_instance_id":state.instance_id.as_str(),"owner_endpoint":state.public_endpoint.as_str()}),
    ))
}
async fn passkey_login_finish(
    State(state): State<AppState>,
    Json(input): Json<PasskeyLoginFinishRequest>,
) -> Result<Json<Value>, ApiError> {
    refresh_shared_auth_state(&state)?;
    validate_id(&input.transaction_id)?;
    let now = vsn_remote::now_ms();
    let pending = match state
        .passkey_authentications
        .lock()
        .map_err(lock_error)?
        .remove(&input.transaction_id)
    {
        Some(v) => v,
        None => {
            if let Some(error) =
                passkey_owner_mismatch(&state, &input.transaction_id, "webauthn_authentication")?
            {
                return Err(error);
            }
            return Err(api_error(
                StatusCode::NOT_FOUND,
                "passkey authentication transaction not found or owning instance restarted",
            ));
        }
    };
    if pending.expires_at_unix_ms < now {
        return Err(api_error(
            StatusCode::GONE,
            "passkey authentication transaction expired",
        ));
    }
    let webauthn = state.webauthn.as_ref().as_ref().ok_or_else(|| {
        api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "WebAuthn is not configured",
        )
    })?;
    let result = webauthn
        .finish_passkey_authentication(&input.credential, &pending.state)
        .map_err(|e| {
            api_error(
                StatusCode::UNAUTHORIZED,
                &format!("passkey authentication failed: {e}"),
            )
        })?;
    consume_passkey_owner(&state, &input.transaction_id, "webauthn_authentication");
    let account = {
        let mut accounts = state.accounts.lock().map_err(lock_error)?;
        let account = accounts
            .get_mut(&pending.account_id)
            .ok_or_else(|| api_error(StatusCode::UNAUTHORIZED, "account not found"))?;
        if account.disabled {
            return Err(api_error(StatusCode::UNAUTHORIZED, "account disabled"));
        }
        let mut matched = false;
        for passkey in &mut account.passkeys {
            if passkey.update_credential(&result).is_some() {
                matched = true;
                break;
            }
        }
        if !matched {
            return Err(api_error(
                StatusCode::CONFLICT,
                "authenticated credential is not associated with this account",
            ));
        }
        account.clone()
    };
    sync_shared_account(&state, &account)?;
    let role = state
        .roles
        .lock()
        .map_err(lock_error)?
        .get(&account.role_id)
        .cloned()
        .ok_or_else(|| api_error(StatusCode::UNAUTHORIZED, "account role no longer exists"))?;
    let policy = state.auth_policy.lock().map_err(lock_error)?.clone();
    let raw = random_id("vsnsess");
    let session = AccountSessionRecord {
        id: random_id("sess"),
        account_id: account.id.clone(),
        token_hash: hash_token(&raw),
        created_at_unix_ms: now,
        expires_at_unix_ms: now + (policy.session_ttl_minutes as u128) * 60_000,
        last_activity_unix_ms: now,
        mfa_verified: true,
        passkey_verified: true,
        revoked: false,
        federated: None,
    };
    store_account_session(&state, &session)?;
    cleanup_sessions(&state)?;
    persist_auth_state(&state)?;
    Ok(Json(
        json!({"session_token":raw,"session":{"id":session.id,"account_id":session.account_id,"expires_at_unix_ms":session.expires_at_unix_ms,"mfa_verified":true,"passkey_verified":true},"role_id":role.id}),
    ))
}

async fn oidc_begin(
    State(state): State<AppState>,
    Json(input): Json<OidcBeginRequest>,
) -> Result<Json<Value>, ApiError> {
    refresh_shared_auth_state(&state)?;
    check_rate_limit(&state, "public:oidc-begin", 120, 60_000)?;
    validate_id(&input.provider_id)?;
    let provider = {
        let policy = state.auth_policy.lock().map_err(lock_error)?;
        policy
            .oidc_providers
            .iter()
            .find(|p| p.id == input.provider_id)
            .cloned()
            .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "OIDC provider not configured"))?
    };
    vsn_auth::validate_oidc(&provider)
        .map_err(|e| api_error(StatusCode::BAD_REQUEST, &e.to_string()))?;
    let endpoint = provider.authorization_endpoint.clone().ok_or_else(|| {
        api_error(
            StatusCode::BAD_REQUEST,
            "OIDC authorization_endpoint is required before browser authorization can begin",
        )
    })?;
    let tx = vsn_auth::create_oidc_pkce_transaction()
        .map_err(|e| api_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;
    let now = vsn_remote::now_ms();
    let pending_record = PendingOidcTransaction {
        provider_id: provider.id.clone(),
        transaction: tx.clone(),
    };
    if state.state_postgres.is_some() {
        store_oidc_transaction(&state, &pending_record)?;
    } else {
        {
            let mut pending = state.oidc_transactions.lock().map_err(lock_error)?;
            pending.retain(|_, v| {
                now.saturating_sub(v.transaction.created_at_unix_ms) <= 10 * 60 * 1000
            });
            if pending.len() >= 4096 {
                return Err(api_error(
                    StatusCode::TOO_MANY_REQUESTS,
                    "too many pending OIDC transactions",
                ));
            }
            pending.insert(tx.state.clone(), pending_record);
        }
    }
    let scope = provider.scopes.join(" ");
    let query=format!("response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&nonce={}&code_challenge={}&code_challenge_method=S256",pct(&provider.client_id),pct(&provider.redirect_url),pct(&scope),pct(&tx.state),pct(&tx.nonce),pct(&tx.code_challenge));
    let separator = if endpoint.contains('?') { "&" } else { "?" };
    Ok(Json(
        json!({"provider_id":provider.id,"authorization_url":format!("{endpoint}{separator}{query}"),"expires_at_unix_ms":tx.created_at_unix_ms+10*60*1000}),
    ))
}

async fn link_oidc_identity(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<OidcLinkRequest>,
) -> Result<Json<Value>, ApiError> {
    require_permission(&state, &headers, "control.auth.manage")?;
    validate_id(&input.account_id)?;
    validate_id(&input.provider_id)?;
    if input.subject.is_empty()
        || input.subject.len() > 512
        || input.subject.chars().any(char::is_control)
    {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "OIDC subject is invalid",
        ));
    }
    {
        let policy = state.auth_policy.lock().map_err(lock_error)?;
        if !policy
            .oidc_providers
            .iter()
            .any(|p| p.id == input.provider_id)
        {
            return Err(api_error(
                StatusCode::NOT_FOUND,
                "OIDC provider not configured",
            ));
        }
    }
    let mut accounts = state.accounts.lock().map_err(lock_error)?;
    if accounts.values().any(|a| {
        a.oidc_identities
            .iter()
            .any(|i| i.provider_id == input.provider_id && i.subject == input.subject)
    }) {
        return Err(api_error(
            StatusCode::CONFLICT,
            "OIDC identity is already linked",
        ));
    }
    let account = accounts
        .get_mut(&input.account_id)
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "account not found"))?;
    if account.oidc_identities.len() >= 32 {
        return Err(api_error(
            StatusCode::CONFLICT,
            "OIDC identity limit reached",
        ));
    }
    account.oidc_identities.push(OidcIdentity {
        provider_id: input.provider_id.clone(),
        subject: input.subject.clone(),
    });
    let updated = account.clone();
    drop(accounts);
    sync_shared_account(&state, &updated)?;
    persist_auth_state(&state)?;
    Ok(Json(
        json!({"ok":true,"account_id":input.account_id,"provider_id":input.provider_id,"subject":input.subject}),
    ))
}

async fn unlink_oidc_identity(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<FederationUnlinkRequest>,
) -> Result<Json<Value>, ApiError> {
    refresh_shared_auth_state(&state)?;
    require_permission(&state, &headers, "control.auth.manage")?;
    validate_id(&input.account_id)?;
    validate_id(&input.provider_id)?;
    let updated = {
        let mut accounts = state.accounts.lock().map_err(lock_error)?;
        let account = accounts
            .get_mut(&input.account_id)
            .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "account not found"))?;
        let before = account.oidc_identities.len();
        account
            .oidc_identities
            .retain(|i| !(i.provider_id == input.provider_id && i.subject == input.subject));
        if account.oidc_identities.len() == before {
            return Err(api_error(
                StatusCode::NOT_FOUND,
                "OIDC identity link not found",
            ));
        }
        account.clone()
    };
    sync_shared_account(&state, &updated)?;
    revoke_account_sessions_for(&state, &input.account_id)?;
    persist_auth_state(&state)?;
    Ok(Json(
        json!({"ok":true,"account_id":input.account_id,"provider_id":input.provider_id,"subject":input.subject,"sessions_revoked":true}),
    ))
}

async fn saml_begin(
    State(state): State<AppState>,
    Json(input): Json<SamlBeginRequest>,
) -> Result<Json<Value>, ApiError> {
    refresh_shared_auth_state(&state)?;
    check_rate_limit(&state, "public:saml-begin", 120, 60_000)?;
    validate_id(&input.provider_id)?;
    let provider = {
        let policy = state.auth_policy.lock().map_err(lock_error)?;
        policy
            .saml_providers
            .iter()
            .find(|p| p.id == input.provider_id)
            .cloned()
            .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "SAML provider not configured"))?
    };
    let start = vsn_saml::create_login_start(&provider, 10 * 60 * 1000)
        .map_err(|e| api_error(StatusCode::BAD_REQUEST, &e.to_string()))?;
    if let Some(store) = state.state_postgres.as_ref() {
        let payload = serde_json::to_string(&start.transaction).map_err(|e| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("SAML transaction serialization failed: {e}"),
            )
        })?;
        store
            .put_auth_transaction(&vsn_control_store::SharedAuthTransaction {
                transaction_id: start.transaction.relay_state.clone(),
                kind: "saml".into(),
                payload,
                created_at_unix_ms: start.transaction.created_at_unix_ms,
                expires_at_unix_ms: start.transaction.expires_at_unix_ms,
                consumed_at_unix_ms: None,
            })
            .map_err(|e| {
                api_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    &format!("shared SAML transaction write failed: {e}"),
                )
            })?;
    } else {
        let now = vsn_remote::now_ms();
        let mut pending = state.saml_transactions.lock().map_err(lock_error)?;
        pending.retain(|_, v| v.expires_at_unix_ms >= now);
        if pending.len() >= 4096 {
            return Err(api_error(
                StatusCode::TOO_MANY_REQUESTS,
                "too many pending SAML transactions",
            ));
        }
        pending.insert(
            start.transaction.relay_state.clone(),
            start.transaction.clone(),
        );
    }
    Ok(Json(
        json!({"provider_id":provider.id,"redirect_url":start.redirect_url,"relay_state":start.transaction.relay_state,"expires_at_unix_ms":start.transaction.expires_at_unix_ms}),
    ))
}
fn take_saml_transaction(
    state: &AppState,
    relay_state: &str,
) -> Result<Option<vsn_saml::SamlLoginTransaction>, ApiError> {
    if let Some(store) = state.state_postgres.as_ref() {
        let Some(record) = store
            .consume_auth_transaction(relay_state, "saml", vsn_remote::now_ms())
            .map_err(|e| {
                api_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    &format!("shared SAML transaction consume failed: {e}"),
                )
            })?
        else {
            return Ok(None);
        };
        let tx = serde_json::from_str(&record.payload).map_err(|e| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("shared SAML transaction payload invalid: {e}"),
            )
        })?;
        return Ok(Some(tx));
    }
    Ok(state
        .saml_transactions
        .lock()
        .map_err(lock_error)?
        .remove(relay_state))
}
async fn saml_acs(
    State(state): State<AppState>,
    Form(input): Form<SamlAcsForm>,
) -> Result<Json<Value>, ApiError> {
    refresh_shared_auth_state(&state)?;
    check_rate_limit(&state, "public:saml-acs", 120, 60_000)?;
    if input.relay_state.len() < 16 || input.relay_state.len() > 256 {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "invalid SAML RelayState",
        ));
    }
    let transaction = take_saml_transaction(&state, &input.relay_state)?.ok_or_else(|| {
        api_error(
            StatusCode::UNAUTHORIZED,
            "SAML transaction not found, expired, or already consumed",
        )
    })?;
    let provider = {
        let policy = state.auth_policy.lock().map_err(lock_error)?;
        policy
            .saml_providers
            .iter()
            .find(|p| p.id == transaction.provider_id)
            .cloned()
            .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "SAML provider no longer configured"))?
    };
    let assertion = vsn_saml::verify_acs_response(
        &provider,
        &transaction,
        &input.saml_response,
        &input.relay_state,
    )
    .map_err(|e| {
        api_error(
            StatusCode::UNAUTHORIZED,
            &format!("SAML assertion verification failed: {e}"),
        )
    })?;
    let account = {
        let accounts = state.accounts.lock().map_err(lock_error)?;
        accounts
            .values()
            .find(|a| {
                !a.disabled
                    && a.saml_identities
                        .iter()
                        .any(|i| i.provider_id == provider.id && i.subject == assertion.subject)
            })
            .cloned()
    };
    let Some(account) = account else {
        return Ok(Json(
            json!({"status":"mapping_required","provider_id":provider.id,"subject":assertion.subject,"email":assertion.email,"message":"Verified SAML identity is not linked to a VSN account; an administrator must explicitly link provider_id + subject."}),
        ));
    };
    let role = state
        .roles
        .lock()
        .map_err(lock_error)?
        .get(&account.role_id)
        .cloned()
        .ok_or_else(|| api_error(StatusCode::UNAUTHORIZED, "account role no longer exists"))?;
    let policy = state.auth_policy.lock().map_err(lock_error)?.clone();
    let now = vsn_remote::now_ms();
    let raw = random_id("vsnsess");
    let session = AccountSessionRecord {
        id: random_id("sess"),
        account_id: account.id.clone(),
        token_hash: hash_token(&raw),
        created_at_unix_ms: now,
        expires_at_unix_ms: now + (policy.session_ttl_minutes as u128) * 60_000,
        last_activity_unix_ms: now,
        mfa_verified: provider.mfa_assured,
        passkey_verified: false,
        revoked: false,
        federated: Some(FederatedSessionContext {
            kind: "saml".into(),
            provider_id: provider.id.clone(),
            subject: assertion.subject.clone(),
            session_index: assertion.session_index.clone(),
        }),
    };
    if role.permissions.iter().any(|p| {
        p == "*"
            || p == "control.iam.manage"
            || p == "control.auth.manage"
            || p == "control.scim.manage"
    }) && policy.require_mfa_for_admin
        && !session.mfa_verified
    {
        return Err(api_error(
            StatusCode::FORBIDDEN,
            "SAML provider is not configured as MFA-assured for this administrative account",
        ));
    }
    store_account_session(&state, &session)?;
    cleanup_sessions(&state)?;
    persist_auth_state(&state)?;
    Ok(Json(
        json!({"status":"authenticated","session_token":raw,"session":{"id":session.id,"account_id":session.account_id,"expires_at_unix_ms":session.expires_at_unix_ms,"mfa_verified":session.mfa_verified},"role_id":role.id,"provider_id":provider.id,"authn_context":assertion.authn_context}),
    ))
}
async fn link_saml_identity(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<SamlLinkRequest>,
) -> Result<Json<Value>, ApiError> {
    refresh_shared_auth_state(&state)?;
    require_permission(&state, &headers, "control.auth.manage")?;
    validate_id(&input.account_id)?;
    validate_id(&input.provider_id)?;
    if input.subject.is_empty()
        || input.subject.len() > 1024
        || input.subject.chars().any(char::is_control)
    {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "SAML subject is invalid",
        ));
    }
    {
        let policy = state.auth_policy.lock().map_err(lock_error)?;
        if !policy
            .saml_providers
            .iter()
            .any(|p| p.id == input.provider_id)
        {
            return Err(api_error(
                StatusCode::NOT_FOUND,
                "SAML provider not configured",
            ));
        }
    }
    let mut accounts = state.accounts.lock().map_err(lock_error)?;
    if accounts.values().any(|a| {
        a.saml_identities
            .iter()
            .any(|i| i.provider_id == input.provider_id && i.subject == input.subject)
    }) {
        return Err(api_error(
            StatusCode::CONFLICT,
            "SAML identity is already linked",
        ));
    }
    let account = accounts
        .get_mut(&input.account_id)
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "account not found"))?;
    if account.saml_identities.len() >= 32 {
        return Err(api_error(
            StatusCode::CONFLICT,
            "SAML identity limit reached",
        ));
    }
    account.saml_identities.push(SamlIdentity {
        provider_id: input.provider_id.clone(),
        subject: input.subject.clone(),
    });
    let updated = account.clone();
    drop(accounts);
    sync_shared_account(&state, &updated)?;
    persist_auth_state(&state)?;
    Ok(Json(
        json!({"ok":true,"account_id":input.account_id,"provider_id":input.provider_id,"subject":input.subject}),
    ))
}

async fn unlink_saml_identity(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<FederationUnlinkRequest>,
) -> Result<Json<Value>, ApiError> {
    refresh_shared_auth_state(&state)?;
    require_permission(&state, &headers, "control.auth.manage")?;
    validate_id(&input.account_id)?;
    validate_id(&input.provider_id)?;
    let updated = {
        let mut accounts = state.accounts.lock().map_err(lock_error)?;
        let account = accounts
            .get_mut(&input.account_id)
            .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "account not found"))?;
        let before = account.saml_identities.len();
        account
            .saml_identities
            .retain(|i| !(i.provider_id == input.provider_id && i.subject == input.subject));
        if account.saml_identities.len() == before {
            return Err(api_error(
                StatusCode::NOT_FOUND,
                "SAML identity link not found",
            ));
        }
        account.clone()
    };
    sync_shared_account(&state, &updated)?;
    revoke_account_sessions_for(&state, &input.account_id)?;
    persist_auth_state(&state)?;
    Ok(Json(
        json!({"ok":true,"account_id":input.account_id,"provider_id":input.provider_id,"subject":input.subject,"sessions_revoked":true}),
    ))
}
async fn federated_logout(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<FederatedLogoutRequest>,
) -> Result<Json<Value>, ApiError> {
    refresh_shared_auth_state(&state)?;
    let supplied = bearer_token(&headers)?;
    let hash = hash_token(supplied);
    let session = if let Some(store) = state.state_postgres.as_ref() {
        let shared = store
            .session_by_token_hash(&hash)
            .map_err(|e| {
                api_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    &format!("shared session lookup failed: {e}"),
                )
            })?
            .ok_or_else(|| api_error(StatusCode::UNAUTHORIZED, "invalid session"))?;
        serde_json::from_str::<AccountSessionRecord>(&shared.payload).map_err(|e| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("shared session payload invalid: {e}"),
            )
        })?
    } else {
        state
            .sessions
            .lock()
            .map_err(lock_error)?
            .values()
            .find(|s| !s.revoked && constant_time_eq(s.token_hash.as_bytes(), hash.as_bytes()))
            .cloned()
            .ok_or_else(|| api_error(StatusCode::UNAUTHORIZED, "invalid session"))?
    };
    if let Some(expected) = input.session_id.as_ref() {
        if &session.id != expected {
            return Err(api_error(StatusCode::FORBIDDEN, "session id mismatch"));
        }
    }
    revoke_account_session_by_id(&state, &session.id)?;
    persist_auth_state(&state)?;
    let Some(ctx) = session.federated else {
        return Ok(Json(
            json!({"ok":true,"session_id":session.id,"provider_logout":null,"federated":false}),
        ));
    };
    let policy = state.auth_policy.lock().map_err(lock_error)?.clone();
    let provider_logout = match ctx.kind.as_str() {
        "oidc" => policy
            .oidc_providers
            .iter()
            .find(|p| p.id == ctx.provider_id)
            .and_then(|p| {
                p.end_session_endpoint.as_ref().map(|endpoint| {
                    let mut url = endpoint.clone();
                    let sep = if url.contains('?') { "&" } else { "?" };
                    url.push_str(sep);
                    url.push_str(&format!("client_id={}", pct(&p.client_id)));
                    if let Some(redir) = &p.post_logout_redirect_url {
                        url.push_str("&post_logout_redirect_uri=");
                        url.push_str(&pct(redir));
                    }
                    url
                })
            }),
        "saml" => {
            let p = policy
                .saml_providers
                .iter()
                .find(|p| p.id == ctx.provider_id);
            match p {
                Some(provider) if provider.slo_url.is_some() => vsn_saml::create_logout_start(
                    provider,
                    &ctx.subject,
                    ctx.session_index.as_deref(),
                )
                .ok()
                .map(|v| v.redirect_url),
                _ => None,
            }
        }
        _ => None,
    };
    Ok(Json(
        json!({"ok":true,"session_id":session.id,"federated":true,"provider_kind":ctx.kind,"provider_id":ctx.provider_id,"provider_logout":provider_logout}),
    ))
}

async fn oidc_callback(
    State(state): State<AppState>,
    Query(query): Query<OidcCallbackQuery>,
) -> Result<Json<Value>, ApiError> {
    refresh_shared_auth_state(&state)?;
    if query.state.len() < 16 || query.state.len() > 256 {
        return Err(api_error(StatusCode::BAD_REQUEST, "invalid OIDC state"));
    }
    check_rate_limit(&state, "public:oidc-callback", 120, 60_000)?;
    let pending = take_oidc_transaction(&state, &query.state)?.ok_or_else(|| {
        api_error(
            StatusCode::UNAUTHORIZED,
            "OIDC transaction not found, expired, or already consumed",
        )
    })?;
    let now = vsn_remote::now_ms();
    vsn_auth::validate_oidc_transaction(&pending.transaction, &query.state, now, 10 * 60 * 1000)
        .map_err(|e| api_error(StatusCode::UNAUTHORIZED, &e.to_string()))?;
    if let Some(error) = query.error {
        return Err(api_error(
            StatusCode::UNAUTHORIZED,
            &format!(
                "OIDC provider returned {error}: {}",
                query.error_description.unwrap_or_default()
            ),
        ));
    }
    let code = query
        .code
        .filter(|v| !v.is_empty() && v.len() <= 4096)
        .ok_or_else(|| api_error(StatusCode::BAD_REQUEST, "OIDC authorization code missing"))?;
    let provider = {
        let policy = state.auth_policy.lock().map_err(lock_error)?;
        policy
            .oidc_providers
            .iter()
            .find(|p| p.id == pending.provider_id)
            .cloned()
            .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "OIDC provider no longer configured"))?
    };
    vsn_auth::validate_oidc(&provider)
        .map_err(|e| api_error(StatusCode::BAD_REQUEST, &e.to_string()))?;
    let http_client = openidconnect::reqwest::ClientBuilder::new()
        .redirect(openidconnect::reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| {
            api_error(
                StatusCode::BAD_GATEWAY,
                &format!("OIDC HTTP client failed: {e}"),
            )
        })?;
    let issuer = IssuerUrl::new(provider.issuer.clone()).map_err(|e| {
        api_error(
            StatusCode::BAD_REQUEST,
            &format!("OIDC issuer invalid: {e}"),
        )
    })?;
    let metadata = CoreProviderMetadata::discover_async(issuer, &http_client)
        .await
        .map_err(|e| {
            api_error(
                StatusCode::BAD_GATEWAY,
                &format!("OIDC discovery failed: {e}"),
            )
        })?;
    let secret = match provider.client_secret_env.as_deref() {
        Some(name) => Some(ClientSecret::new(std::env::var(name).map_err(|_| {
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "OIDC client secret environment variable is not configured",
            )
        })?)),
        None => None,
    };
    let client = CoreClient::from_provider_metadata(
        metadata,
        ClientId::new(provider.client_id.clone()),
        secret,
    )
    .set_redirect_uri(
        RedirectUrl::new(provider.redirect_url.clone()).map_err(|e| {
            api_error(
                StatusCode::BAD_REQUEST,
                &format!("OIDC redirect URL invalid: {e}"),
            )
        })?,
    );
    let response = client
        .exchange_code(AuthorizationCode::new(code))
        .map_err(|e| {
            api_error(
                StatusCode::BAD_GATEWAY,
                &format!("OIDC token endpoint unavailable: {e}"),
            )
        })?
        .set_pkce_verifier(PkceCodeVerifier::new(
            pending.transaction.code_verifier.clone(),
        ))
        .request_async(&http_client)
        .await
        .map_err(|e| {
            api_error(
                StatusCode::BAD_GATEWAY,
                &format!("OIDC code exchange failed: {e}"),
            )
        })?;
    let id_token = response.id_token().ok_or_else(|| {
        api_error(
            StatusCode::UNAUTHORIZED,
            "OIDC provider did not return an ID token",
        )
    })?;
    let verifier = client.id_token_verifier();
    let claims = id_token
        .claims(
            &verifier,
            &OidcNonce::new(pending.transaction.nonce.clone()),
        )
        .map_err(|e| {
            api_error(
                StatusCode::UNAUTHORIZED,
                &format!("OIDC ID token verification failed: {e}"),
            )
        })?;
    let subject = claims.subject().as_str().to_string();
    if subject.is_empty() || subject.len() > 512 {
        return Err(api_error(
            StatusCode::UNAUTHORIZED,
            "OIDC subject is invalid",
        ));
    }
    let email = claims.email().map(|v| v.as_str().to_string());
    let email_verified = claims.email_verified().unwrap_or(false);
    let account = {
        let accounts = state.accounts.lock().map_err(lock_error)?;
        accounts
            .values()
            .find(|a| {
                !a.disabled
                    && a.oidc_identities
                        .iter()
                        .any(|i| i.provider_id == provider.id && i.subject == subject)
            })
            .cloned()
    };
    let Some(account) = account else {
        return Ok(Json(
            json!({"status":"mapping_required","provider_id":provider.id,"subject":subject,"email":email,"email_verified":email_verified,"message":"Validated OIDC identity is not linked to a VSN account; an administrator must explicitly link provider_id + subject."}),
        ));
    };
    let role = state
        .roles
        .lock()
        .map_err(lock_error)?
        .get(&account.role_id)
        .cloned()
        .ok_or_else(|| api_error(StatusCode::UNAUTHORIZED, "account role no longer exists"))?;
    let policy = state.auth_policy.lock().map_err(lock_error)?.clone();
    let raw = random_id("vsnsess");
    let session = AccountSessionRecord {
        id: random_id("sess"),
        account_id: account.id.clone(),
        token_hash: hash_token(&raw),
        created_at_unix_ms: now,
        expires_at_unix_ms: now + (policy.session_ttl_minutes as u128) * 60_000,
        last_activity_unix_ms: now,
        mfa_verified: provider.mfa_assured,
        passkey_verified: false,
        revoked: false,
        federated: Some(FederatedSessionContext {
            kind: "oidc".into(),
            provider_id: provider.id.clone(),
            subject: subject.clone(),
            session_index: None,
        }),
    };
    if role.permissions.iter().any(|p| {
        p == "*"
            || p == "control.iam.manage"
            || p == "control.auth.manage"
            || p == "control.scim.manage"
    }) && policy.require_mfa_for_admin
        && !session.mfa_verified
    {
        return Err(api_error(
            StatusCode::FORBIDDEN,
            "OIDC provider is not configured as MFA-assured for this administrative account",
        ));
    }
    store_account_session(&state, &session)?;
    cleanup_sessions(&state)?;
    persist_auth_state(&state)?;
    Ok(Json(
        json!({"status":"authenticated","session_token":raw,"session":{"id":session.id,"account_id":session.account_id,"expires_at_unix_ms":session.expires_at_unix_ms,"mfa_verified":session.mfa_verified},"role_id":role.id,"provider_id":provider.id}),
    ))
}

async fn account_login(
    State(state): State<AppState>,
    Json(input): Json<LoginRequest>,
) -> Result<Json<Value>, ApiError> {
    refresh_shared_auth_state(&state)?;
    let email = normalize_email(&input.email)?;
    check_rate_limit(&state, &format!("login:{}", hash_token(&email)), 12, 60_000)?;
    let snapshot = {
        let accounts = state.accounts.lock().map_err(lock_error)?;
        accounts.values().find(|a| a.email == email).cloned()
    };
    let candidate_hash = snapshot
        .as_ref()
        .map(|a| a.password_hash.as_str())
        .unwrap_or_else(|| dummy_password_hash());
    let password_ok = vsn_auth::verify_password(&input.password, candidate_hash)
        .map_err(|e| api_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;
    let account =
        snapshot.ok_or_else(|| api_error(StatusCode::UNAUTHORIZED, "invalid credentials"))?;
    if !password_ok || account.disabled {
        return Err(api_error(StatusCode::UNAUTHORIZED, "invalid credentials"));
    }
    let mut mfa_verified = false;
    let mut matched_step = None;
    let mut recovery_index = None;
    if account.totp_secret.is_some() || !account.recovery_code_hashes.is_empty() {
        if let Some(code) = input.recovery_code.as_deref() {
            let matched = vsn_auth::match_recovery_code(code, &account.recovery_code_hashes)
                .map_err(|e| api_error(StatusCode::UNAUTHORIZED, &e.to_string()))?;
            let index = matched
                .ok_or_else(|| api_error(StatusCode::UNAUTHORIZED, "invalid recovery code"))?;
            mfa_verified = true;
            recovery_index = Some(index);
        } else if let Some(secret) = account.totp_secret.as_ref() {
            let key = state.auth_encryption_key.as_ref().as_ref().ok_or_else(|| {
                api_error(StatusCode::SERVICE_UNAVAILABLE, "TOTP key unavailable")
            })?;
            let raw = decrypt_auth_secret(key, secret)?;
            let secret_b32 = String::from_utf8(raw).map_err(|_| {
                api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "stored TOTP secret is invalid",
                )
            })?;
            let token = input.totp_code.as_deref().ok_or_else(|| {
                api_error(StatusCode::UNAUTHORIZED, "TOTP or recovery code required")
            })?;
            let verification = vsn_auth::verify_totp(&secret_b32, token)
                .map_err(|e| api_error(StatusCode::UNAUTHORIZED, &e.to_string()))?;
            let step = verification
                .matched_step
                .ok_or_else(|| api_error(StatusCode::UNAUTHORIZED, "invalid TOTP code"))?;
            if account.last_totp_step == Some(step) {
                return Err(api_error(
                    StatusCode::CONFLICT,
                    "TOTP code was already used",
                ));
            }
            mfa_verified = true;
            matched_step = Some(step);
        } else {
            return Err(api_error(
                StatusCode::UNAUTHORIZED,
                "recovery code required",
            ));
        }
    }
    let policy = state.auth_policy.lock().map_err(lock_error)?.clone();
    let now = vsn_remote::now_ms();
    let raw = random_id("vsnsess");
    let session = AccountSessionRecord {
        id: random_id("sess"),
        account_id: account.id.clone(),
        token_hash: hash_token(&raw),
        created_at_unix_ms: now,
        expires_at_unix_ms: now + (policy.session_ttl_minutes as u128) * 60_000,
        last_activity_unix_ms: now,
        mfa_verified,
        passkey_verified: false,
        revoked: false,
        federated: None,
    };
    {
        let mut accounts = state.accounts.lock().map_err(lock_error)?;
        if let Some(a) = accounts.get_mut(&account.id) {
            if let Some(step) = matched_step {
                if a.last_totp_step == Some(step) {
                    return Err(api_error(
                        StatusCode::CONFLICT,
                        "TOTP code was already used",
                    ));
                }
                a.last_totp_step = Some(step);
            }
            if let Some(index) = recovery_index {
                let expected = account.recovery_code_hashes.get(index).ok_or_else(|| {
                    api_error(StatusCode::CONFLICT, "recovery code state changed")
                })?;
                if a.recovery_code_hashes.get(index) != Some(expected) {
                    return Err(api_error(
                        StatusCode::CONFLICT,
                        "recovery code was already consumed or regenerated",
                    ));
                }
                a.recovery_code_hashes.remove(index);
            }
        }
    }
    let updated = state
        .accounts
        .lock()
        .map_err(lock_error)?
        .get(&account.id)
        .cloned()
        .ok_or_else(|| api_error(StatusCode::UNAUTHORIZED, "account disappeared during login"))?;
    sync_shared_account(&state, &updated)?;
    store_account_session(&state, &session)?;
    cleanup_sessions(&state)?;
    persist_auth_state(&state)?;
    Ok(Json(
        json!({"session_token":raw,"session":{"id":session.id,"account_id":session.account_id,"expires_at_unix_ms":session.expires_at_unix_ms,"mfa_verified":session.mfa_verified}}),
    ))
}
async fn account_me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let principal = authenticate_account_session(&state, &headers, true)?;
    Ok(Json(
        json!({"principal_id":principal.id,"permissions":principal.permissions,"bootstrap":false}),
    ))
}
async fn account_logout(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<LogoutRequest>,
) -> Result<Json<Value>, ApiError> {
    let supplied = bearer_token(&headers)?;
    let hash = hash_token(supplied);
    let session_id = if let Some(store) = state.state_postgres.as_ref() {
        let shared = store
            .session_by_token_hash(&hash)
            .map_err(|e| {
                api_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    &format!("shared session lookup failed: {e}"),
                )
            })?
            .ok_or_else(|| api_error(StatusCode::UNAUTHORIZED, "invalid session"))?;
        shared.session_id
    } else {
        let sessions = state.sessions.lock().map_err(lock_error)?;
        sessions
            .values()
            .find(|s| !s.revoked && constant_time_eq(s.token_hash.as_bytes(), hash.as_bytes()))
            .map(|s| s.id.clone())
            .ok_or_else(|| api_error(StatusCode::UNAUTHORIZED, "invalid session"))?
    };
    if let Some(expected) = input.session_id.as_ref() {
        if &session_id != expected {
            return Err(api_error(StatusCode::FORBIDDEN, "session id mismatch"));
        }
    }
    revoke_account_session_by_id(&state, &session_id)?;
    persist_auth_state(&state)?;
    Ok(Json(json!({"ok":true,"session_id":session_id})))
}

async fn list_devices(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    require_permission(&state, &headers, "control.devices.view")?;
    let devices = all_device_records(&state)?;
    Ok(Json(
        json!({"devices":devices,"shared":state.state_postgres.is_some()}),
    ))
}
async fn list_results(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    require_permission(&state, &headers, "control.results.view")?;
    if let Some(store) = state.state_postgres.as_ref() {
        let commands = store.recent_commands(500).map_err(|e| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("shared results list failed: {e}"),
            )
        })?;
        let mut results = Vec::new();
        for command in commands {
            if let Some(payload) = command.result_payload {
                let result: AgentCommandResultV1 = serde_json::from_str(&payload).map_err(|e| {
                    api_error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        &format!("shared result payload invalid: {e}"),
                    )
                })?;
                results.push(result);
                if results.len() >= 100 {
                    break;
                }
            }
        }
        return Ok(Json(json!({"results":results,"shared":true})));
    }
    let results = state
        .results
        .lock()
        .map_err(lock_error)?
        .iter()
        .rev()
        .take(100)
        .cloned()
        .collect::<Vec<_>>();
    Ok(Json(json!({"results":results,"shared":false})))
}
async fn list_deliveries(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    require_permission(&state, &headers, "control.commands.queue")?;
    if let Some(store) = state.state_postgres.as_ref() {
        let shared = store.recent_commands(500).map_err(|e| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("shared delivery list failed: {e}"),
            )
        })?;
        return Ok(Json(json!({"deliveries":shared,"shared":true})));
    }
    let deliveries = state.deliveries.lock().map_err(lock_error)?.clone();
    Ok(Json(json!({"deliveries":deliveries,"shared":false})))
}

async fn create_role(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<RoleRequest>,
) -> Result<Json<IamRole>, ApiError> {
    let principal = require_permission(&state, &headers, "control.iam.manage")?;
    validate_id(&input.id)?;
    if input.name.trim().is_empty() || input.name.len() > 128 || input.permissions.len() > 256 {
        return Err(api_error(StatusCode::BAD_REQUEST, "invalid role"));
    }
    for p in &input.permissions {
        validate_permission_string(p)?;
        if !principal.bootstrap && !principal.allows(p) {
            return Err(api_error(
                StatusCode::FORBIDDEN,
                "scoped IAM principal cannot create a role broader than its own permissions",
            ));
        }
    }
    let role = IamRole {
        id: input.id,
        name: input.name,
        permissions: input.permissions,
    };
    state
        .roles
        .lock()
        .map_err(lock_error)?
        .insert(role.id.clone(), role.clone());
    sync_shared_role(&state, &role)?;
    persist_auth_state(&state)?;
    Ok(Json(role))
}
async fn list_roles(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    refresh_shared_auth_state(&state)?;
    require_permission(&state, &headers, "control.iam.manage")?;
    let roles: Vec<_> = state
        .roles
        .lock()
        .map_err(lock_error)?
        .values()
        .cloned()
        .collect();
    Ok(Json(json!({"roles":roles})))
}
async fn create_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<TokenRequest>,
) -> Result<Json<Value>, ApiError> {
    let principal = require_permission(&state, &headers, "control.iam.manage")?;
    refresh_shared_iam_fleet_state(&state)?;
    validate_id(&input.principal_id)?;
    let role = state
        .roles
        .lock()
        .map_err(lock_error)?
        .get(&input.role_id)
        .cloned()
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "role not found"))?;
    if !principal.bootstrap && role.permissions.iter().any(|p| !principal.allows(p)) {
        return Err(api_error(
            StatusCode::FORBIDDEN,
            "scoped IAM principal cannot mint a token broader than its own permissions",
        ));
    }
    let raw = random_id("vsnpat");
    let record = ApiTokenRecord {
        id: random_id("token"),
        principal_id: input.principal_id,
        role_id: input.role_id,
        token_hash: hash_token(&raw),
        created_at_unix_ms: vsn_remote::now_ms(),
        revoked: false,
    };
    state
        .tokens
        .lock()
        .map_err(lock_error)?
        .insert(record.id.clone(), record.clone());
    sync_shared_api_token(&state, &record)?;
    persist_operational_state(&state)?;
    Ok(Json(
        json!({"token":raw,"record":{"id":record.id,"principal_id":record.principal_id,"role_id":record.role_id,"created_at_unix_ms":record.created_at_unix_ms}}),
    ))
}
async fn list_tokens(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    require_permission(&state, &headers, "control.iam.manage")?;
    refresh_shared_iam_fleet_state(&state)?;
    let tokens:Vec<_>=state.tokens.lock().map_err(lock_error)?.values().map(|t|json!({"id":t.id,"principal_id":t.principal_id,"role_id":t.role_id,"created_at_unix_ms":t.created_at_unix_ms,"revoked":t.revoked})).collect();
    Ok(Json(
        json!({"tokens":tokens,"shared":state.state_postgres.is_some()}),
    ))
}
async fn revoke_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<RevokeTokenRequest>,
) -> Result<Json<Value>, ApiError> {
    require_permission(&state, &headers, "control.iam.manage")?;
    refresh_shared_iam_fleet_state(&state)?;
    let record = {
        let mut tokens = state.tokens.lock().map_err(lock_error)?;
        let token = tokens
            .get_mut(&input.token_id)
            .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "token not found"))?;
        token.revoked = true;
        token.clone()
    };
    sync_shared_api_token(&state, &record)?;
    persist_operational_state(&state)?;
    Ok(Json(json!({"ok":true,"token_id":input.token_id})))
}

fn authenticate(state: &AppState, headers: &HeaderMap) -> Result<AuthPrincipal, ApiError> {
    let supplied = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("");
    authenticate_token(state, supplied, true)
}
fn authenticate_token(
    state: &AppState,
    supplied: &str,
    touch: bool,
) -> Result<AuthPrincipal, ApiError> {
    refresh_shared_auth_state(state)?;
    if supplied.is_empty() || supplied.len() > 4096 {
        return Err(api_error(
            StatusCode::UNAUTHORIZED,
            "authorization required",
        ));
    }
    if constant_time_eq(supplied.as_bytes(), state.admin_token.as_bytes()) {
        return Ok(AuthPrincipal {
            id: "bootstrap-admin".into(),
            permissions: ["*".to_string()].into_iter().collect(),
            bootstrap: true,
        });
    }
    if supplied.starts_with("vsnsess_") {
        return authenticate_account_session_token(state, supplied, touch);
    }
    let hash = hash_token(supplied);
    let token = if let Some(store) = state.state_postgres.as_ref() {
        let shared = store
            .api_token_by_hash(&hash)
            .map_err(|e| {
                api_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    &format!("shared API token lookup failed: {e}"),
                )
            })?
            .ok_or_else(|| api_error(StatusCode::UNAUTHORIZED, "invalid or revoked API token"))?;
        if shared.revoked {
            return Err(api_error(
                StatusCode::UNAUTHORIZED,
                "invalid or revoked API token",
            ));
        }
        ApiTokenRecord {
            id: shared.token_id,
            principal_id: shared.principal_id,
            role_id: shared.role_id,
            token_hash: shared.token_hash,
            created_at_unix_ms: shared.created_at_unix_ms,
            revoked: shared.revoked,
        }
    } else {
        state
            .tokens
            .lock()
            .map_err(lock_error)?
            .values()
            .find(|t| !t.revoked && constant_time_eq(t.token_hash.as_bytes(), hash.as_bytes()))
            .cloned()
            .ok_or_else(|| api_error(StatusCode::UNAUTHORIZED, "invalid or revoked API token"))?
    };
    let role = state
        .roles
        .lock()
        .map_err(lock_error)?
        .get(&token.role_id)
        .cloned()
        .ok_or_else(|| api_error(StatusCode::UNAUTHORIZED, "token role no longer exists"))?;
    Ok(AuthPrincipal {
        id: token.principal_id,
        permissions: role.permissions.into_iter().collect(),
        bootstrap: false,
    })
}
fn require_permission(
    state: &AppState,
    headers: &HeaderMap,
    permission: &str,
) -> Result<AuthPrincipal, ApiError> {
    let principal = authenticate(state, headers)?;
    check_rate_limit(state, &principal.id, 240, 60_000)?;
    if principal.allows(permission) {
        Ok(principal)
    } else {
        Err(api_error(
            StatusCode::FORBIDDEN,
            "control-plane permission denied",
        ))
    }
}
fn check_rate_limit(
    state: &AppState,
    key: &str,
    limit: usize,
    window_ms: u128,
) -> Result<(), ApiError> {
    let now = vsn_remote::now_ms();
    if let Some(store) = state.state_postgres.as_ref() {
        let limit = u32::try_from(limit).map_err(|_| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "rate-limit configuration exceeds u32",
            )
        })?;
        let window = u64::try_from(window_ms).map_err(|_| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "rate-limit window exceeds u64",
            )
        })?;
        let accepted = store
            .consume_rate_limit(key, limit, window, now)
            .map_err(|e| {
                api_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    &format!("shared rate-limit store unavailable: {e}"),
                )
            })?;
        if !accepted {
            return Err(api_error(
                StatusCode::TOO_MANY_REQUESTS,
                "control-plane rate limit exceeded",
            ));
        }
        return Ok(());
    }
    let mut limits = state.rate_limits.lock().map_err(lock_error)?;
    let bucket = limits.entry(key.into()).or_default();
    while bucket
        .front()
        .copied()
        .map(|v| v.saturating_add(window_ms) < now)
        .unwrap_or(false)
    {
        bucket.pop_front();
    }
    if bucket.len() >= limit {
        return Err(api_error(
            StatusCode::TOO_MANY_REQUESTS,
            "control-plane rate limit exceeded",
        ));
    }
    bucket.push_back(now);
    Ok(())
}

fn is_loopback_bind(bind: &str) -> bool {
    bind.starts_with("127.") || bind.starts_with("[::1]:") || bind.starts_with("localhost:")
}
fn safe_identifier(v: &str) -> bool {
    v.len() >= 2
        && v.len() <= 96
        && v.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}
fn load_postgres_state_store() -> Result<Option<vsn_control_store::PostgresSnapshotStore>, String> {
    let dsn = match std::env::var("VSN_CONTROL_POSTGRES_DSN") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => return Ok(None),
    };
    let ca = PathBuf::from(std::env::var("VSN_CONTROL_POSTGRES_CA_PEM").map_err(|_| {
        "VSN_CONTROL_POSTGRES_CA_PEM is required when VSN_CONTROL_POSTGRES_DSN is set".to_string()
    })?);
    let store =
        vsn_control_store::PostgresSnapshotStore::open(dsn, ca).map_err(|e| e.to_string())?;
    Ok(Some(store))
}
fn load_persistent_state_postgres(
    store: &vsn_control_store::PostgresSnapshotStore,
    local_path: &Path,
) -> Result<(PersistentState, u64), String> {
    if let Some(snapshot) = store.load("control-plane").map_err(|e| e.to_string())? {
        let parsed = serde_json::from_slice(&snapshot.payload)
            .map_err(|e| format!("PostgreSQL state parse failed: {e}"))?;
        return Ok((parsed, snapshot.generation));
    }
    let import_local = std::env::var("VSN_CONTROL_POSTGRES_IMPORT_LOCAL")
        .map(|v| matches!(v.as_str(), "1" | "true" | "yes"))
        .unwrap_or(false);
    if !import_local {
        return Ok((PersistentState::default(), 0));
    }
    let (parsed, _) = load_persistent_state(local_path)?;
    let mut bytes = serde_json::to_vec_pretty(&parsed)
        .map_err(|e| format!("local state serialize for PostgreSQL import failed: {e}"))?;
    bytes.push(b'\n');
    let committed = store
        .save_if_generation("control-plane", 0, &bytes)
        .map_err(|e| format!("PostgreSQL state import failed: {e}"))?;
    Ok((parsed, committed.generation))
}
fn state_backup_path(path: &Path) -> PathBuf {
    path.with_extension("bak")
}
fn load_persistent_state(path: &Path) -> Result<(PersistentState, u64), String> {
    if path.extension().and_then(|v| v.to_str()) == Some("db") {
        let store = vsn_control_store::SnapshotStore::open(path).map_err(|e| e.to_string())?;
        if let Some(snapshot) = store.load("control-plane").map_err(|e| e.to_string())? {
            let parsed = serde_json::from_slice(&snapshot.payload).map_err(|e| e.to_string())?;
            return Ok((parsed, snapshot.generation));
        }
        // One-time upgrade path from the 0.6 JSON snapshot when the new SQLite
        // store has no committed snapshot yet. The source is preserved with a
        // .migrated suffix after a verified SQLite commit.
        let legacy = path.with_extension("json");
        let legacy_backup = state_backup_path(&legacy);
        let source = if legacy.exists() {
            Some(legacy.clone())
        } else if legacy_backup.exists() {
            Some(legacy_backup.clone())
        } else {
            None
        };
        if let Some(source) = source {
            let bytes = fs::read(&source).map_err(|e| format!("legacy state read failed: {e}"))?;
            let parsed: PersistentState = serde_json::from_slice(&bytes)
                .map_err(|e| format!("legacy state parse failed: {e}"))?;
            let canonical = serde_json::to_vec_pretty(&parsed)
                .map_err(|e| format!("legacy state serialize failed: {e}"))?;
            let committed = store
                .save("control-plane", &canonical)
                .map_err(|e| format!("legacy state SQLite commit failed: {e}"))?;
            let check = store
                .load("control-plane")
                .map_err(|e| e.to_string())?
                .ok_or_else(|| {
                    "legacy state migration verification snapshot missing".to_string()
                })?;
            let _: PersistentState = serde_json::from_slice(&check.payload)
                .map_err(|e| format!("legacy migration verification failed: {e}"))?;
            let mut migrated_name = source.as_os_str().to_os_string();
            migrated_name.push(".migrated");
            let migrated = PathBuf::from(migrated_name);
            fs::rename(&source, &migrated).map_err(|e| {
                format!("legacy state archive failed after successful migration: {e}")
            })?;
            return Ok((parsed, committed.generation));
        }
        return Ok((PersistentState::default(), 0));
    }
    let backup = state_backup_path(path);
    if !path.exists() && !backup.exists() {
        return Ok((PersistentState::default(), 0));
    }
    let source = if path.exists() {
        path
    } else {
        backup.as_path()
    };
    let bytes = fs::read(source).map_err(|e| e.to_string())?;
    let parsed = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if source == backup.as_path() && !path.exists() {
        let _ = fs::rename(&backup, path);
    }
    Ok((parsed, 0))
}
fn persist_auth_state(state: &AppState) -> Result<(), ApiError> {
    if state.state_postgres.is_some() {
        return Ok(());
    }
    persist_state(state)
}
fn persist_state(state: &AppState) -> Result<(), ApiError> {
    let _guard = state.persist_lock.lock().map_err(lock_error)?;
    let snapshot = PersistentState {
        pairings: state.pairings.lock().map_err(lock_error)?.clone(),
        devices: state.devices.lock().map_err(lock_error)?.clone(),
        queues: state.queues.lock().map_err(lock_error)?.clone(),
        deliveries: state.deliveries.lock().map_err(lock_error)?.clone(),
        results: state.results.lock().map_err(lock_error)?.clone(),
        roles: state.roles.lock().map_err(lock_error)?.clone(),
        tokens: state.tokens.lock().map_err(lock_error)?.clone(),
        fleet_groups: state.fleet_groups.lock().map_err(lock_error)?.clone(),
        environments: state.environments.lock().map_err(lock_error)?.clone(),
        approvals: state.approvals.lock().map_err(lock_error)?.clone(),
        central_audit: state.central_audit.lock().map_err(lock_error)?.clone(),
        auth_policy: state.auth_policy.lock().map_err(lock_error)?.clone(),
        accounts: state.accounts.lock().map_err(lock_error)?.clone(),
        scim_groups: state.scim_groups.lock().map_err(lock_error)?.clone(),
        sessions: state.sessions.lock().map_err(lock_error)?.clone(),
    };
    let mut bytes = serde_json::to_vec_pretty(&snapshot).map_err(|e| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("state serialize failed: {e}"),
        )
    })?;
    bytes.push(b'\n');
    if let Some(store) = state.state_postgres.as_ref() {
        let mut generation = state.state_generation.lock().map_err(lock_error)?;
        let committed = store
            .save_if_generation("control-plane", *generation, &bytes)
            .map_err(|e| {
                api_error(
                    StatusCode::CONFLICT,
                    &format!("state PostgreSQL generation conflict or commit failure: {e}"),
                )
            })?;
        *generation = committed.generation;
        return Ok(());
    }
    if let Some(parent) = state.state_path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("state directory create failed: {e}"),
            )
        })?;
        set_private_dir_permissions(parent).map_err(|e| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("state directory permission hardening failed: {e}"),
            )
        })?;
    }
    let tmp = state.state_path.with_extension("tmp");
    let backup = state_backup_path(state.state_path.as_ref());
    if state.state_path.extension().and_then(|v| v.to_str()) == Some("db") {
        let store =
            vsn_control_store::SnapshotStore::open(state.state_path.as_ref()).map_err(|e| {
                api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("state sqlite open failed: {e}"),
                )
            })?;
        let mut generation = state.state_generation.lock().map_err(lock_error)?;
        let committed = store
            .save_if_generation("control-plane", *generation, &bytes)
            .map_err(|e| {
                api_error(
                    StatusCode::CONFLICT,
                    &format!("state sqlite generation conflict or commit failure: {e}"),
                )
            })?;
        *generation = committed.generation;
        return Ok(());
    }
    {
        use std::io::Write as _;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&tmp)
            .map_err(|e| {
                api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("state temp open failed: {e}"),
                )
            })?;
        file.write_all(&bytes).map_err(|e| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("state write failed: {e}"),
            )
        })?;
        file.sync_all().map_err(|e| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("state sync failed: {e}"),
            )
        })?;
    }
    set_private_file_permissions(&tmp).map_err(|e| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("state permission hardening failed: {e}"),
        )
    })?;
    if backup.exists() {
        let _ = fs::remove_file(&backup);
    }
    if state.state_path.exists() {
        fs::rename(state.state_path.as_ref(), &backup).map_err(|e| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("state backup failed: {e}"),
            )
        })?;
    }
    if let Err(err) = fs::rename(&tmp, state.state_path.as_ref()) {
        if backup.exists() {
            let _ = fs::rename(&backup, state.state_path.as_ref());
        }
        return Err(api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("state commit failed: {err}"),
        ));
    }
    if backup.exists() {
        fs::remove_file(&backup).map_err(|e| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("state backup cleanup failed: {e}"),
            )
        })?;
    }
    if let Some(parent) = state.state_path.parent() {
        if let Ok(dir) = fs::File::open(parent) {
            let _ = dir.sync_all();
        }
    }
    Ok(())
}
#[cfg(unix)]
fn set_private_file_permissions(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
}
#[cfg(not(unix))]
fn set_private_file_permissions(_path: &Path) -> std::io::Result<()> {
    Ok(())
}
#[cfg(unix)]
fn set_private_dir_permissions(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
}
#[cfg(not(unix))]
fn set_private_dir_permissions(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

fn persist_operational_state(state: &AppState) -> Result<(), ApiError> {
    if state.state_postgres.is_some() {
        return Ok(());
    }
    persist_state(state)
}
fn sync_shared_api_token(state: &AppState, record: &ApiTokenRecord) -> Result<(), ApiError> {
    let Some(store) = state.state_postgres.as_ref() else {
        return Ok(());
    };
    store
        .upsert_api_token(&vsn_control_store::SharedApiTokenRecord {
            token_id: record.id.clone(),
            principal_id: record.principal_id.clone(),
            role_id: record.role_id.clone(),
            token_hash: record.token_hash.clone(),
            created_at_unix_ms: record.created_at_unix_ms,
            revoked: record.revoked,
            updated_at_unix_ms: vsn_remote::now_ms(),
        })
        .map_err(|e| {
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!("shared API token write failed: {e}"),
            )
        })
}
fn sync_shared_fleet_group(state: &AppState, group: &FleetGroup) -> Result<(), ApiError> {
    let Some(store) = state.state_postgres.as_ref() else {
        return Ok(());
    };
    let payload = serde_json::to_string(group).map_err(|e| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("fleet group serialization failed: {e}"),
        )
    })?;
    store
        .upsert_fleet_group(&vsn_control_store::SharedFleetGroupRecord {
            group_id: group.id.clone(),
            payload,
            updated_at_unix_ms: vsn_remote::now_ms(),
        })
        .map_err(|e| {
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!("shared fleet group write failed: {e}"),
            )
        })
}
fn sync_shared_environment(state: &AppState, env: &EnvironmentRecord) -> Result<(), ApiError> {
    let Some(store) = state.state_postgres.as_ref() else {
        return Ok(());
    };
    let payload = serde_json::to_string(env).map_err(|e| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("environment serialization failed: {e}"),
        )
    })?;
    store
        .upsert_environment(&vsn_control_store::SharedEnvironmentRecord {
            environment_id: env.id.clone(),
            payload,
            updated_at_unix_ms: vsn_remote::now_ms(),
        })
        .map_err(|e| {
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!("shared environment write failed: {e}"),
            )
        })
}
fn sync_shared_device_fleet(state: &AppState, device: &DeviceRecord) -> Result<(), ApiError> {
    let Some(store) = state.state_postgres.as_ref() else {
        return Ok(());
    };
    let payload = serde_json::to_string(&json!({"labels":device.labels,"groups":device.groups}))
        .map_err(|e| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("device fleet serialization failed: {e}"),
            )
        })?;
    store
        .upsert_device_fleet(&vsn_control_store::SharedDeviceFleetRecord {
            device_id: device.device_id.clone(),
            payload,
            updated_at_unix_ms: vsn_remote::now_ms(),
        })
        .map_err(|e| {
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!("shared device fleet write failed: {e}"),
            )
        })
}
fn backfill_shared_iam_fleet_once(state: &AppState) -> Result<(), ApiError> {
    let Some(store) = state.state_postgres.as_ref() else {
        return Ok(());
    };
    if store.api_token_count().map_err(|e| {
        api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            &format!("shared API token count failed: {e}"),
        )
    })? == 0
    {
        for token in state
            .tokens
            .lock()
            .map_err(lock_error)?
            .values()
            .cloned()
            .collect::<Vec<_>>()
        {
            sync_shared_api_token(state, &token)?;
        }
    }
    if store.fleet_group_count().map_err(|e| {
        api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            &format!("shared fleet group count failed: {e}"),
        )
    })? == 0
    {
        for group in state
            .fleet_groups
            .lock()
            .map_err(lock_error)?
            .values()
            .cloned()
            .collect::<Vec<_>>()
        {
            sync_shared_fleet_group(state, &group)?;
        }
    }
    if store.environment_count().map_err(|e| {
        api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            &format!("shared environment count failed: {e}"),
        )
    })? == 0
    {
        for env in state
            .environments
            .lock()
            .map_err(lock_error)?
            .values()
            .cloned()
            .collect::<Vec<_>>()
        {
            sync_shared_environment(state, &env)?;
        }
    }
    for device in state
        .devices
        .lock()
        .map_err(lock_error)?
        .values()
        .cloned()
        .collect::<Vec<_>>()
    {
        if !device.labels.is_empty() || !device.groups.is_empty() {
            sync_shared_device_fleet(state, &device)?;
        }
    }
    Ok(())
}
fn refresh_shared_iam_fleet_state(state: &AppState) -> Result<(), ApiError> {
    let Some(store) = state.state_postgres.as_ref() else {
        return Ok(());
    };
    let tokens = store.list_api_tokens().map_err(|e| {
        api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            &format!("shared API tokens unavailable: {e}"),
        )
    })?;
    let groups = store.list_fleet_groups().map_err(|e| {
        api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            &format!("shared fleet groups unavailable: {e}"),
        )
    })?;
    let envs = store.list_environments_shared().map_err(|e| {
        api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            &format!("shared environments unavailable: {e}"),
        )
    })?;
    let device_fleet = store.list_device_fleet().map_err(|e| {
        api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            &format!("shared device fleet unavailable: {e}"),
        )
    })?;
    let mut token_map = HashMap::new();
    for t in tokens {
        token_map.insert(
            t.token_id.clone(),
            ApiTokenRecord {
                id: t.token_id,
                principal_id: t.principal_id,
                role_id: t.role_id,
                token_hash: t.token_hash,
                created_at_unix_ms: t.created_at_unix_ms,
                revoked: t.revoked,
            },
        );
    }
    let mut group_map = HashMap::new();
    for row in groups {
        let value: FleetGroup = serde_json::from_str(&row.payload).map_err(|e| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("shared fleet group payload invalid: {e}"),
            )
        })?;
        group_map.insert(value.id.clone(), value);
    }
    let mut env_map = HashMap::new();
    for row in envs {
        let value: EnvironmentRecord = serde_json::from_str(&row.payload).map_err(|e| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("shared environment payload invalid: {e}"),
            )
        })?;
        env_map.insert(value.id.clone(), value);
    }
    *state.tokens.lock().map_err(lock_error)? = token_map;
    *state.fleet_groups.lock().map_err(lock_error)? = group_map;
    *state.environments.lock().map_err(lock_error)? = env_map;
    let mut devices = state.devices.lock().map_err(lock_error)?;
    for row in device_fleet {
        if let Some(device) = devices.get_mut(&row.device_id) {
            let value: Value = serde_json::from_str(&row.payload).map_err(|e| {
                api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("shared device fleet payload invalid: {e}"),
                )
            })?;
            device.labels =
                serde_json::from_value(value.get("labels").cloned().unwrap_or_else(|| json!({})))
                    .map_err(|e| {
                    api_error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        &format!("shared device labels invalid: {e}"),
                    )
                })?;
            device.groups =
                serde_json::from_value(value.get("groups").cloned().unwrap_or_else(|| json!([])))
                    .map_err(|e| {
                    api_error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        &format!("shared device groups invalid: {e}"),
                    )
                })?;
        }
    }
    Ok(())
}

fn backfill_shared_sessions_once(state: &AppState) -> Result<(), ApiError> {
    let Some(store) = state.state_postgres.as_ref() else {
        return Ok(());
    };
    if store.session_count().map_err(|e| {
        api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            &format!("shared session count failed: {e}"),
        )
    })? > 0
    {
        return Ok(());
    }
    let sessions = state
        .sessions
        .lock()
        .map_err(lock_error)?
        .values()
        .cloned()
        .collect::<Vec<_>>();
    for session in sessions {
        let payload = account_session_payload(&session)?;
        store
            .upsert_session(&vsn_control_store::SharedSessionRecord {
                session_id: session.id.clone(),
                account_id: session.account_id.clone(),
                token_hash: session.token_hash.clone(),
                payload,
                created_at_unix_ms: session.created_at_unix_ms,
                expires_at_unix_ms: session.expires_at_unix_ms,
                last_activity_unix_ms: session.last_activity_unix_ms,
                revoked: session.revoked,
            })
            .map_err(|e| {
                api_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    &format!("shared session backfill failed: {e}"),
                )
            })?;
    }
    Ok(())
}

fn backfill_shared_auth_once(state: &AppState) -> Result<(), ApiError> {
    let Some(store) = state.state_postgres.as_ref() else {
        return Ok(());
    };
    let now = vsn_remote::now_ms();
    if store.role_count().map_err(|e| {
        api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            &format!("shared role count failed: {e}"),
        )
    })? == 0
    {
        let roles = state
            .roles
            .lock()
            .map_err(lock_error)?
            .values()
            .cloned()
            .collect::<Vec<_>>();
        for role in roles {
            let payload = serde_json::to_string(&role).map_err(|e| {
                api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("role serialization failed: {e}"),
                )
            })?;
            store
                .upsert_role(&vsn_control_store::SharedRoleRecord {
                    role_id: role.id,
                    payload,
                    updated_at_unix_ms: now,
                })
                .map_err(|e| {
                    api_error(
                        StatusCode::SERVICE_UNAVAILABLE,
                        &format!("shared role backfill failed: {e}"),
                    )
                })?;
        }
    }
    if store.account_count().map_err(|e| {
        api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            &format!("shared account count failed: {e}"),
        )
    })? == 0
    {
        let accounts = state
            .accounts
            .lock()
            .map_err(lock_error)?
            .values()
            .cloned()
            .collect::<Vec<_>>();
        for account in accounts {
            let payload = serde_json::to_string(&account).map_err(|e| {
                api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("account serialization failed: {e}"),
                )
            })?;
            store
                .upsert_account(&vsn_control_store::SharedAccountRecord {
                    account_id: account.id.clone(),
                    email: account.email.clone(),
                    role_id: account.role_id.clone(),
                    payload,
                    disabled: account.disabled,
                    updated_at_unix_ms: now,
                })
                .map_err(|e| {
                    api_error(
                        StatusCode::SERVICE_UNAVAILABLE,
                        &format!("shared account backfill failed: {e}"),
                    )
                })?;
        }
    }
    if store.scim_group_count().map_err(|e| {
        api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            &format!("shared SCIM group count failed: {e}"),
        )
    })? == 0
    {
        let groups = state
            .scim_groups
            .lock()
            .map_err(lock_error)?
            .values()
            .cloned()
            .collect::<Vec<_>>();
        for group in groups {
            let payload = serde_json::to_string(&group).map_err(|e| {
                api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("SCIM group serialization failed: {e}"),
                )
            })?;
            store
                .upsert_scim_group(&vsn_control_store::SharedScimGroupRecord {
                    group_id: group.id.clone(),
                    display_name: group.display_name.clone(),
                    payload,
                    updated_at_unix_ms: now,
                })
                .map_err(|e| {
                    api_error(
                        StatusCode::SERVICE_UNAVAILABLE,
                        &format!("shared SCIM group backfill failed: {e}"),
                    )
                })?;
        }
    }
    if store
        .auth_policy("enterprise")
        .map_err(|e| {
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!("shared auth policy lookup failed: {e}"),
            )
        })?
        .is_none()
    {
        let policy = state.auth_policy.lock().map_err(lock_error)?.clone();
        let payload = serde_json::to_string(&policy).map_err(|e| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("auth policy serialization failed: {e}"),
            )
        })?;
        store
            .upsert_auth_policy(&vsn_control_store::SharedAuthPolicyRecord {
                policy_id: "enterprise".into(),
                payload,
                updated_at_unix_ms: now,
            })
            .map_err(|e| {
                api_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    &format!("shared auth policy backfill failed: {e}"),
                )
            })?;
    }
    Ok(())
}
fn refresh_shared_auth_state(state: &AppState) -> Result<(), ApiError> {
    let Some(store) = state.state_postgres.as_ref() else {
        return Ok(());
    };
    let shared_roles = store.list_roles_shared().map_err(|e| {
        api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            &format!("shared roles unavailable: {e}"),
        )
    })?;
    let shared_accounts = store.list_accounts_shared().map_err(|e| {
        api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            &format!("shared accounts unavailable: {e}"),
        )
    })?;
    let shared_groups = store.list_scim_groups().map_err(|e| {
        api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            &format!("shared SCIM groups unavailable: {e}"),
        )
    })?;
    let mut roles = HashMap::new();
    for record in shared_roles {
        let mut role: IamRole = serde_json::from_str(&record.payload).map_err(|e| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("shared role payload invalid: {e}"),
            )
        })?;
        role.id = record.role_id;
        roles.insert(role.id.clone(), role);
    }
    let mut accounts = HashMap::new();
    for record in shared_accounts {
        let mut account: AccountRecord = serde_json::from_str(&record.payload).map_err(|e| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("shared account payload invalid: {e}"),
            )
        })?;
        account.id = record.account_id;
        account.email = record.email;
        account.role_id = record.role_id;
        account.disabled = record.disabled;
        accounts.insert(account.id.clone(), account);
    }
    let mut groups = HashMap::new();
    for record in shared_groups {
        let mut group: ScimGroupRecord = serde_json::from_str(&record.payload).map_err(|e| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("shared SCIM group payload invalid: {e}"),
            )
        })?;
        group.id = record.group_id;
        group.display_name = record.display_name;
        groups.insert(group.id.clone(), group);
    }
    *state.roles.lock().map_err(lock_error)? = roles;
    *state.accounts.lock().map_err(lock_error)? = accounts;
    *state.scim_groups.lock().map_err(lock_error)? = groups;
    if let Some(record) = store.auth_policy("enterprise").map_err(|e| {
        api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            &format!("shared auth policy unavailable: {e}"),
        )
    })? {
        let policy: vsn_auth::EnterpriseAuthPolicy = serde_json::from_str(&record.payload)
            .map_err(|e| {
                api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("shared auth policy payload invalid: {e}"),
                )
            })?;
        vsn_auth::validate_policy(&policy).map_err(|e| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("shared auth policy invalid: {e}"),
            )
        })?;
        *state.auth_policy.lock().map_err(lock_error)? = policy;
    }
    Ok(())
}
fn sync_shared_auth_policy(
    state: &AppState,
    policy: &vsn_auth::EnterpriseAuthPolicy,
) -> Result<(), ApiError> {
    let Some(store) = state.state_postgres.as_ref() else {
        return Ok(());
    };
    let payload = serde_json::to_string(policy).map_err(|e| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("auth policy serialization failed: {e}"),
        )
    })?;
    store
        .upsert_auth_policy(&vsn_control_store::SharedAuthPolicyRecord {
            policy_id: "enterprise".into(),
            payload,
            updated_at_unix_ms: vsn_remote::now_ms(),
        })
        .map_err(|e| {
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!("shared auth policy write failed: {e}"),
            )
        })?;
    Ok(())
}
fn sync_shared_role(state: &AppState, role: &IamRole) -> Result<(), ApiError> {
    let Some(store) = state.state_postgres.as_ref() else {
        return Ok(());
    };
    let payload = serde_json::to_string(role).map_err(|e| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("role serialization failed: {e}"),
        )
    })?;
    store
        .upsert_role(&vsn_control_store::SharedRoleRecord {
            role_id: role.id.clone(),
            payload,
            updated_at_unix_ms: vsn_remote::now_ms(),
        })
        .map_err(|e| {
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!("shared role write failed: {e}"),
            )
        })?;
    Ok(())
}
fn sync_shared_account(state: &AppState, account: &AccountRecord) -> Result<(), ApiError> {
    let Some(store) = state.state_postgres.as_ref() else {
        return Ok(());
    };
    let payload = serde_json::to_string(account).map_err(|e| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("account serialization failed: {e}"),
        )
    })?;
    store
        .upsert_account(&vsn_control_store::SharedAccountRecord {
            account_id: account.id.clone(),
            email: account.email.clone(),
            role_id: account.role_id.clone(),
            payload,
            disabled: account.disabled,
            updated_at_unix_ms: vsn_remote::now_ms(),
        })
        .map_err(|e| {
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!("shared account write failed: {e}"),
            )
        })?;
    Ok(())
}
fn delete_shared_account(state: &AppState, account_id: &str) -> Result<(), ApiError> {
    if let Some(store) = state.state_postgres.as_ref() {
        store.delete_account(account_id).map_err(|e| {
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!("shared account delete failed: {e}"),
            )
        })?;
    }
    Ok(())
}
fn sync_shared_scim_group(state: &AppState, group: &ScimGroupRecord) -> Result<(), ApiError> {
    if let Some(store) = state.state_postgres.as_ref() {
        let payload = serde_json::to_string(group).map_err(|e| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("SCIM group serialization failed: {e}"),
            )
        })?;
        store
            .upsert_scim_group(&vsn_control_store::SharedScimGroupRecord {
                group_id: group.id.clone(),
                display_name: group.display_name.clone(),
                payload,
                updated_at_unix_ms: vsn_remote::now_ms(),
            })
            .map_err(|e| {
                api_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    &format!("shared SCIM group write failed: {e}"),
                )
            })?;
    }
    Ok(())
}
fn delete_shared_scim_group(state: &AppState, group_id: &str) -> Result<(), ApiError> {
    if let Some(store) = state.state_postgres.as_ref() {
        store.delete_scim_group(group_id).map_err(|e| {
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!("shared SCIM group delete failed: {e}"),
            )
        })?;
    }
    Ok(())
}
fn store_oidc_transaction(
    state: &AppState,
    pending: &PendingOidcTransaction,
) -> Result<(), ApiError> {
    if let Some(store) = state.state_postgres.as_ref() {
        let payload =
            serde_json::to_string(&(pending.provider_id.clone(), pending.transaction.clone()))
                .map_err(|e| {
                    api_error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        &format!("OIDC transaction serialization failed: {e}"),
                    )
                })?;
        store
            .put_auth_transaction(&vsn_control_store::SharedAuthTransaction {
                transaction_id: pending.transaction.state.clone(),
                kind: "oidc".into(),
                payload,
                created_at_unix_ms: pending.transaction.created_at_unix_ms,
                expires_at_unix_ms: pending.transaction.created_at_unix_ms + 10 * 60 * 1000,
                consumed_at_unix_ms: None,
            })
            .map_err(|e| {
                api_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    &format!("shared OIDC transaction write failed: {e}"),
                )
            })?;
    }
    Ok(())
}
fn take_oidc_transaction(
    state: &AppState,
    transaction_id: &str,
) -> Result<Option<PendingOidcTransaction>, ApiError> {
    if let Some(store) = state.state_postgres.as_ref() {
        let Some(record) = store
            .consume_auth_transaction(transaction_id, "oidc", vsn_remote::now_ms())
            .map_err(|e| {
                api_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    &format!("shared OIDC transaction consume failed: {e}"),
                )
            })?
        else {
            return Ok(None);
        };
        let (provider_id, transaction): (String, vsn_auth::OidcPkceTransaction) =
            serde_json::from_str(&record.payload).map_err(|e| {
                api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("shared OIDC transaction payload invalid: {e}"),
                )
            })?;
        return Ok(Some(PendingOidcTransaction {
            provider_id,
            transaction,
        }));
    }
    Ok(state
        .oidc_transactions
        .lock()
        .map_err(lock_error)?
        .remove(transaction_id))
}

fn normalize_email(value: &str) -> Result<String, ApiError> {
    let v = value.trim().to_ascii_lowercase();
    if v.len() < 3
        || v.len() > 320
        || v.chars().any(|c| c.is_whitespace() || c.is_control())
        || !v.contains('@')
    {
        Err(api_error(StatusCode::BAD_REQUEST, "invalid email"))
    } else {
        Ok(v)
    }
}
fn bearer_token(headers: &HeaderMap) -> Result<&str, ApiError> {
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .filter(|v| !v.is_empty())
        .ok_or_else(|| api_error(StatusCode::UNAUTHORIZED, "authorization required"))
}
fn account_session_payload(session: &AccountSessionRecord) -> Result<String, ApiError> {
    serde_json::to_string(session).map_err(|e| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("session serialization failed: {e}"),
        )
    })
}
fn store_account_session(state: &AppState, session: &AccountSessionRecord) -> Result<(), ApiError> {
    state
        .sessions
        .lock()
        .map_err(lock_error)?
        .insert(session.id.clone(), session.clone());
    if let Some(store) = state.state_postgres.as_ref() {
        let payload = account_session_payload(session)?;
        let shared = vsn_control_store::SharedSessionRecord {
            session_id: session.id.clone(),
            account_id: session.account_id.clone(),
            token_hash: session.token_hash.clone(),
            payload,
            created_at_unix_ms: session.created_at_unix_ms,
            expires_at_unix_ms: session.expires_at_unix_ms,
            last_activity_unix_ms: session.last_activity_unix_ms,
            revoked: session.revoked,
        };
        store.upsert_session(&shared).map_err(|e| {
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!("shared session store unavailable: {e}"),
            )
        })?;
    }
    Ok(())
}
fn revoke_account_sessions_for(state: &AppState, account_id: &str) -> Result<(), ApiError> {
    {
        let mut sessions = state.sessions.lock().map_err(lock_error)?;
        for session in sessions.values_mut().filter(|s| s.account_id == account_id) {
            session.revoked = true;
        }
    }
    if let Some(store) = state.state_postgres.as_ref() {
        store.revoke_account_sessions(account_id).map_err(|e| {
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!("shared session revoke failed: {e}"),
            )
        })?;
    }
    Ok(())
}
fn revoke_account_session_by_id(state: &AppState, session_id: &str) -> Result<(), ApiError> {
    if let Some(session) = state
        .sessions
        .lock()
        .map_err(lock_error)?
        .get_mut(session_id)
    {
        session.revoked = true;
    }
    if let Some(store) = state.state_postgres.as_ref() {
        if !store.revoke_session(session_id).map_err(|e| {
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!("shared session revoke failed: {e}"),
            )
        })? {
            return Err(api_error(StatusCode::NOT_FOUND, "session not found"));
        }
    }
    Ok(())
}
fn authenticate_account_session(
    state: &AppState,
    headers: &HeaderMap,
    touch: bool,
) -> Result<AuthPrincipal, ApiError> {
    let token = bearer_token(headers)?;
    authenticate_account_session_token(state, token, touch)
}
fn authenticate_account_session_token(
    state: &AppState,
    token: &str,
    touch: bool,
) -> Result<AuthPrincipal, ApiError> {
    refresh_shared_auth_state(state)?;
    let hash = hash_token(token);
    let now = vsn_remote::now_ms();
    let policy = state.auth_policy.lock().map_err(lock_error)?.clone();
    let (account_id, mfa, passkey, created_at, last_activity) =
        if let Some(store) = state.state_postgres.as_ref() {
            let shared = store
                .session_by_token_hash(&hash)
                .map_err(|e| {
                    api_error(
                        StatusCode::SERVICE_UNAVAILABLE,
                        &format!("shared session lookup failed: {e}"),
                    )
                })?
                .ok_or_else(|| api_error(StatusCode::UNAUTHORIZED, "invalid or expired session"))?;
            let mut session: AccountSessionRecord =
                serde_json::from_str(&shared.payload).map_err(|e| {
                    api_error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        &format!("shared session payload invalid: {e}"),
                    )
                })?;
            session.id = shared.session_id;
            session.account_id = shared.account_id;
            session.token_hash = shared.token_hash;
            session.created_at_unix_ms = shared.created_at_unix_ms;
            session.expires_at_unix_ms = shared.expires_at_unix_ms;
            session.last_activity_unix_ms = shared.last_activity_unix_ms;
            session.revoked = shared.revoked;
            if session.revoked
                || session.expires_at_unix_ms < now
                || session
                    .last_activity_unix_ms
                    .saturating_add((policy.idle_ttl_minutes as u128) * 60_000)
                    < now
            {
                let _ = store.revoke_session(&session.id);
                return Err(api_error(
                    StatusCode::UNAUTHORIZED,
                    "invalid or expired session",
                ));
            }
            if touch && now.saturating_sub(session.last_activity_unix_ms) >= 60_000 {
                session.last_activity_unix_ms = now;
                let payload = account_session_payload(&session)?;
                if !store
                    .touch_session(&session.id, now, &payload)
                    .map_err(|e| {
                        api_error(
                            StatusCode::SERVICE_UNAVAILABLE,
                            &format!("shared session touch failed: {e}"),
                        )
                    })?
                {
                    return Err(api_error(
                        StatusCode::UNAUTHORIZED,
                        "session expired during refresh",
                    ));
                }
            }
            state
                .sessions
                .lock()
                .map_err(lock_error)?
                .insert(session.id.clone(), session.clone());
            (
                session.account_id.clone(),
                session.mfa_verified,
                session.passkey_verified,
                session.created_at_unix_ms,
                session.last_activity_unix_ms,
            )
        } else {
            let mut sessions = state.sessions.lock().map_err(lock_error)?;
            let session = sessions
                .values_mut()
                .find(|s| !s.revoked && constant_time_eq(s.token_hash.as_bytes(), hash.as_bytes()))
                .ok_or_else(|| api_error(StatusCode::UNAUTHORIZED, "invalid or expired session"))?;
            if session.expires_at_unix_ms < now
                || session
                    .last_activity_unix_ms
                    .saturating_add((policy.idle_ttl_minutes as u128) * 60_000)
                    < now
            {
                session.revoked = true;
                return Err(api_error(
                    StatusCode::UNAUTHORIZED,
                    "invalid or expired session",
                ));
            }
            let changed = touch && now.saturating_sub(session.last_activity_unix_ms) >= 60_000;
            if touch {
                session.last_activity_unix_ms = now;
            }
            let tuple = (
                session.account_id.clone(),
                session.mfa_verified,
                session.passkey_verified,
                session.created_at_unix_ms,
                session.last_activity_unix_ms,
            );
            drop(sessions);
            if changed {
                persist_state(state)?;
            }
            tuple
        };
    let account = state
        .accounts
        .lock()
        .map_err(lock_error)?
        .get(&account_id)
        .cloned()
        .ok_or_else(|| api_error(StatusCode::UNAUTHORIZED, "session account no longer exists"))?;
    if account.disabled {
        return Err(api_error(StatusCode::UNAUTHORIZED, "account disabled"));
    }
    let role = state
        .roles
        .lock()
        .map_err(lock_error)?
        .get(&account.role_id)
        .cloned()
        .ok_or_else(|| api_error(StatusCode::UNAUTHORIZED, "account role no longer exists"))?;
    let assurance = vsn_auth::SessionAssurance {
        authenticated: true,
        mfa_verified: mfa,
        passkey_verified: passkey,
        admin: role.permissions.iter().any(|p| {
            p == "*"
                || p == "control.iam.manage"
                || p == "control.auth.manage"
                || p == "control.scim.manage"
        }),
        authenticated_at_unix_ms: created_at,
        last_activity_unix_ms: last_activity,
    };
    if assurance.admin && vsn_auth::requires_step_up(&policy, &assurance, false) {
        return Err(api_error(
            StatusCode::FORBIDDEN,
            "account session does not satisfy current admin MFA policy",
        ));
    }
    Ok(AuthPrincipal {
        id: account.id,
        permissions: role.permissions.into_iter().collect(),
        bootstrap: false,
    })
}

fn cleanup_sessions(state: &AppState) -> Result<(), ApiError> {
    let now = vsn_remote::now_ms();
    let policy = state.auth_policy.lock().map_err(lock_error)?.clone();
    {
        let mut sessions = state.sessions.lock().map_err(lock_error)?;
        sessions.retain(|_, s| {
            !s.revoked
                && s.expires_at_unix_ms >= now
                && s.last_activity_unix_ms
                    .saturating_add((policy.idle_ttl_minutes as u128) * 60_000)
                    >= now
        });
        if sessions.len() > 10_000 {
            let mut ids = sessions
                .values()
                .map(|s| (s.last_activity_unix_ms, s.id.clone()))
                .collect::<Vec<_>>();
            ids.sort();
            for (_, id) in ids.into_iter().take(sessions.len().saturating_sub(10_000)) {
                sessions.remove(&id);
            }
        }
    }
    if let Some(store) = state.state_postgres.as_ref() {
        let _ = store.cleanup_sessions(24 * 60 * 60 * 1000).map_err(|e| {
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!("shared session cleanup failed: {e}"),
            )
        })?;
    }
    Ok(())
}
fn account_webauthn_uuid(account_id: &str) -> Uuid {
    let digest = Sha256::digest(format!("vsn-control-account:{account_id}").as_bytes());
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}
fn load_webauthn() -> Result<Option<Webauthn>, String> {
    let rp_id = match std::env::var("VSN_WEBAUTHN_RP_ID") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => return Ok(None),
    };
    let origin_raw = std::env::var("VSN_WEBAUTHN_ORIGIN").map_err(|_| {
        "VSN_WEBAUTHN_ORIGIN is required when VSN_WEBAUTHN_RP_ID is set".to_string()
    })?;
    let origin =
        Url::parse(&origin_raw).map_err(|e| format!("VSN_WEBAUTHN_ORIGIN is invalid: {e}"))?;
    if origin.scheme() != "https"
        && !matches!(origin.host_str(), Some("localhost") | Some("127.0.0.1"))
    {
        return Err("WebAuthn origin must use HTTPS except loopback development".into());
    }
    let builder = WebauthnBuilder::new(&rp_id, &origin)
        .map_err(|e| format!("WebAuthn relying-party configuration rejected: {e}"))?;
    builder
        .build()
        .map(Some)
        .map_err(|e| format!("WebAuthn build failed: {e}"))
}
fn load_team_vault_keyring() -> Result<TeamVaultKeyRing, String> {
    let mut keys: BTreeMap<String, [u8; 32]> = BTreeMap::new();
    if let Ok(raw) = std::env::var("VSN_CONTROL_VAULT_KEYRING_JSON") {
        let encoded: BTreeMap<String, String> = serde_json::from_str(&raw)
            .map_err(|e| format!("VSN_CONTROL_VAULT_KEYRING_JSON must be a JSON object: {e}"))?;
        if encoded.len() > 32 {
            return Err("team Vault keyring supports at most 32 loaded keys".into());
        }
        for (id, value) in encoded {
            validate_team_vault_key_id_str(&id)?;
            let bytes = B64
                .decode(value.trim())
                .map_err(|_| format!("team Vault key {id} is not valid base64"))?;
            let key: [u8; 32] = bytes
                .as_slice()
                .try_into()
                .map_err(|_| format!("team Vault key {id} must decode to exactly 32 bytes"))?;
            keys.insert(id, key);
        }
    } else if let Ok(raw) = std::env::var("VSN_CONTROL_VAULT_KEY_B64") {
        let bytes = B64
            .decode(raw.trim())
            .map_err(|_| "VSN_CONTROL_VAULT_KEY_B64 is not valid base64".to_string())?;
        let key: [u8; 32] = bytes
            .as_slice()
            .try_into()
            .map_err(|_| "VSN_CONTROL_VAULT_KEY_B64 must decode to exactly 32 bytes".to_string())?;
        keys.insert("legacy".into(), key);
    }
    let initial_active = if keys.is_empty() {
        None
    } else if let Ok(id) = std::env::var("VSN_CONTROL_VAULT_ACTIVE_KEY_ID") {
        validate_team_vault_key_id_str(&id)?;
        if !keys.contains_key(&id) {
            return Err(
                "VSN_CONTROL_VAULT_ACTIVE_KEY_ID is not present in VSN_CONTROL_VAULT_KEYRING_JSON"
                    .into(),
            );
        }
        Some(id)
    } else {
        keys.keys().next().cloned()
    };
    Ok(TeamVaultKeyRing {
        keys,
        initial_active,
    })
}
fn validate_team_vault_key_id_str(value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
    {
        Err("team Vault key id is invalid".into())
    } else {
        Ok(())
    }
}
fn validate_team_vault_key_id(value: &str) -> Result<(), ApiError> {
    validate_team_vault_key_id_str(value).map_err(|e| api_error(StatusCode::BAD_REQUEST, &e))
}
fn team_vault_active_key(
    state: &AppState,
    store: &vsn_control_store::PostgresSnapshotStore,
) -> Result<(String, [u8; 32]), ApiError> {
    let key_id = store
        .team_vault_active_key()
        .map_err(|e| {
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!("team Vault metadata failed: {e}"),
            )
        })?
        .or_else(|| state.team_vault_keys.initial_active.clone())
        .ok_or_else(|| {
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "team Vault keyring is not configured",
            )
        })?;
    let key = *state.team_vault_keys.keys.get(&key_id).ok_or_else(|| {
        api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            &format!("active team Vault key {key_id} is not loaded on this Control Plane"),
        )
    })?;
    if store
        .team_vault_active_key()
        .map_err(|e| {
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!("team Vault metadata failed: {e}"),
            )
        })?
        .is_none()
    {
        store.set_team_vault_active_key(&key_id).map_err(|e| {
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!("team Vault active key initialization failed: {e}"),
            )
        })?;
    }
    Ok((key_id, key))
}
fn encrypt_secret_bytes(key: &[u8; 32], plain: &[u8]) -> Result<EncryptedAuthSecret, ApiError> {
    if plain.len() > 1024 * 1024 {
        return Err(api_error(StatusCode::BAD_REQUEST, "secret too large"));
    }
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut nonce);
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce), plain)
        .map_err(|_| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "secret encryption failed",
            )
        })?;
    Ok(EncryptedAuthSecret {
        nonce_b64: B64.encode(nonce),
        ciphertext_b64: B64.encode(ciphertext),
    })
}
fn decrypt_secret_bytes(key: &[u8; 32], secret: &EncryptedAuthSecret) -> Result<Vec<u8>, ApiError> {
    let nonce = B64.decode(&secret.nonce_b64).map_err(|_| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "stored secret nonce is invalid",
        )
    })?;
    let ciphertext = B64.decode(&secret.ciphertext_b64).map_err(|_| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "stored secret ciphertext is invalid",
        )
    })?;
    if nonce.len() != 12 {
        return Err(api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "stored secret nonce length invalid",
        ));
    }
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    cipher
        .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
        .map_err(|_| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "secret decryption failed",
            )
        })
}
fn load_auth_encryption_key() -> Result<Option<[u8; 32]>, String> {
    let Ok(raw) = std::env::var("VSN_CONTROL_AUTH_KEY_B64") else {
        return Ok(None);
    };
    let bytes = B64
        .decode(raw.trim())
        .map_err(|_| "VSN_CONTROL_AUTH_KEY_B64 is not valid base64".to_string())?;
    let key: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| "VSN_CONTROL_AUTH_KEY_B64 must decode to exactly 32 bytes".to_string())?;
    Ok(Some(key))
}
fn encrypt_auth_secret(key: &[u8; 32], plain: &[u8]) -> Result<EncryptedAuthSecret, ApiError> {
    if plain.len() > 4096 {
        return Err(api_error(StatusCode::BAD_REQUEST, "auth secret too large"));
    }
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut nonce);
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce), plain)
        .map_err(|_| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "auth secret encryption failed",
            )
        })?;
    Ok(EncryptedAuthSecret {
        nonce_b64: B64.encode(nonce),
        ciphertext_b64: B64.encode(ciphertext),
    })
}
fn decrypt_auth_secret(key: &[u8; 32], secret: &EncryptedAuthSecret) -> Result<Vec<u8>, ApiError> {
    let nonce = B64.decode(&secret.nonce_b64).map_err(|_| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "stored auth nonce is invalid",
        )
    })?;
    let ciphertext = B64.decode(&secret.ciphertext_b64).map_err(|_| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "stored auth ciphertext is invalid",
        )
    })?;
    if nonce.len() != 12 {
        return Err(api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "stored auth nonce length invalid",
        ));
    }
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    cipher
        .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
        .map_err(|_| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "auth secret decryption failed",
            )
        })
}

fn dummy_password_hash() -> &'static str {
    use std::sync::OnceLock;
    static HASH: OnceLock<String> = OnceLock::new();
    HASH.get_or_init(|| {
        vsn_auth::hash_password("VSN dummy credential 8c3906f8bff441f1")
            .expect("dummy password hash")
    })
}

fn pct(value: &str) -> String {
    let mut out = String::new();
    for b in value.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
        } else {
            use std::fmt::Write as _;
            let _ = write!(&mut out, "%{b:02X}");
        }
    }
    out
}

fn hash_token(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    digest.iter().map(|b| format!("{b:02x}")).collect()
}
fn validate_id(value: &str) -> Result<(), ApiError> {
    if value.len() < 2
        || value.len() > 96
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    {
        Err(api_error(StatusCode::BAD_REQUEST, "invalid identifier"))
    } else {
        Ok(())
    }
}
fn validate_permission_string(value: &str) -> Result<(), ApiError> {
    if value == "*" {
        return Ok(());
    }
    if value.len() < 3
        || value.len() > 128
        || !value.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'_' | b'-')
        })
    {
        Err(api_error(
            StatusCode::BAD_REQUEST,
            "invalid permission string",
        ))
    } else {
        Ok(())
    }
}
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b) {
        diff |= *x ^ *y;
    }
    diff == 0
}
type ApiError = (StatusCode, Json<Value>);
fn lock_error<T>(_: std::sync::PoisonError<T>) -> ApiError {
    api_error(StatusCode::INTERNAL_SERVER_ERROR, "state lock poisoned")
}
fn remote_error(e: vsn_remote::RemoteError) -> ApiError {
    api_error(StatusCode::UNAUTHORIZED, &e.to_string())
}
fn api_error(status: StatusCode, message: &str) -> ApiError {
    (status, Json(json!({"error":message})))
}
fn random_id(prefix: &str) -> String {
    let mut bytes = [0u8; 16];
    OsRng.fill_bytes(&mut bytes);
    let mut out = format!("{prefix}_");
    for b in bytes {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{b:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replay_window_rejects_duplicate_and_evicts_old_entries() {
        let mut window = ReplayWindow::new(2);
        assert!(window.insert("a".into()));
        assert!(!window.insert("a".into()));
        assert!(window.insert("b".into()));
        assert!(window.insert("c".into()));
        assert!(window.insert("a".into()));
    }
    #[test]
    fn token_hash_is_stable() {
        assert_eq!(hash_token("abc"), hash_token("abc"));
        assert_ne!(hash_token("abc"), hash_token("abd"));
    }
    #[test]
    fn delivery_defaults_to_queued() {
        assert_eq!(DeliveryMeta::default().state, DeliveryState::Queued);
    }
    #[test]
    fn sensitive_permissions_require_approval() {
        assert!(approval_required("terminal.execute", "terminal.exec"));
        assert!(approval_required("files.write", "files.binary.write"));
        assert!(!approval_required("machine.view", "status"));
    }
}
