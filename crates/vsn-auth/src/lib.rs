use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthPolicyError {
    #[error("invalid enterprise auth configuration: {0}")]
    Invalid(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OidcProviderConfig {
    pub id: String,
    pub issuer: String,
    pub client_id: String,
    pub redirect_url: String,
    #[serde(default)] pub authorization_endpoint: Option<String>,
    #[serde(default)] pub end_session_endpoint: Option<String>,
    #[serde(default)] pub post_logout_redirect_url: Option<String>,
    #[serde(default)] pub client_secret_env: Option<String>,
    #[serde(default)] pub mfa_assured: bool,
    #[serde(default = "default_scopes")] pub scopes: Vec<String>,
    #[serde(default = "default_true")] pub pkce_required: bool,
}
fn default_true()->bool{true}
fn default_scopes()->Vec<String>{vec!["openid".into(),"profile".into(),"email".into()]}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SamlProviderConfig {
    pub id: String,
    /// VSN/SP entity ID used as AuthnRequest Issuer.
    pub entity_id: String,
    /// Expected IdP entity ID. Empty legacy values are rejected by validation.
    #[serde(default)] pub idp_entity_id: String,
    pub sso_url: String,
    #[serde(default)] pub slo_url: Option<String>,
    pub acs_url: String,
    pub audience: String,
    pub x509_certificate_pem_env: String,
    #[serde(default = "default_saml_name_id")] pub name_id_format: String,
    #[serde(default = "default_email_attribute")] pub email_attribute: String,
    #[serde(default)] pub mfa_assured: bool,
}
fn default_saml_name_id()->String{"urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress".into()}
fn default_email_attribute()->String{"email".into()}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all="snake_case")]
pub enum MfaFactor { Passkey, Totp, SecurityKey, RecoveryCode }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnterpriseAuthPolicy {
    #[serde(default = "default_session_ttl")] pub session_ttl_minutes: u32,
    #[serde(default = "default_idle_ttl")] pub idle_ttl_minutes: u32,
    #[serde(default = "default_true")] pub require_mfa_for_admin: bool,
    #[serde(default = "default_true")] pub require_step_up_for_high_risk: bool,
    #[serde(default)] pub require_passkey_for_high_risk: bool,
    #[serde(default)] pub allowed_factors: Vec<MfaFactor>,
    #[serde(default)] pub oidc_providers: Vec<OidcProviderConfig>,
    #[serde(default)] pub saml_providers: Vec<SamlProviderConfig>,
}
fn default_session_ttl()->u32{480}
fn default_idle_ttl()->u32{60}
impl Default for EnterpriseAuthPolicy {
    fn default()->Self{Self{session_ttl_minutes:default_session_ttl(),idle_ttl_minutes:default_idle_ttl(),require_mfa_for_admin:true,require_step_up_for_high_risk:true,require_passkey_for_high_risk:false,allowed_factors:vec![MfaFactor::Passkey,MfaFactor::Totp,MfaFactor::SecurityKey,MfaFactor::RecoveryCode],oidc_providers:vec![],saml_providers:vec![]}}
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionAssurance {
    pub authenticated: bool,
    pub mfa_verified: bool,
    pub passkey_verified: bool,
    pub admin: bool,
    pub authenticated_at_unix_ms: u128,
    pub last_activity_unix_ms: u128,
}

pub fn validate_policy(policy:&EnterpriseAuthPolicy)->Result<(),AuthPolicyError>{
    if !(5..=10_080).contains(&policy.session_ttl_minutes){return Err(AuthPolicyError::Invalid("session_ttl_minutes must be 5..10080".into()));}
    if !(1..=1_440).contains(&policy.idle_ttl_minutes)||policy.idle_ttl_minutes>policy.session_ttl_minutes{return Err(AuthPolicyError::Invalid("idle_ttl_minutes must be <= session ttl".into()));}
    if policy.allowed_factors.is_empty(){return Err(AuthPolicyError::Invalid("at least one MFA factor must be allowed".into()));}
    if policy.require_passkey_for_high_risk&&!policy.allowed_factors.contains(&MfaFactor::Passkey){return Err(AuthPolicyError::Invalid("passkey is required for high-risk actions but not allowed".into()));}
    if policy.oidc_providers.len()>32{return Err(AuthPolicyError::Invalid("too many OIDC providers".into()));}
    if policy.saml_providers.len()>32{return Err(AuthPolicyError::Invalid("too many SAML providers".into()));}
    for provider in &policy.oidc_providers{validate_oidc(provider)?;}
    for provider in &policy.saml_providers{validate_saml(provider)?;}
    Ok(())
}

pub fn validate_oidc(provider:&OidcProviderConfig)->Result<(),AuthPolicyError>{
    safe_id(&provider.id)?;
    if !is_secure_url(&provider.issuer,false){return Err(AuthPolicyError::Invalid("OIDC issuer must use HTTPS".into()));}
    if provider.client_id.trim().is_empty()||provider.client_id.len()>512{return Err(AuthPolicyError::Invalid("OIDC client_id is invalid".into()));}
    if !is_secure_url(&provider.redirect_url,true){return Err(AuthPolicyError::Invalid("OIDC redirect_url must use HTTPS except loopback development".into()));}
    if let Some(endpoint)=&provider.authorization_endpoint{if !is_secure_url(endpoint,false){return Err(AuthPolicyError::Invalid("OIDC authorization_endpoint must use HTTPS".into()));}}
    if let Some(endpoint)=&provider.end_session_endpoint{if !is_secure_url(endpoint,false){return Err(AuthPolicyError::Invalid("OIDC end_session_endpoint must use HTTPS".into()));}}
    if let Some(url)=&provider.post_logout_redirect_url{if !is_secure_url(url,true){return Err(AuthPolicyError::Invalid("OIDC post_logout_redirect_url must use HTTPS except loopback development".into()));}}
    if let Some(name)=&provider.client_secret_env{if name.len()<3||name.len()>128||!name.bytes().all(|b|b.is_ascii_uppercase()||b.is_ascii_digit()||b==b'_'){return Err(AuthPolicyError::Invalid("OIDC client_secret_env must be an uppercase environment-variable name".into()));}}
    if !provider.pkce_required{return Err(AuthPolicyError::Invalid("OIDC PKCE must remain enabled".into()));}
    if !provider.scopes.iter().any(|v|v=="openid"){return Err(AuthPolicyError::Invalid("OIDC openid scope is required".into()));}
    if provider.scopes.len()>32||provider.scopes.iter().any(|v|v.is_empty()||v.len()>96){return Err(AuthPolicyError::Invalid("OIDC scope list is invalid".into()));}
    Ok(())
}

pub fn validate_saml(provider:&SamlProviderConfig)->Result<(),AuthPolicyError>{
    safe_id(&provider.id)?;
    if provider.entity_id.trim().is_empty()||provider.entity_id.len()>1024||provider.entity_id.chars().any(|c|c.is_control()){return Err(AuthPolicyError::Invalid("SAML SP entity_id is invalid".into()));}
    if provider.idp_entity_id.trim().is_empty()||provider.idp_entity_id.len()>1024||provider.idp_entity_id.chars().any(|c|c.is_control()){return Err(AuthPolicyError::Invalid("SAML idp_entity_id is required and invalid".into()));}
    if !is_secure_url(&provider.sso_url,false){return Err(AuthPolicyError::Invalid("SAML SSO URL must use HTTPS".into()));}
    if let Some(url)=&provider.slo_url{if !is_secure_url(url,false){return Err(AuthPolicyError::Invalid("SAML SLO URL must use HTTPS".into()));}}
    if !is_secure_url(&provider.acs_url,true){return Err(AuthPolicyError::Invalid("SAML ACS URL must use HTTPS except loopback development".into()));}
    if provider.audience.trim().is_empty()||provider.audience.len()>1024||provider.audience.chars().any(|c|c.is_control()){return Err(AuthPolicyError::Invalid("SAML audience is invalid".into()));}
    let env=&provider.x509_certificate_pem_env;
    if env.len()<3||env.len()>128||!env.bytes().all(|b|b.is_ascii_uppercase()||b.is_ascii_digit()||b==b'_'){return Err(AuthPolicyError::Invalid("SAML certificate environment variable name is invalid".into()));}
    if provider.name_id_format.trim().is_empty()||provider.name_id_format.len()>512{return Err(AuthPolicyError::Invalid("SAML NameID format is invalid".into()));}
    if provider.email_attribute.trim().is_empty()||provider.email_attribute.len()>256||provider.email_attribute.chars().any(|c|c.is_control()){return Err(AuthPolicyError::Invalid("SAML email attribute is invalid".into()));}
    Ok(())
}

pub fn requires_step_up(policy:&EnterpriseAuthPolicy, session:&SessionAssurance, high_risk:bool)->bool{
    if !session.authenticated{return true;}
    if session.admin&&policy.require_mfa_for_admin&&!session.mfa_verified{return true;}
    if high_risk&&policy.require_step_up_for_high_risk&&!session.mfa_verified{return true;}
    if high_risk&&policy.require_passkey_for_high_risk&&!session.passkey_verified{return true;}
    false
}

fn safe_id(value:&str)->Result<(),AuthPolicyError>{if value.len()<2||value.len()>96||!value.bytes().all(|b|b.is_ascii_alphanumeric()||matches!(b,b'-'|b'_'|b'.')){Err(AuthPolicyError::Invalid("provider id must be a safe identifier".into()))}else{Ok(())}}
fn is_secure_url(value:&str,loopback_ok:bool)->bool{value.starts_with("https://")||(loopback_ok&&(value.starts_with("http://127.0.0.1")||value.starts_with("http://localhost")))}



#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TotpEnrollment {
    pub secret_base32: String,
    pub otpauth_url: String,
    pub issuer: String,
    pub account_name: String,
    pub digits: u8,
    pub step_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TotpVerification {
    pub valid: bool,
    pub matched_step: Option<u64>,
}

/// Hash a user password into a PHC string using Argon2id defaults.
/// The caller is responsible for rate limiting and breach-password policy.
pub fn hash_password(password: &str) -> Result<String, AuthPolicyError> {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };
    validate_password_input(password)?;
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| AuthPolicyError::Invalid(format!("password hashing failed: {e}")))
}

