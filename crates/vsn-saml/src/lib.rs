use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use flate2::{write::DeflateEncoder, Compression};
use rand_core::{OsRng, RngCore};
use roxmltree::{Document, Node};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use thiserror::Error;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use url::Url;

const MAX_SAML_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
const MAX_XMLSEC_OUTPUT: usize = 64 * 1024;
const CLOCK_SKEW_SECONDS: i64 = 120;

#[derive(Debug, Error)]
pub enum SamlError {
    #[error("SAML request rejected: {0}")]
    Invalid(String),
    #[error("SAML XML signature verification failed: {0}")]
    Signature(String),
    #[error("SAML verifier unavailable: {0}")]
    VerifierUnavailable(String),
    #[error("SAML I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("SAML JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SamlLoginTransaction {
    pub request_id: String,
    pub relay_state: String,
    pub provider_id: String,
    pub created_at_unix_ms: u128,
    pub expires_at_unix_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SamlLoginStart {
    pub transaction: SamlLoginTransaction,
    pub redirect_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SamlLogoutStart {
    pub request_id: String,
    pub relay_state: String,
    pub redirect_url: String,
}

pub fn create_logout_start(
    provider: &vsn_auth::SamlProviderConfig,
    subject: &str,
    session_index: Option<&str>,
) -> Result<SamlLogoutStart, SamlError> {
    vsn_auth::validate_saml(provider).map_err(|e| SamlError::Invalid(e.to_string()))?;
    let slo = provider
        .slo_url
        .as_ref()
        .ok_or_else(|| SamlError::Invalid("SAML provider has no slo_url".into()))?;
    if subject.is_empty() || subject.len() > 1024 || subject.chars().any(char::is_control) {
        return Err(SamlError::Invalid("SAML logout subject is invalid".into()));
    }
    if session_index
        .is_some_and(|v| v.is_empty() || v.len() > 1024 || v.chars().any(char::is_control))
    {
        return Err(SamlError::Invalid("SAML SessionIndex is invalid".into()));
    }
    let request_id = format!("_vsn_logout_{}", random_hex(24));
    let relay_state = random_hex(32);
    let issue = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|e| SamlError::Invalid(e.to_string()))?;
    let session = session_index
        .map(|v| format!("<samlp:SessionIndex>{}</samlp:SessionIndex>", xml_escape(v)))
        .unwrap_or_default();
    let xml = format!(
        r#"<samlp:LogoutRequest xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol" xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion" ID="{request_id}" Version="2.0" IssueInstant="{issue}" Destination="{slo}"><saml:Issuer>{entity}</saml:Issuer><saml:NameID>{subject}</saml:NameID>{session}</samlp:LogoutRequest>"#,
        slo = xml_escape(slo),
        entity = xml_escape(&provider.entity_id),
        subject = xml_escape(subject)
    );
    let mut enc = DeflateEncoder::new(Vec::new(), Compression::default());
    enc.write_all(xml.as_bytes())?;
    let request_b64 = B64.encode(enc.finish()?);
    let mut url =
        Url::parse(slo).map_err(|e| SamlError::Invalid(format!("invalid SAML SLO URL: {e}")))?;
    {
        let mut q = url.query_pairs_mut();
        q.append_pair("SAMLRequest", &request_b64);
        q.append_pair("RelayState", &relay_state);
    }
    Ok(SamlLogoutStart {
        request_id,
        relay_state,
        redirect_url: url.to_string(),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerifiedSamlAssertion {
    pub provider_id: String,
    pub issuer: String,
    pub subject: String,
    pub email: Option<String>,
    pub audience: String,
    pub in_response_to: String,
    pub session_index: Option<String>,
    pub authn_context: Option<String>,
    pub attributes: BTreeMap<String, Vec<String>>,
    pub verified_at_unix_ms: u128,
}

pub fn create_login_start(
    provider: &vsn_auth::SamlProviderConfig,
    ttl_ms: u128,
) -> Result<SamlLoginStart, SamlError> {
    vsn_auth::validate_saml(provider).map_err(|e| SamlError::Invalid(e.to_string()))?;
    if !(60_000..=10 * 60_000).contains(&ttl_ms) {
        return Err(SamlError::Invalid(
            "SAML transaction TTL must be 1..10 minutes".into(),
        ));
    }
    let now = now_ms();
    let request_id = format!("_vsn_{}", random_hex(24));
    let relay_state = random_hex(32);
    let issue = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|e| SamlError::Invalid(e.to_string()))?;
    let xml = format!(
        r#"<samlp:AuthnRequest xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol" xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion" ID="{request_id}" Version="2.0" IssueInstant="{issue}" Destination="{sso}" AssertionConsumerServiceURL="{acs}"><saml:Issuer>{entity}</saml:Issuer><samlp:NameIDPolicy Format="{name_id}" AllowCreate="true"/></samlp:AuthnRequest>"#,
        sso = xml_escape(&provider.sso_url),
        acs = xml_escape(&provider.acs_url),
        entity = xml_escape(&provider.entity_id),
        name_id = xml_escape(&provider.name_id_format)
    );
    let mut enc = DeflateEncoder::new(Vec::new(), Compression::default());
    enc.write_all(xml.as_bytes())?;
    let compressed = enc.finish()?;
    let request_b64 = B64.encode(compressed);
    let mut url = Url::parse(&provider.sso_url)
        .map_err(|e| SamlError::Invalid(format!("invalid SAML SSO URL: {e}")))?;
    {
        let mut q = url.query_pairs_mut();
        q.append_pair("SAMLRequest", &request_b64);
        q.append_pair("RelayState", &relay_state);
    }
    Ok(SamlLoginStart {
        transaction: SamlLoginTransaction {
            request_id,
            relay_state,
            provider_id: provider.id.clone(),
            created_at_unix_ms: now,
            expires_at_unix_ms: now + ttl_ms,
        },
        redirect_url: url.to_string(),
    })
}

pub fn verify_acs_response(
    provider: &vsn_auth::SamlProviderConfig,
    transaction: &SamlLoginTransaction,
    saml_response_b64: &str,
    relay_state: &str,
) -> Result<VerifiedSamlAssertion, SamlError> {
    vsn_auth::validate_saml(provider).map_err(|e| SamlError::Invalid(e.to_string()))?;
    let now = now_ms();
    if transaction.provider_id != provider.id {
        return Err(SamlError::Invalid(
            "SAML provider/transaction mismatch".into(),
        ));
    }
    if transaction.expires_at_unix_ms < now || transaction.created_at_unix_ms > now {
        return Err(SamlError::Invalid("SAML transaction expired".into()));
    }
    if !constant_time_eq(relay_state.as_bytes(), transaction.relay_state.as_bytes()) {
        return Err(SamlError::Invalid("SAML RelayState mismatch".into()));
    }
    if saml_response_b64.len() > MAX_SAML_RESPONSE_BYTES * 2 {
        return Err(SamlError::Invalid("SAMLResponse exceeds size limit".into()));
    }
    let xml = B64
        .decode(saml_response_b64)
        .map_err(|_| SamlError::Invalid("SAMLResponse is not valid base64".into()))?;
    if xml.is_empty() || xml.len() > MAX_SAML_RESPONSE_BYTES {
        return Err(SamlError::Invalid("SAMLResponse size is invalid".into()));
    }
    let text = std::str::from_utf8(&xml)
        .map_err(|_| SamlError::Invalid("SAMLResponse XML is not UTF-8".into()))?;
    let upper = text.to_ascii_uppercase();
    if upper.contains("<!DOCTYPE") || upper.contains("<!ENTITY") {
        return Err(SamlError::Invalid(
            "DTD/entity declarations are forbidden in SAML responses".into(),
        ));
    }
    validate_signature_shape(text)?;
    verify_xml_signature(provider, &xml)?;
    parse_verified_response(provider, transaction, text, now)
}

fn validate_signature_shape(xml: &str) -> Result<(), SamlError> {
    let doc = Document::parse(xml)
        .map_err(|e| SamlError::Invalid(format!("SAML XML parse failed: {e}")))?;
    let signatures = doc
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "Signature")
        .collect::<Vec<_>>();
    if signatures.is_empty() || signatures.len() > 2 {
        return Err(SamlError::Invalid(
            "SAML response must contain one or two XML Signature elements".into(),
        ));
    }
    for signature in signatures {
        let parent = signature
            .parent()
            .filter(|n| n.is_element())
            .ok_or_else(|| SamlError::Invalid("SAML Signature parent missing".into()))?;
        if !matches!(parent.tag_name().name(), "Response" | "Assertion") {
            return Err(SamlError::Invalid(
                "SAML Signature must be a direct child of Response or Assertion".into(),
            ));
        }
    }
    Ok(())
}

fn verify_xml_signature(
    provider: &vsn_auth::SamlProviderConfig,
    xml: &[u8],
) -> Result<(), SamlError> {
    let cert_pem = std::env::var(&provider.x509_certificate_pem_env).map_err(|_| {
        SamlError::VerifierUnavailable(format!(
            "{} is not configured",
            provider.x509_certificate_pem_env
        ))
    })?;
    if cert_pem.len() < 64
        || cert_pem.len() > 1024 * 1024
        || !cert_pem.contains("BEGIN CERTIFICATE")
    {
        return Err(SamlError::Invalid(
            "configured SAML certificate PEM is invalid".into(),
        ));
    }
    let xmlsec = vsn_system::find_executable("xmlsec1")
        .map_err(|e| SamlError::VerifierUnavailable(e.to_string()))?;
    let nonce = random_hex(16);
    let dir = std::env::temp_dir().join(format!("vsn-saml-{nonce}"));
    fs::create_dir(&dir)?;
    harden_temp_dir(&dir)?;
    let xml_path = dir.join("response.xml");
    let cert_path = dir.join("idp.pem");
    let result = (|| {
        fs::write(&xml_path, xml)?;
        fs::write(&cert_path, cert_pem.as_bytes())?;
        let mut child = Command::new(xmlsec)
            .args(["--verify", "--pubkey-cert-pem"])
            .arg(&cert_path)
            .args([
                "--id-attr:ID",
                "urn:oasis:names:tc:SAML:2.0:protocol:Response",
                "--id-attr:ID",
                "urn:oasis:names:tc:SAML:2.0:assertion:Assertion",
            ])
            .arg(&xml_path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| SamlError::VerifierUnavailable(e.to_string()))?;
        let stderr = child.stderr.take().ok_or_else(|| {
            SamlError::VerifierUnavailable("xmlsec1 stderr pipe unavailable".into())
        })?;
        let reader = thread::spawn(move || {
            let mut limited = stderr.take(MAX_XMLSEC_OUTPUT as u64 + 1);
            let mut bytes = Vec::new();
            let _ = limited.read_to_end(&mut bytes);
            bytes
        });
        let started = Instant::now();
        let status = loop {
            if let Some(status) = child.try_wait()? {
                break status;
            }
            if started.elapsed() > Duration::from_secs(15) {
                let _ = child.kill();
                let _ = child.wait();
                let _ = reader.join();
                return Err(SamlError::VerifierUnavailable(
                    "xmlsec1 verification timed out".into(),
                ));
            }
            thread::sleep(Duration::from_millis(20));
        };
        let mut stderr = reader.join().unwrap_or_default();
        if stderr.len() > MAX_XMLSEC_OUTPUT {
            stderr.truncate(MAX_XMLSEC_OUTPUT);
        }
        if !status.success() {
            return Err(SamlError::Signature(
                String::from_utf8_lossy(&stderr).into_owned(),
            ));
        }
        Ok(())
    })();
    let _ = fs::remove_file(&xml_path);
    let _ = fs::remove_file(&cert_path);
    let _ = fs::remove_dir(&dir);
    result
}
#[cfg(unix)]
fn harden_temp_dir(path: &std::path::Path) -> Result<(), SamlError> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}
#[cfg(not(unix))]
fn harden_temp_dir(_path: &std::path::Path) -> Result<(), SamlError> {
    Ok(())
}

fn parse_verified_response(
    provider: &vsn_auth::SamlProviderConfig,
    transaction: &SamlLoginTransaction,
    xml: &str,
    now_ms: u128,
) -> Result<VerifiedSamlAssertion, SamlError> {
    let doc = Document::parse(xml)
        .map_err(|e| SamlError::Invalid(format!("SAML XML parse failed: {e}")))?;
    let responses = doc
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "Response")
        .collect::<Vec<_>>();
    let assertions = doc
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "Assertion")
        .collect::<Vec<_>>();
    if responses.len() != 1 || assertions.len() != 1 {
        return Err(SamlError::Invalid(
            "SAML response must contain exactly one Response and one Assertion".into(),
        ));
    }
    let response = responses[0];
    let assertion = assertions[0];
    let response_id = response
        .attribute("ID")
        .ok_or_else(|| SamlError::Invalid("SAML Response ID missing".into()))?;
    let assertion_id = assertion
        .attribute("ID")
        .ok_or_else(|| SamlError::Invalid("SAML Assertion ID missing".into()))?;
    if response_id == assertion_id {
        return Err(SamlError::Invalid(
            "SAML Response and Assertion IDs must differ".into(),
        ));
    }
    let ids = doc
        .descendants()
        .filter(|n| n.is_element())
        .filter_map(|n| n.attribute("ID"))
        .collect::<Vec<_>>();
    let unique = ids
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    if ids.len() != unique.len() {
        return Err(SamlError::Invalid(
            "duplicate XML ID attributes are forbidden in SAML responses".into(),
        ));
    }
    let destination = response
        .attribute("Destination")
        .ok_or_else(|| SamlError::Invalid("SAML Response Destination missing".into()))?;
    if destination != provider.acs_url {
        return Err(SamlError::Invalid(
            "SAML Response Destination mismatch".into(),
        ));
    }
    let in_response_to = response
        .attribute("InResponseTo")
        .ok_or_else(|| SamlError::Invalid("SAML InResponseTo missing".into()))?;
    if in_response_to != transaction.request_id {
        return Err(SamlError::Invalid("SAML InResponseTo mismatch".into()));
    }
    let status_success = doc
        .descendants()
        .find(|n| n.is_element() && n.tag_name().name() == "StatusCode")
        .and_then(|n| n.attribute("Value"))
        .is_some_and(|v| v.ends_with(":Success"));
    if !status_success {
        return Err(SamlError::Invalid(
            "SAML response status is not Success".into(),
        ));
    }
    let issuer = assertion
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "Issuer")
        .and_then(|n| n.text())
        .unwrap_or("")
        .trim()
        .to_string();
    if issuer != provider.idp_entity_id {
        return Err(SamlError::Invalid(
            "SAML assertion Issuer does not match configured IdP entity ID".into(),
        ));
    }
    if let Some(response_issuer) = response
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "Issuer")
        .and_then(|n| n.text())
        .map(str::trim)
    {
        if response_issuer != provider.idp_entity_id {
            return Err(SamlError::Invalid(
                "SAML Response Issuer does not match configured IdP entity ID".into(),
            ));
        }
    }
    let audience = doc
        .descendants()
        .find(|n| n.is_element() && n.tag_name().name() == "Audience")
        .and_then(|n| n.text())
        .unwrap_or("")
        .trim()
        .to_string();
    if audience != provider.audience {
        return Err(SamlError::Invalid("SAML audience mismatch".into()));
    }
    validate_conditions(assertion, now_ms)?;
    let subject = doc
        .descendants()
        .find(|n| n.is_element() && n.tag_name().name() == "NameID")
        .and_then(|n| n.text())
        .unwrap_or("")
        .trim()
        .to_string();
    if subject.is_empty() || subject.len() > 1024 {
        return Err(SamlError::Invalid("SAML NameID is invalid".into()));
    }
    if let Some(recipient) = doc
        .descendants()
        .find(|n| n.is_element() && n.tag_name().name() == "SubjectConfirmationData")
        .and_then(|n| n.attribute("Recipient"))
    {
        if recipient != provider.acs_url {
            return Err(SamlError::Invalid(
                "SAML SubjectConfirmationData Recipient mismatch".into(),
            ));
        }
    }
    if let Some(ir) = doc
        .descendants()
        .find(|n| n.is_element() && n.tag_name().name() == "SubjectConfirmationData")
        .and_then(|n| n.attribute("InResponseTo"))
    {
        if ir != transaction.request_id {
            return Err(SamlError::Invalid(
                "SAML SubjectConfirmationData InResponseTo mismatch".into(),
            ));
        }
    }
    let mut attributes: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for attr in doc
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "Attribute")
    {
        if let Some(name) = attr.attribute("Name") {
            if name.len() > 256 {
                continue;
            }
            let values = attr
                .children()
                .filter(|n| n.is_element() && n.tag_name().name() == "AttributeValue")
                .filter_map(|n| n.text())
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty() && v.len() <= 4096)
                .take(32)
                .collect::<Vec<_>>();
            if !values.is_empty() {
                attributes.insert(name.to_string(), values);
            }
        }
    }
    let email = attributes
        .get(&provider.email_attribute)
        .and_then(|v| v.first())
        .cloned()
        .or_else(|| {
            if provider.name_id_format.contains("emailAddress") {
                Some(subject.clone())
            } else {
                None
            }
        });
    let authn = doc
        .descendants()
        .find(|n| n.is_element() && n.tag_name().name() == "AuthnStatement");
    let session_index = authn
        .and_then(|n| n.attribute("SessionIndex"))
        .map(str::to_string);
    let authn_context = doc
        .descendants()
        .find(|n| n.is_element() && n.tag_name().name() == "AuthnContextClassRef")
        .and_then(|n| n.text())
        .map(|v| v.trim().to_string());
    Ok(VerifiedSamlAssertion {
        provider_id: provider.id.clone(),
        issuer,
        subject,
        email,
        audience,
        in_response_to: in_response_to.into(),
        session_index,
        authn_context,
        attributes,
        verified_at_unix_ms: now_ms,
    })
}

