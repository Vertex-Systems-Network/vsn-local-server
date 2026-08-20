use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use ed25519_dalek::{Signer, SigningKey};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashSet, VecDeque},
    fs,
    net::TcpStream,
    path::Path,
    sync::Mutex,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use thiserror::Error;
use vsn_security::{DeviceIdentity, SecurityError};

pub const REMOTE_PROTOCOL_VERSION: u32 = 1;
pub const MAX_REMOTE_COMMAND_TTL_MS: u128 = 5 * 60 * 1000;
pub const MAX_CLOCK_SKEW_MS: u128 = 30 * 1000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceEnrollmentV1 {
    pub version: u32,
    pub device_id: String,
    pub public_key: String,
    pub display_name: String,
    pub os: String,
    pub pairing_nonce: String,
    pub proof_signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RemoteCommandV1 {
    pub version: u32,
    pub command_id: String,
    pub device_id: String,
    pub principal_id: String,
    pub issued_at_unix_ms: u128,
    pub expires_at_unix_ms: u128,
    pub permission: String,
    pub command: String,
    #[serde(default)]
    pub params: serde_json::Value,
    pub session_id: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentPollV1 {
    pub version: u32,
    pub device_id: String,
    pub public_key: String,
    pub timestamp_unix_ms: u128,
    pub nonce: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentPollResponseV1 {
    pub command: Option<RemoteCommandV1>,
    pub server_time_unix_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAuditBatchV1 {
    pub version: u32,
    pub device_id: String,
    pub events: Vec<vsn_audit::AuditEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentCommandResultV1 {
    pub version: u32,
    pub device_id: String,
    pub command_id: String,
    pub session_id: String,
    pub ok: bool,
    pub payload: serde_json::Value,
    pub timestamp_unix_ms: u128,
    pub nonce: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum AgentGatewayRequestV1 {
    Poll(AgentPollV1),
    Result(AgentCommandResultV1),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum AgentGatewayResponseV1 {
    Poll(AgentPollResponseV1),
    Ack { ok: bool, duplicate: bool },
    Error { message: String },
}


pub const STREAM_RELAY_PROTOCOL_VERSION:u32=2;
pub const MAX_STREAM_RELAY_FRAME_BYTES:usize=256*1024;
pub const STREAM_RELAY_RESUME_TTL_MS:u128=60_000;

#[derive(Debug,Clone,Serialize,Deserialize,PartialEq,Eq)]
pub struct AgentStreamHelloV1{
    pub version:u32,
    pub device_id:String,
    pub public_key:String,
    pub timestamp_unix_ms:u128,
    pub nonce:String,
    pub signature:String,
}

#[derive(Debug,Serialize)]
struct UnsignedAgentStreamHello<'a>{version:u32,device_id:&'a str,public_key:&'a str,timestamp_unix_ms:u128,nonce:&'a str}

impl AgentStreamHelloV1{
    fn canonical_bytes(&self)->Result<Vec<u8>,RemoteError>{Ok(serde_json::to_vec(&UnsignedAgentStreamHello{version:self.version,device_id:&self.device_id,public_key:&self.public_key,timestamp_unix_ms:self.timestamp_unix_ms,nonce:&self.nonce})?)}
}

pub fn build_agent_stream_hello(identity:&DeviceIdentity)->Result<AgentStreamHelloV1,RemoteError>{
    let meta=identity.metadata();let mut hello=AgentStreamHelloV1{version:STREAM_RELAY_PROTOCOL_VERSION,device_id:meta.device_id.clone(),public_key:meta.public_key.clone(),timestamp_unix_ms:now_ms(),nonce:random_id("streamhello"),signature:String::new()};hello.signature=identity.sign(&hello.canonical_bytes()?);Ok(hello)
}

pub fn verify_agent_stream_hello(hello:&AgentStreamHelloV1)->Result<(),RemoteError>{
    if hello.version!=STREAM_RELAY_PROTOCOL_VERSION{return Err(RemoteError::Version);}if !safe_id(&hello.device_id,8,160)||!safe_id(&hello.nonce,16,160){return Err(RemoteError::InvalidId);}validate_fresh_timestamp(hello.timestamp_unix_ms)?;DeviceIdentity::verify_with_public_key(&hello.public_key,&hello.canonical_bytes()?,&hello.signature)?;Ok(())
}

#[derive(Debug,Clone,Serialize,Deserialize,PartialEq,Eq)]
pub struct RelayStreamOpenV1{
    pub kind:vsn_stream::StreamKind,
    pub direction:vsn_stream::StreamDirection,
    pub resource_id:String,
    #[serde(default)]pub metadata:std::collections::HashMap<String,String>,
}

#[derive(Debug,Clone,Serialize,Deserialize,PartialEq,Eq)]
pub struct RelayStreamFrameV1{pub seq:u64,pub eof:bool,pub payload_base64:String}
impl RelayStreamFrameV1{pub fn decoded_len(&self)->Result<usize,RemoteError>{let bytes=B64.decode(&self.payload_base64).map_err(|_|RemoteError::Transport("relay frame payload is not base64".into()))?;if bytes.len()>MAX_STREAM_RELAY_FRAME_BYTES{return Err(RemoteError::Transport("relay frame exceeds 256 KiB".into()));}Ok(bytes.len())}}

#[derive(Debug,Clone,Serialize,Deserialize,PartialEq)]
#[serde(tag="type",content="payload",rename_all="snake_case")]
pub enum AgentStreamServerMessageV1{
    Open{relay_id:String,authorization:RemoteCommandV1,request:RelayStreamOpenV1},
    Input{relay_id:String,frame:RelayStreamFrameV1},
    Close{relay_id:String,reason:Option<String>},
    Ping{timestamp_unix_ms:u128},
}

#[derive(Debug,Clone,Serialize,Deserialize,PartialEq)]
#[serde(tag="type",content="payload",rename_all="snake_case")]
pub enum AgentStreamClientMessageV1{
    Hello(AgentStreamHelloV1),
    Opened{relay_id:String,ok:bool,stream_id:Option<String>,resource_id:Option<String>,error:Option<String>},
    Output{relay_id:String,frame:RelayStreamFrameV1},
    InputAck{relay_id:String,next_input_seq:u64,#[serde(default)]committed_bytes:Option<u64>,#[serde(default)]digest_sha256:Option<String>},
    Closed{relay_id:String,reason:Option<String>},
    Pong{timestamp_unix_ms:u128},
    Error{relay_id:Option<String>,message:String},
}

#[derive(Debug,Clone,Serialize,Deserialize,PartialEq,Eq)]
pub struct BrowserStreamResumeV1{
    pub relay_id:String,
    pub resume_token:String,
    #[serde(default)]pub last_output_seq:Option<u64>,
}

#[derive(Debug,Clone,Serialize,Deserialize,PartialEq)]
#[serde(tag="type",content="payload",rename_all="snake_case")]
pub enum BrowserStreamClientMessageV1{
    Hello{token:String,device_id:String,request:RelayStreamOpenV1,#[serde(default)]resume:Option<BrowserStreamResumeV1>},
    Input{frame:RelayStreamFrameV1},
    Close{reason:Option<String>},
    Ping{timestamp_unix_ms:u128},
}

#[derive(Debug,Clone,Serialize,Deserialize,PartialEq)]
#[serde(tag="type",content="payload",rename_all="snake_case")]
pub enum BrowserStreamServerMessageV1{
    Opened{relay_id:String,ok:bool,resource_id:Option<String>,error:Option<String>,#[serde(default)]resume_token:Option<String>,#[serde(default)]resumed:bool,#[serde(default)]next_input_seq:u64},
    Output{frame:RelayStreamFrameV1},
    InputAck{next_input_seq:u64,#[serde(default)]committed_bytes:Option<u64>,#[serde(default)]digest_sha256:Option<String>},
    Closed{reason:Option<String>},
    Pong{timestamp_unix_ms:u128},
    Error{message:String},
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ControlPlaneKeypair {
    pub private_key: String,
    pub public_key: String,
}

#[derive(Debug, Serialize)]
struct UnsignedRemoteCommand<'a> {
    version: u32,
    command_id: &'a str,
    device_id: &'a str,
    principal_id: &'a str,
    issued_at_unix_ms: u128,
    expires_at_unix_ms: u128,
    permission: &'a str,
    command: &'a str,
    params: &'a serde_json::Value,
    session_id: &'a str,
}

#[derive(Debug, Serialize)]
struct UnsignedAgentPoll<'a> {
    version: u32,
    device_id: &'a str,
    public_key: &'a str,
    timestamp_unix_ms: u128,
    nonce: &'a str,
}

#[derive(Debug, Serialize)]
struct UnsignedAgentResult<'a> {
    version: u32,
    device_id: &'a str,
    command_id: &'a str,
    session_id: &'a str,
    ok: bool,
    payload: &'a serde_json::Value,
    timestamp_unix_ms: u128,
    nonce: &'a str,
}

#[derive(Debug, Error)]
pub enum RemoteError {
    #[error("unsupported remote protocol version")]
    Version,
    #[error("remote command is for another device")]
    DeviceMismatch,
    #[error("remote command timing is invalid or expired")]
    Timing,
    #[error("remote command TTL exceeds policy")]
    Ttl,
    #[error("remote command replay detected")]
    Replay,
    #[error("invalid remote identifier")]
    InvalidId,
    #[error("remote signature verification failed: {0}")]
    Signature(#[from] SecurityError),
    #[error("canonical serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("remote transport error: {0}")]
    Transport(String),
    #[error("invalid control plane key: {0}")]
    Key(String),
}

pub fn build_device_enrollment(identity: &DeviceIdentity, pairing_nonce: &str) -> Result<DeviceEnrollmentV1, RemoteError> {
    if !safe_id(pairing_nonce, 32, 256) { return Err(RemoteError::InvalidId); }
    let meta = identity.metadata();
    let proof = enrollment_proof_bytes(&meta.device_id, &meta.public_key, pairing_nonce)?;
    Ok(DeviceEnrollmentV1 {
        version: REMOTE_PROTOCOL_VERSION,
        device_id: meta.device_id.clone(),
        public_key: meta.public_key.clone(),
        display_name: meta.display_name.clone(),
        os: meta.os.clone(),
        pairing_nonce: pairing_nonce.to_string(),
        proof_signature: identity.sign(&proof),
    })
}

pub fn verify_device_enrollment(enrollment: &DeviceEnrollmentV1) -> Result<(), RemoteError> {
    if enrollment.version != REMOTE_PROTOCOL_VERSION { return Err(RemoteError::Version); }
    if !safe_id(&enrollment.pairing_nonce, 32, 256) { return Err(RemoteError::InvalidId); }
    let derived_device_id = vsn_security::device_id_from_public_key_b64(&enrollment.public_key)?;
    if enrollment.device_id != derived_device_id { return Err(RemoteError::DeviceMismatch); }
    let proof = enrollment_proof_bytes(&enrollment.device_id, &enrollment.public_key, &enrollment.pairing_nonce)?;
    vsn_security::verify_signature(&enrollment.public_key, &proof, &enrollment.proof_signature)?;
    Ok(())
}

pub fn build_agent_poll(identity: &DeviceIdentity) -> Result<AgentPollV1, RemoteError> {
    let meta=identity.metadata();
    let mut poll=AgentPollV1{version:REMOTE_PROTOCOL_VERSION,device_id:meta.device_id.clone(),public_key:meta.public_key.clone(),timestamp_unix_ms:now_ms(),nonce:random_id("poll"),signature:String::new()};
    poll.signature=identity.sign(&poll.canonical_bytes()?); Ok(poll)
}

pub fn verify_agent_poll(poll:&AgentPollV1)->Result<(),RemoteError>{
    if poll.version!=REMOTE_PROTOCOL_VERSION{return Err(RemoteError::Version)}
    if !safe_id(&poll.nonce,16,128){return Err(RemoteError::InvalidId)}
    let derived=vsn_security::device_id_from_public_key_b64(&poll.public_key)?; if derived!=poll.device_id{return Err(RemoteError::DeviceMismatch)}
    validate_fresh_timestamp(poll.timestamp_unix_ms)?; vsn_security::verify_signature(&poll.public_key,&poll.canonical_bytes()?,&poll.signature)?; Ok(())
}

pub fn build_agent_result(identity:&DeviceIdentity,command:&RemoteCommandV1,ok:bool,payload:serde_json::Value)->Result<AgentCommandResultV1,RemoteError>{
    let mut result=AgentCommandResultV1{version:REMOTE_PROTOCOL_VERSION,device_id:identity.metadata().device_id.clone(),command_id:command.command_id.clone(),session_id:command.session_id.clone(),ok,payload,timestamp_unix_ms:now_ms(),nonce:random_id("result"),signature:String::new()};
    result.signature=identity.sign(&result.canonical_bytes()?); Ok(result)
}

pub fn verify_agent_result(result:&AgentCommandResultV1,public_key:&str)->Result<(),RemoteError>{
    if result.version!=REMOTE_PROTOCOL_VERSION{return Err(RemoteError::Version)} if !safe_id(&result.command_id,16,128)||!safe_id(&result.session_id,16,256)||!safe_id(&result.nonce,16,128){return Err(RemoteError::InvalidId)}
    validate_fresh_timestamp(result.timestamp_unix_ms)?; vsn_security::verify_signature(public_key,&result.canonical_bytes()?,&result.signature)?; Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryVerification { Fresh, Duplicate }

pub struct RemoteCommandVerifier {
    control_plane_public_key: String,
    expected_device_id: String,
    replay: Mutex<ReplayCache>,
}

impl RemoteCommandVerifier {
    pub fn new(control_plane_public_key: impl Into<String>, expected_device_id: impl Into<String>) -> Self {
        Self { control_plane_public_key: control_plane_public_key.into(), expected_device_id: expected_device_id.into(), replay: Mutex::new(ReplayCache::new(4096)) }
    }

    pub fn verify_delivery(&self, command: &RemoteCommandV1) -> Result<DeliveryVerification, RemoteError> {
        if command.version != REMOTE_PROTOCOL_VERSION { return Err(RemoteError::Version); }
        if command.device_id != self.expected_device_id { return Err(RemoteError::DeviceMismatch); }
        if !safe_id(&command.command_id, 16, 128) || !safe_id(&command.session_id, 16, 256) || command.principal_id.is_empty() || command.permission.is_empty() || command.command.is_empty() { return Err(RemoteError::InvalidId); }
        if command.expires_at_unix_ms < command.issued_at_unix_ms || command.expires_at_unix_ms - command.issued_at_unix_ms > MAX_REMOTE_COMMAND_TTL_MS { return Err(RemoteError::Ttl); }
        let now = now_ms();
        if command.issued_at_unix_ms > now.saturating_add(MAX_CLOCK_SKEW_MS) || command.expires_at_unix_ms.saturating_add(MAX_CLOCK_SKEW_MS) < now { return Err(RemoteError::Timing); }
        let canonical = command.canonical_bytes()?;
        vsn_security::verify_signature(&self.control_plane_public_key, &canonical, &command.signature)?;
        let mut replay = self.replay.lock().map_err(|_| RemoteError::Replay)?;
        if replay.insert(command.command_id.clone()) { Ok(DeliveryVerification::Fresh) } else { Ok(DeliveryVerification::Duplicate) }
    }

    pub fn verify(&self, command: &RemoteCommandV1) -> Result<(), RemoteError> {
        match self.verify_delivery(command)? { DeliveryVerification::Fresh => Ok(()), DeliveryVerification::Duplicate => Err(RemoteError::Replay) }
    }
}

impl RemoteCommandV1 {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, RemoteError> {
        Ok(serde_json::to_vec(&UnsignedRemoteCommand { version:self.version, command_id:&self.command_id, device_id:&self.device_id, principal_id:&self.principal_id, issued_at_unix_ms:self.issued_at_unix_ms, expires_at_unix_ms:self.expires_at_unix_ms, permission:&self.permission, command:&self.command, params:&self.params, session_id:&self.session_id })?)
    }
}
impl AgentPollV1{pub fn canonical_bytes(&self)->Result<Vec<u8>,RemoteError>{Ok(serde_json::to_vec(&UnsignedAgentPoll{version:self.version,device_id:&self.device_id,public_key:&self.public_key,timestamp_unix_ms:self.timestamp_unix_ms,nonce:&self.nonce})?)}}
impl AgentCommandResultV1{pub fn canonical_bytes(&self)->Result<Vec<u8>,RemoteError>{Ok(serde_json::to_vec(&UnsignedAgentResult{version:self.version,device_id:&self.device_id,command_id:&self.command_id,session_id:&self.session_id,ok:self.ok,payload:&self.payload,timestamp_unix_ms:self.timestamp_unix_ms,nonce:&self.nonce})?)}}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct AgentResultCacheFile { entries: Vec<AgentCommandResultV1> }

fn load_agent_result_cache(path:&Path)->Result<AgentResultCacheFile,RemoteError>{
    if !path.exists(){return Ok(AgentResultCacheFile::default());}
    let bytes=fs::read(path).map_err(|e|RemoteError::Transport(e.to_string()))?;
    serde_json::from_slice(&bytes).map_err(RemoteError::from)
}

fn save_agent_result_cache(path:&Path,cache:&AgentResultCacheFile)->Result<(),RemoteError>{
    if let Some(parent)=path.parent(){fs::create_dir_all(parent).map_err(|e|RemoteError::Transport(e.to_string()))?;}
    let tmp=path.with_extension("tmp");
    let mut bytes=serde_json::to_vec_pretty(cache)?;bytes.push(b'\n');
    fs::write(&tmp,bytes).map_err(|e|RemoteError::Transport(e.to_string()))?;
    if path.exists(){fs::remove_file(path).map_err(|e|RemoteError::Transport(e.to_string()))?;}
    fs::rename(tmp,path).map_err(|e|RemoteError::Transport(e.to_string()))?;Ok(())
}

pub fn list_cached_agent_results(path:&Path)->Result<Vec<AgentCommandResultV1>,RemoteError>{
    Ok(load_agent_result_cache(path)?.entries)
}

pub fn load_cached_agent_result(path:&Path, command_id:&str)->Result<Option<AgentCommandResultV1>,RemoteError>{
    Ok(load_agent_result_cache(path)?.entries.into_iter().rev().find(|entry|entry.command_id==command_id))
}

pub fn store_cached_agent_result(path:&Path,result:&AgentCommandResultV1)->Result<(),RemoteError>{
    let mut cache=load_agent_result_cache(path)?;
    cache.entries.retain(|entry|entry.command_id!=result.command_id);cache.entries.push(result.clone());
    if cache.entries.len()>2048{let drain=cache.entries.len()-2048;cache.entries.drain(0..drain);}
    save_agent_result_cache(path,&cache)
}

pub fn remove_cached_agent_result(path:&Path,command_id:&str)->Result<(),RemoteError>{
    let mut cache=load_agent_result_cache(path)?;
    let before=cache.entries.len();cache.entries.retain(|entry|entry.command_id!=command_id);
    if cache.entries.len()!=before{save_agent_result_cache(path,&cache)?;}
    Ok(())
}

pub fn refresh_agent_result(identity:&DeviceIdentity,prior:&AgentCommandResultV1)->Result<AgentCommandResultV1,RemoteError>{
    if prior.device_id!=identity.metadata().device_id{return Err(RemoteError::DeviceMismatch);}
    if !safe_id(&prior.command_id,16,128)||!safe_id(&prior.session_id,16,256){return Err(RemoteError::InvalidId);}
    let mut result=AgentCommandResultV1{version:REMOTE_PROTOCOL_VERSION,device_id:prior.device_id.clone(),command_id:prior.command_id.clone(),session_id:prior.session_id.clone(),ok:prior.ok,payload:prior.payload.clone(),timestamp_unix_ms:now_ms(),nonce:random_id("result"),signature:String::new()};
    result.signature=identity.sign(&result.canonical_bytes()?);Ok(result)
}

pub fn generate_control_plane_keypair()->ControlPlaneKeypair{let signing=SigningKey::generate(&mut OsRng);ControlPlaneKeypair{private_key:B64.encode(signing.to_bytes()),public_key:B64.encode(signing.verifying_key().to_bytes())}}
pub fn control_plane_public_key(private_key_b64:&str)->Result<String,RemoteError>{let signing=decode_signing_key(private_key_b64)?;Ok(B64.encode(signing.verifying_key().to_bytes()))}
pub fn sign_remote_command(private_key_b64:&str,command:&mut RemoteCommandV1)->Result<(),RemoteError>{let signing=decode_signing_key(private_key_b64)?;command.signature=B64.encode(signing.sign(&command.canonical_bytes()?).to_bytes());Ok(())}


pub struct AgentStreamRelayClient{
    socket:tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<TcpStream>>,
}
impl AgentStreamRelayClient{
    pub fn connect(base_url:&str,identity:&DeviceIdentity)->Result<Self,RemoteError>{
        let url=stream_gateway_url(base_url,"/v1/agent/streams/ws")?;let(socket,_)=tungstenite::connect(url.as_str()).map_err(|e|RemoteError::Transport(format!("stream gateway connect failed: {e}")))?;let mut out=Self{socket};out.send_client(&AgentStreamClientMessageV1::Hello(build_agent_stream_hello(identity)?))?;Ok(out)
    }
    pub fn read_server(&mut self)->Result<AgentStreamServerMessageV1,RemoteError>{
        loop{let message=self.socket.read().map_err(|e|RemoteError::Transport(format!("stream gateway read failed: {e}")))?;match message{
            tungstenite::Message::Text(text)=>{if text.len()>2*1024*1024{return Err(RemoteError::Transport("stream gateway message exceeds 2 MiB".into()));}return serde_json::from_str(text.as_str()).map_err(RemoteError::from)},
            tungstenite::Message::Binary(bytes)=>{if bytes.len()>2*1024*1024{return Err(RemoteError::Transport("stream gateway message exceeds 2 MiB".into()));}return serde_json::from_slice(&bytes).map_err(RemoteError::from)},
            tungstenite::Message::Ping(v)=>{self.socket.send(tungstenite::Message::Pong(v)).map_err(|e|RemoteError::Transport(format!("stream gateway pong failed: {e}")))?;},
            tungstenite::Message::Pong(_)|tungstenite::Message::Frame(_)=>{},
            tungstenite::Message::Close(_)=>return Err(RemoteError::Transport("stream gateway closed connection".into())),
        }}
    }
    pub fn send_client(&mut self,message:&AgentStreamClientMessageV1)->Result<(),RemoteError>{let text=serde_json::to_string(message)?;if text.len()>2*1024*1024{return Err(RemoteError::Transport("stream gateway message exceeds 2 MiB".into()));}self.socket.send(tungstenite::Message::Text(text.into())).map_err(|e|RemoteError::Transport(format!("stream gateway send failed: {e}"))) }
}
fn stream_gateway_url(base_url:&str,path:&str)->Result<String,RemoteError>{
    validate_control_plane_url(base_url)?;let base=base_url.trim_end_matches('/');if let Some(rest)=base.strip_prefix("https://"){return Ok(format!("wss://{rest}{path}"));}if let Some(rest)=base.strip_prefix("http://127.0.0.1"){return Ok(format!("ws://127.0.0.1{rest}{path}"));}if let Some(rest)=base.strip_prefix("http://localhost"){return Ok(format!("ws://localhost{rest}{path}"));}Err(RemoteError::Transport("control plane URL cannot be converted to a secure stream gateway URL".into()))
}

pub struct WebSocketControlPlaneClient {
    socket: tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<TcpStream>>,
}
impl WebSocketControlPlaneClient {
    pub fn new(base_url: &str) -> Result<Self, RemoteError> {
        let url = gateway_url(base_url)?;
        let (socket, _) = tungstenite::connect(url.as_str()).map_err(|e|RemoteError::Transport(format!("gateway connect failed: {e}")))?;
        Ok(Self { socket })
    }

    pub fn poll(&mut self, poll: &AgentPollV1) -> Result<AgentPollResponseV1, RemoteError> {
        match self.request(&AgentGatewayRequestV1::Poll(poll.clone()))? {
            AgentGatewayResponseV1::Poll(response) => Ok(response),
            AgentGatewayResponseV1::Error { message } => Err(RemoteError::Transport(format!("gateway rejected poll: {message}"))),
            other => Err(RemoteError::Transport(format!("unexpected gateway response to poll: {other:?}"))),
        }
    }

    pub fn submit_result(&mut self, result: &AgentCommandResultV1) -> Result<(), RemoteError> {
        match self.request(&AgentGatewayRequestV1::Result(result.clone()))? {
            AgentGatewayResponseV1::Ack { ok: true, .. } => Ok(()),
            AgentGatewayResponseV1::Ack { ok: false, .. } => Err(RemoteError::Transport("gateway returned negative acknowledgement".into())),
            AgentGatewayResponseV1::Error { message } => Err(RemoteError::Transport(format!("gateway rejected result: {message}"))),
            other => Err(RemoteError::Transport(format!("unexpected gateway response to result: {other:?}"))),
        }
    }

    fn request(&mut self, request: &AgentGatewayRequestV1) -> Result<AgentGatewayResponseV1, RemoteError> {
        let text = serde_json::to_string(request)?;
        if text.len() > 2 * 1024 * 1024 {
            return Err(RemoteError::Transport("gateway request exceeds 2 MiB safety limit".into()));
        }
        self.socket.send(tungstenite::Message::Text(text.into())).map_err(|e|RemoteError::Transport(format!("gateway send failed: {e}")))?;
        loop {
            let message = self.socket.read().map_err(|e|RemoteError::Transport(format!("gateway read failed: {e}")))?;
            match message {
                tungstenite::Message::Text(text) => {
                    if text.len() > 2 * 1024 * 1024 { return Err(RemoteError::Transport("gateway response exceeds 2 MiB safety limit".into())); }
                    return serde_json::from_str(text.as_str()).map_err(RemoteError::from);
                }
                tungstenite::Message::Binary(bytes) => {
                    if bytes.len() > 2 * 1024 * 1024 { return Err(RemoteError::Transport("gateway response exceeds 2 MiB safety limit".into())); }
                    return serde_json::from_slice(&bytes).map_err(RemoteError::from);
                }
                tungstenite::Message::Close(_) => return Err(RemoteError::Transport("gateway closed the connection".into())),
                tungstenite::Message::Ping(_) | tungstenite::Message::Pong(_) | tungstenite::Message::Frame(_) => continue,
            }
        }
    }
}

fn gateway_url(base_url: &str) -> Result<String, RemoteError> {
    validate_control_plane_url(base_url)?;
    let base = base_url.trim_end_matches('/');
    if let Some(rest) = base.strip_prefix("https://") { return Ok(format!("wss://{rest}/v1/agent/ws")); }
    if let Some(rest) = base.strip_prefix("http://127.0.0.1") { return Ok(format!("ws://127.0.0.1{rest}/v1/agent/ws")); }
    if let Some(rest) = base.strip_prefix("http://localhost") { return Ok(format!("ws://localhost{rest}/v1/agent/ws")); }
    Err(RemoteError::Transport("control plane URL cannot be converted to a secure gateway URL".into()))
}

#[derive(Clone)]
pub struct HttpControlPlaneClient { base_url:String, client:reqwest::blocking::Client }
impl HttpControlPlaneClient {
    pub fn new(base_url:&str)->Result<Self,RemoteError>{validate_control_plane_url(base_url)?;let client=reqwest::blocking::Client::builder().timeout(Duration::from_secs(20)).build().map_err(|e|RemoteError::Transport(e.to_string()))?;Ok(Self{base_url:base_url.trim_end_matches('/').into(),client})}
    pub fn enroll(&self,enrollment:&DeviceEnrollmentV1)->Result<(),RemoteError>{let response=self.client.post(format!("{}/v1/devices/enroll",self.base_url)).json(enrollment).send().map_err(http_err)?;ensure_success(response)?;Ok(())}
    pub fn poll(&self,poll:&AgentPollV1)->Result<AgentPollResponseV1,RemoteError>{let response=self.client.post(format!("{}/v1/agent/poll",self.base_url)).json(poll).send().map_err(http_err)?;let response=ensure_success(response)?;response.json().map_err(http_err)}
    pub fn submit_result(&self,result:&AgentCommandResultV1)->Result<(),RemoteError>{let response=self.client.post(format!("{}/v1/agent/result",self.base_url)).json(result).send().map_err(http_err)?;ensure_success(response)?;Ok(())}
    pub fn submit_audit(&self,batch:&AgentAuditBatchV1)->Result<(),RemoteError>{let response=self.client.post(format!("{}/v1/agent/audit",self.base_url)).json(batch).send().map_err(http_err)?;ensure_success(response)?;Ok(())}
}

fn ensure_success(response:reqwest::blocking::Response)->Result<reqwest::blocking::Response,RemoteError>{let status=response.status();if !status.is_success(){let body=response.text().unwrap_or_default();return Err(RemoteError::Transport(format!("control plane returned {status}: {body}")));}Ok(response)}
fn http_err<E:std::fmt::Display>(e:E)->RemoteError{RemoteError::Transport(e.to_string())}
fn validate_control_plane_url(value:&str)->Result<(),RemoteError>{if value.starts_with("https://")||value.starts_with("http://127.0.0.1")||value.starts_with("http://localhost"){Ok(())}else{Err(RemoteError::Transport("control plane URL must use HTTPS except loopback development".into()))}}
fn decode_signing_key(value:&str)->Result<SigningKey,RemoteError>{let bytes=B64.decode(value).map_err(|e|RemoteError::Key(e.to_string()))?;let bytes:[u8;32]=bytes.as_slice().try_into().map_err(|_|RemoteError::Key("private key must be 32 bytes".into()))?;Ok(SigningKey::from_bytes(&bytes))}
fn enrollment_proof_bytes(device_id:&str,public_key:&str,pairing_nonce:&str)->Result<Vec<u8>,RemoteError>{#[derive(Serialize)]struct Proof<'a>{version:u32,purpose:&'static str,device_id:&'a str,public_key:&'a str,pairing_nonce:&'a str}Ok(serde_json::to_vec(&Proof{version:REMOTE_PROTOCOL_VERSION,purpose:"vsn-device-enrollment",device_id,public_key,pairing_nonce})?)}
fn validate_fresh_timestamp(timestamp:u128)->Result<(),RemoteError>{let now=now_ms();if timestamp>now.saturating_add(MAX_CLOCK_SKEW_MS)||timestamp.saturating_add(MAX_CLOCK_SKEW_MS)<now{Err(RemoteError::Timing)}else{Ok(())}}
fn safe_id(value:&str,min:usize,max:usize)->bool{value.len()>=min&&value.len()<=max&&value.bytes().all(|b|b.is_ascii_alphanumeric()||matches!(b,b'-'|b'_'|b'.'|b':'))}
fn random_id(prefix:&str)->String{let mut bytes=[0u8;16];OsRng.fill_bytes(&mut bytes);let mut out=String::with_capacity(prefix.len()+1+32);out.push_str(prefix);out.push('_');for b in bytes{use std::fmt::Write as _;let _=write!(&mut out,"{b:02x}");}out}
pub fn now_ms()->u128{SystemTime::now().duration_since(UNIX_EPOCH).map(|d|d.as_millis()).unwrap_or(0)}

struct ReplayCache{capacity:usize,order:VecDeque<String>,seen:HashSet<String>}
impl ReplayCache{fn new(capacity:usize)->Self{Self{capacity,order:VecDeque::new(),seen:HashSet::new()}}fn insert(&mut self,id:String)->bool{if self.seen.contains(&id){return false;}self.seen.insert(id.clone());self.order.push_back(id);while self.order.len()>self.capacity{if let Some(old)=self.order.pop_front(){self.seen.remove(&old);}}true}}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replay_cache_rejects_duplicate() {
        let mut c = ReplayCache::new(2);
        assert!(c.insert("abcdefghijklmnop".into()));
        assert!(!c.insert("abcdefghijklmnop".into()));
    }

    #[test]
    fn unsafe_ids_are_rejected() {
        assert!(!safe_id("aaaaaaaaaaaaaaa/", 16, 128));
        assert!(safe_id("0123456789abcdef", 16, 128));
    }

    #[test]
    fn generated_control_plane_key_can_sign() {
        let pair = generate_control_plane_keypair();
        let mut cmd = RemoteCommandV1 {
            version: 1,
            command_id: "0123456789abcdef".into(),
            device_id: "dev_x".into(),
            principal_id: "admin".into(),
            issued_at_unix_ms: now_ms(),
            expires_at_unix_ms: now_ms() + 1000,
            permission: "machine.view".into(),
            command: "status".into(),
            params: serde_json::Value::Null,
            session_id: "0123456789abcdef".into(),
            signature: String::new(),
        };
        assert!(sign_remote_command(&pair.private_key, &mut cmd).is_ok());
        assert!(!cmd.signature.is_empty());
    }

    #[test]
    fn durable_result_cache_can_list_and_ack() {
        let path = std::env::temp_dir().join(format!("vsn-result-cache-{}.json", now_ms()));
        let result = AgentCommandResultV1 {
            version: REMOTE_PROTOCOL_VERSION,
            device_id: "dev_0123456789abcdef".into(),
            command_id: "cmd_0123456789abcdef".into(),
            session_id: "session_0123456789abcdef".into(),
            ok: true,
            payload: serde_json::json!({"value": 1}),
            timestamp_unix_ms: now_ms(),
            nonce: "result_0123456789abcdef".into(),
            signature: "test-signature".into(),
        };
        store_cached_agent_result(&path, &result).unwrap();
        assert_eq!(list_cached_agent_results(&path).unwrap().len(), 1);
        assert!(load_cached_agent_result(&path, &result.command_id).unwrap().is_some());
        remove_cached_agent_result(&path, &result.command_id).unwrap();
        assert!(list_cached_agent_results(&path).unwrap().is_empty());
        let _ = std::fs::remove_file(path);
    }
}