pub fn verify_password(password: &str, phc_hash: &str) -> Result<bool, AuthPolicyError> {
    use argon2::{
        password_hash::{PasswordHash, PasswordVerifier},
        Argon2,
    };
    if password.len() > 4096 || phc_hash.len() > 4096 {
        return Err(AuthPolicyError::Invalid("password credential exceeds safety limit".into()));
    }
    let parsed = PasswordHash::new(phc_hash)
        .map_err(|_| AuthPolicyError::Invalid("stored password hash is malformed".into()))?;
    Ok(Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok())
}

/// Generate a 160-bit TOTP secret and standards-compatible otpauth URL.
/// The returned secret must be encrypted at rest by the caller (e.g. VSN Vault).
pub fn create_totp_enrollment(account_name: &str, issuer: &str) -> Result<TotpEnrollment, AuthPolicyError> {
    use rand_core::{OsRng, RngCore};
    use totp_rs::{Algorithm, Builder};
    validate_totp_label(account_name, "account name")?;
    validate_totp_label(issuer, "issuer")?;
    let mut secret = [0u8; 20];
    OsRng.fill_bytes(&mut secret);
    let totp = Builder::new()
        .with_algorithm(Algorithm::SHA1)
        .with_digits(6)
        .with_skew(1)
        .with_step_duration(30)
        .with_secret(secret.to_vec())
        .with_account_name(account_name)
        .with_issuer(Some(issuer))
        .build()
        .map_err(|e| AuthPolicyError::Invalid(format!("TOTP enrollment failed: {e}")))?;
    let url = totp.to_url()
        .map_err(|e| AuthPolicyError::Invalid(format!("TOTP URL generation failed: {e}")))?;
    Ok(TotpEnrollment {
        secret_base32: data_encoding::BASE32_NOPAD.encode(&secret),
        otpauth_url: url,
        issuer: issuer.to_string(),
        account_name: account_name.to_string(),
        digits: 6,
        step_seconds: 30,
    })
}