fn validate_conditions(assertion: Node<'_, '_>, now_ms: u128) -> Result<(), SamlError> {
    let now = OffsetDateTime::from_unix_timestamp((now_ms / 1000) as i64)
        .map_err(|e| SamlError::Invalid(e.to_string()))?;
    if let Some(conditions) = assertion
        .descendants()
        .find(|n| n.is_element() && n.tag_name().name() == "Conditions")
    {
        if let Some(v) = conditions.attribute("NotBefore") {
            let t = parse_time(v)?;
            if now + time::Duration::seconds(CLOCK_SKEW_SECONDS) < t {
                return Err(SamlError::Invalid("SAML assertion is not yet valid".into()));
            }
        }
        if let Some(v) = conditions.attribute("NotOnOrAfter") {
            let t = parse_time(v)?;
            if now - time::Duration::seconds(CLOCK_SKEW_SECONDS) >= t {
                return Err(SamlError::Invalid("SAML assertion expired".into()));
            }
        }
    }
    if let Some(data) = assertion
        .descendants()
        .find(|n| n.is_element() && n.tag_name().name() == "SubjectConfirmationData")
    {
        if let Some(v) = data.attribute("NotOnOrAfter") {
            let t = parse_time(v)?;
            if now - time::Duration::seconds(CLOCK_SKEW_SECONDS) >= t {
                return Err(SamlError::Invalid(
                    "SAML subject confirmation expired".into(),
                ));
            }
        }
    }
    Ok(())
}
fn parse_time(value: &str) -> Result<OffsetDateTime, SamlError> {
    OffsetDateTime::parse(value, &Rfc3339)
        .map_err(|_| SamlError::Invalid("invalid SAML timestamp".into()))
}
fn random_hex(bytes: usize) -> String {
    let mut raw = vec![0u8; bytes];
    OsRng.fill_bytes(&mut raw);
    raw.iter().map(|b| format!("{b:02x}")).collect()
}
fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (i, j) in a.iter().zip(b) {
        diff |= i ^ j;
    }
    diff == 0
}
fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn constant_compare() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
    }
    #[test]
    fn xml_escaping() {
        assert_eq!(xml_escape("a&<\""), "a&amp;&lt;&quot;");
    }
}