/// Verify a TOTP code and return the matched time-step. Persist the matched step per
/// credential and reject a repeated step to prevent OTP replay.
pub fn verify_totp(secret_base32: &str, token: &str) -> Result<TotpVerification, AuthPolicyError> {
    use totp_rs::{Algorithm, Builder};
    if token.len() != 6 || !token.bytes().all(|b| b.is_ascii_digit()) {
        return Ok(TotpVerification { valid: false, matched_step: None });
    }
    if secret_base32.len() > 256 {
        return Err(AuthPolicyError::Invalid("TOTP secret exceeds safety limit".into()));
    }
    let secret = data_encoding::BASE32_NOPAD
        .decode(secret_base32.trim().to_ascii_uppercase().as_bytes())
        .map_err(|_| AuthPolicyError::Invalid("TOTP secret is not valid base32".into()))?;
    if secret.len() < 16 || secret.len() > 64 {
        return Err(AuthPolicyError::Invalid("TOTP secret length is invalid".into()));
    }
    let totp = Builder::new()
        .with_algorithm(Algorithm::SHA1)
        .with_digits(6)
        .with_skew(1)
        .with_step_duration(30)
        .with_secret(secret)
        .build()
        .map_err(|e| AuthPolicyError::Invalid(format!("TOTP verification setup failed: {e}")))?;
    let matched_step = totp.check_current(token);
    Ok(TotpVerification { valid: matched_step.is_some(), matched_step })
}

fn validate_password_input(password: &str) -> Result<(), AuthPolicyError> {
    // Length is deliberately a guardrail, not a prescriptive composition policy.
    if password.len() < 12 {
        return Err(AuthPolicyError::Invalid("password must be at least 12 characters".into()));
    }
    if password.len() > 4096 {
        return Err(AuthPolicyError::Invalid("password exceeds safety limit".into()));
    }
    Ok(())
}

fn validate_totp_label(value: &str, field: &str) -> Result<(), AuthPolicyError> {
    let value = value.trim();
    if value.is_empty() || value.len() > 254 || value.contains(':') || value.chars().any(|c| c.is_control()) {
        Err(AuthPolicyError::Invalid(format!("TOTP {field} is invalid")))
    } else {
        Ok(())
    }
}

#[cfg(test)] mod tests{
    use super::*;
    #[test] fn oidc_requires_https_and_pkce(){let mut p=OidcProviderConfig{id:"corp".into(),issuer:"https://id.example.com".into(),client_id:"vsn".into(),redirect_url:"https://control.example.com/callback".into(),authorization_endpoint:Some("https://id.example.com/authorize".into()),end_session_endpoint:Some("https://id.example.com/logout".into()),post_logout_redirect_url:Some("https://control.example.com/logout-complete".into()),client_secret_env:None,mfa_assured:false,scopes:default_scopes(),pkce_required:true};assert!(validate_oidc(&p).is_ok());p.pkce_required=false;assert!(validate_oidc(&p).is_err());}
    #[test] fn password_hash_roundtrip(){let h=hash_password("correct horse battery staple").unwrap();assert!(verify_password("correct horse battery staple",&h).unwrap());assert!(!verify_password("wrong-password-value",&h).unwrap());}
    #[test] fn totp_enrollment_has_secret_and_url(){let e=create_totp_enrollment("user@example.com","VSN").unwrap();assert!(!e.secret_base32.is_empty());assert!(e.otpauth_url.starts_with("otpauth://totp/"));}
    #[test] fn high_risk_can_require_passkey(){let mut p=EnterpriseAuthPolicy::default();p.require_passkey_for_high_risk=true;let s=SessionAssurance{authenticated:true,mfa_verified:true,passkey_verified:false,admin:true,authenticated_at_unix_ms:0,last_activity_unix_ms:0};assert!(requires_step_up(&p,&s,true));}
    #[test] fn saml_policy_requires_https_and_secret_indirection(){let p=SamlProviderConfig{id:"corp-saml".into(),entity_id:"urn:vsn:corp".into(),idp_entity_id:"https://id.example.com/entity".into(),sso_url:"https://id.example.com/sso".into(),slo_url:Some("https://id.example.com/slo".into()),acs_url:"https://control.example.com/v1/auth/saml/callback".into(),audience:"urn:vsn:corp".into(),x509_certificate_pem_env:"VSN_SAML_CORP_CERT".into(),name_id_format:default_saml_name_id(),email_attribute:"email".into(),mfa_assured:true};assert!(validate_saml(&p).is_ok());}
}

#[derive(Debug,Clone,Serialize,Deserialize,PartialEq,Eq)]
pub struct RecoveryCodeSet{pub codes:Vec<String>,pub hashes:Vec<String>}
#[derive(Debug,Clone,Serialize,Deserialize,PartialEq,Eq)]
pub struct OidcPkceTransaction{
    pub state:String,
    pub nonce:String,
    pub code_verifier:String,
    pub code_challenge:String,
    pub created_at_unix_ms:u128,
}

/// Generate one-time recovery codes. Plaintext codes are returned once; callers must
/// persist only the PHC hashes and remove a hash after successful use.
pub fn create_recovery_codes(count:usize)->Result<RecoveryCodeSet,AuthPolicyError>{
    use rand_core::{OsRng,RngCore};
    if !(4..=20).contains(&count){return Err(AuthPolicyError::Invalid("recovery code count must be 4..20".into()));}
    let mut codes=Vec::with_capacity(count);let mut hashes=Vec::with_capacity(count);
    for _ in 0..count{let mut raw=[0u8;10];OsRng.fill_bytes(&mut raw);let compact=data_encoding::BASE32_NOPAD.encode(&raw);let code=format!("{}-{}-{}-{}",&compact[0..4],&compact[4..8],&compact[8..12],&compact[12..16]);hashes.push(hash_password(&code)?);codes.push(code);}
    Ok(RecoveryCodeSet{codes,hashes})
}
pub fn match_recovery_code(code:&str,hashes:&[String])->Result<Option<usize>,AuthPolicyError>{
    let normalized=code.trim().to_ascii_uppercase();if normalized.len()!=19||normalized.chars().any(|c|!(c.is_ascii_alphanumeric()||c=='-')){return Ok(None);}
    if hashes.len()>20{return Err(AuthPolicyError::Invalid("too many stored recovery codes".into()));}
    for(index,hash)in hashes.iter().enumerate(){if verify_password(&normalized,hash)?{return Ok(Some(index));}}
    Ok(None)
}

/// Build the browser-side OIDC authorization transaction inputs. The verifier is
/// secret until the authorization-code exchange; state and nonce must be matched and
/// the whole transaction should be expired by the caller after a short TTL.
pub fn create_oidc_pkce_transaction()->Result<OidcPkceTransaction,AuthPolicyError>{
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD,Engine as _};use rand_core::{OsRng,RngCore};use sha2::{Digest,Sha256};use std::time::{SystemTime,UNIX_EPOCH};
    let mut state_raw=[0u8;24];let mut nonce_raw=[0u8;24];let mut verifier_raw=[0u8;32];OsRng.fill_bytes(&mut state_raw);OsRng.fill_bytes(&mut nonce_raw);OsRng.fill_bytes(&mut verifier_raw);
    let state=URL_SAFE_NO_PAD.encode(state_raw);let nonce=URL_SAFE_NO_PAD.encode(nonce_raw);let code_verifier=URL_SAFE_NO_PAD.encode(verifier_raw);let challenge=Sha256::digest(code_verifier.as_bytes());let code_challenge=URL_SAFE_NO_PAD.encode(challenge);
    let created_at_unix_ms=SystemTime::now().duration_since(UNIX_EPOCH).map(|v|v.as_millis()).unwrap_or(0);
    Ok(OidcPkceTransaction{state,nonce,code_verifier,code_challenge,created_at_unix_ms})
}
pub fn validate_oidc_transaction(transaction:&OidcPkceTransaction,returned_state:&str,now_unix_ms:u128,max_age_ms:u128)->Result<(),AuthPolicyError>{
    if returned_state.len()>256||returned_state!=transaction.state{return Err(AuthPolicyError::Invalid("OIDC state mismatch".into()));}
    if max_age_ms==0||max_age_ms>30*60*1000{return Err(AuthPolicyError::Invalid("OIDC transaction TTL is invalid".into()));}
    if now_unix_ms<transaction.created_at_unix_ms||now_unix_ms.saturating_sub(transaction.created_at_unix_ms)>max_age_ms{return Err(AuthPolicyError::Invalid("OIDC transaction expired".into()));}
    Ok(())
}

#[cfg(test)]
mod enterprise_flow_tests{use super::*;#[test]fn recovery_codes_are_single_use_ready(){let set=create_recovery_codes(4).unwrap();assert_eq!(set.codes.len(),4);assert_eq!(match_recovery_code(&set.codes[0],&set.hashes).unwrap(),Some(0));assert_eq!(match_recovery_code("AAAA-BBBB-CCCC-DDDD",&set.hashes).unwrap(),None);}#[test]fn oidc_pkce_has_s256_material(){let t=create_oidc_pkce_transaction().unwrap();assert!(t.code_verifier.len()>=43);assert_eq!(t.code_challenge.len(),43);assert!(validate_oidc_transaction(&t,&t.state,t.created_at_unix_ms+1000,300000).is_ok());}}
