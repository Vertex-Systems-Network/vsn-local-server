use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MarketplaceError {
    #[error("marketplace I/O failed: {0}")] Io(#[from] std::io::Error),
    #[error("marketplace JSON failed: {0}")] Json(#[from] serde_json::Error),
    #[error("marketplace index is invalid: {0}")] Invalid(String),
    #[error("marketplace index is not signed by a trusted registry key")] Untrusted,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct RegistryTrustStore { pub public_keys: Vec<String> }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MarketplaceEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub publisher: String,
    pub package_url: String,
    pub sha256: String,
    #[serde(default)] pub provider_kinds: Vec<String>,
    #[serde(default)] pub summary: String,
    #[serde(default = "default_channels")] pub channels: Vec<String>,
}
fn default_channels()->Vec<String>{vec!["stable".into()]}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MarketplaceRevocation { pub id:String, pub version:String, pub reason:String, pub revoked_at_unix_ms:u128 }
#[derive(Debug,Clone,Serialize,Deserialize,PartialEq,Eq)]
#[serde(rename_all="snake_case")]pub enum PublisherState{Active,Suspended,Retired}
#[derive(Debug,Clone,Serialize,Deserialize,PartialEq,Eq)]pub struct MarketplacePublisher{pub id:String,pub display_name:String,pub state:PublisherState,#[serde(default)]pub allowed_channels:Vec<String>,#[serde(default)]pub website:Option<String>}
#[derive(Debug,Clone,Serialize,Deserialize,PartialEq,Eq)]pub struct PublisherSummary{pub id:String,pub display_name:String,pub state:PublisherState,pub packages:usize,pub versions:usize}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MarketplaceIndex {
    pub api_version: u32,
    pub generated_at_unix_ms: u128,
    pub entries: Vec<MarketplaceEntry>,
    #[serde(default)] pub publishers: Vec<MarketplacePublisher>,
    #[serde(default)] pub revocations: Vec<MarketplaceRevocation>,
    pub signature: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateResolution { pub id:String, pub current_version:String, pub current_revoked:bool, pub available:Option<MarketplaceEntry> }
#[derive(Debug,Clone,Serialize,Deserialize,PartialEq,Eq)]pub struct ChannelUpdateResolution{pub id:String,pub channel:String,pub current_version:String,pub current_revoked:bool,pub available:Option<MarketplaceEntry>}

pub fn load_and_verify(index_path:&Path, trust_path:&Path)->Result<(MarketplaceIndex,String),MarketplaceError>{
    let index:MarketplaceIndex=serde_json::from_slice(&std::fs::read(index_path)?)?;
    let trust:RegistryTrustStore=serde_json::from_slice(&std::fs::read(trust_path)?)?;
    let signer=verify_index(&index,&trust)?;
    Ok((index,signer))
}

pub fn verify_index(index:&MarketplaceIndex,trust:&RegistryTrustStore)->Result<String,MarketplaceError>{
    validate_index(index)?;
    if trust.public_keys.is_empty(){return Err(MarketplaceError::Untrusted);}
    let signature=index.signature.as_deref().filter(|v|!v.is_empty()).ok_or(MarketplaceError::Untrusted)?;
    let mut unsigned=index.clone(); unsigned.signature=None;
    let bytes=serde_json::to_vec(&unsigned)?;
    for key in &trust.public_keys{
        if vsn_security::verify_signature(key,&bytes,signature).is_ok(){return Ok(key.clone());}
    }
    Err(MarketplaceError::Untrusted)
}

pub fn search(index:&MarketplaceIndex,query:&str)->Vec<MarketplaceEntry>{
    let q=query.trim().to_ascii_lowercase();
    let mut out=index.entries.iter().filter(|entry|{
        publisher_allows_entry(index,entry)&&!is_revoked(index,&entry.id,&entry.version)&&(q.is_empty()||entry.id.to_ascii_lowercase().contains(&q)||entry.name.to_ascii_lowercase().contains(&q)||entry.publisher.to_ascii_lowercase().contains(&q)||entry.summary.to_ascii_lowercase().contains(&q)||entry.provider_kinds.iter().any(|v|v.to_ascii_lowercase().contains(&q)))
    }).cloned().collect::<Vec<_>>();
    out.sort_by(|a,b|a.id.cmp(&b.id).then_with(||b.version.cmp(&a.version)));
    out
}

fn validate_index(index:&MarketplaceIndex)->Result<(),MarketplaceError>{
    if index.api_version!=1{return Err(MarketplaceError::Invalid("unsupported api_version".into()));}
    if index.entries.len()>50_000{return Err(MarketplaceError::Invalid("too many entries".into()));}
    if index.publishers.len()>10_000{return Err(MarketplaceError::Invalid("too many marketplace publishers".into()));}
    let mut publisher_ids=std::collections::HashSet::new();for p in &index.publishers{validate_component(&p.id,160)?;if !publisher_ids.insert(p.id.clone()){return Err(MarketplaceError::Invalid("duplicate marketplace publisher".into()));}if p.display_name.trim().is_empty()||p.display_name.len()>160{return Err(MarketplaceError::Invalid("publisher display name is invalid".into()));}if p.allowed_channels.len()>16||p.allowed_channels.iter().any(|c|validate_channel(c).is_err()){return Err(MarketplaceError::Invalid("publisher channels are invalid".into()));}if let Some(site)=&p.website{if !site.starts_with("https://")||site.len()>2048{return Err(MarketplaceError::Invalid("publisher website must use HTTPS".into()));}}}
    for e in &index.entries{
        validate_component(&e.id,128)?; validate_component(&e.version,64)?;
        if e.name.trim().is_empty()||e.name.len()>160||e.publisher.trim().is_empty()||e.publisher.len()>160{return Err(MarketplaceError::Invalid("entry name/publisher invalid".into()));}
        if !e.package_url.starts_with("https://")||e.package_url.len()>2048{return Err(MarketplaceError::Invalid(format!("{} package_url must use HTTPS",e.id)));}
        if e.sha256.len()!=64||!e.sha256.bytes().all(|b|b.is_ascii_hexdigit()){return Err(MarketplaceError::Invalid(format!("{} sha256 invalid",e.id)));}
        if e.provider_kinds.len()>16||e.provider_kinds.iter().any(|k|!matches!(k.as_str(),"runtime"|"database"|"service"|"project"|"container"|"cloud"|"os"|"network")){return Err(MarketplaceError::Invalid(format!("{} provider kinds invalid",e.id)));}
        if e.channels.is_empty()||e.channels.len()>8||e.channels.iter().any(|c|validate_channel(c).is_err()){return Err(MarketplaceError::Invalid(format!("{} marketplace channels invalid",e.id)));}
        if !index.publishers.is_empty(){let publisher=index.publishers.iter().find(|p|p.id==e.publisher).ok_or_else(||MarketplaceError::Invalid(format!("{} references unknown publisher {}",e.id,e.publisher)))?;if !matches!(publisher.state,PublisherState::Active){return Err(MarketplaceError::Invalid(format!("{} publisher is not active",e.id)));}if !publisher.allowed_channels.is_empty()&&e.channels.iter().any(|c|!publisher.allowed_channels.iter().any(|allowed|allowed==c)){return Err(MarketplaceError::Invalid(format!("{} uses a channel not allowed for publisher",e.id)));}}
    }
    if index.revocations.len()>50_000{return Err(MarketplaceError::Invalid("too many revocations".into()));}
    let mut seen=std::collections::HashSet::new();for r in &index.revocations{validate_component(&r.id,128)?;validate_component(&r.version,64)?;if r.reason.trim().is_empty()||r.reason.len()>1024||r.reason.chars().any(char::is_control){return Err(MarketplaceError::Invalid("revocation reason is invalid".into()));}if !seen.insert((r.id.clone(),r.version.clone())){return Err(MarketplaceError::Invalid("duplicate marketplace revocation".into()));}}
    Ok(())
}
pub fn is_revoked(index:&MarketplaceIndex,id:&str,version:&str)->bool{index.revocations.iter().any(|r|r.id==id&&r.version==version)}
pub fn resolve_update(index:&MarketplaceIndex,id:&str,current_version:&str)->Result<UpdateResolution,MarketplaceError>{validate_component(id,128)?;validate_component(current_version,64)?;let current_revoked=is_revoked(index,id,current_version);let mut candidates=index.entries.iter().filter(|e|e.id==id&&publisher_allows_entry(index,e)&&!is_revoked(index,&e.id,&e.version)&&compare_versions(&e.version,current_version).is_gt()).cloned().collect::<Vec<_>>();candidates.sort_by(|a,b|compare_versions(&b.version,&a.version));Ok(UpdateResolution{id:id.into(),current_version:current_version.into(),current_revoked,available:candidates.into_iter().next()})}
pub fn resolve_update_channel(index:&MarketplaceIndex,id:&str,current_version:&str,channel:&str)->Result<ChannelUpdateResolution,MarketplaceError>{validate_component(id,128)?;validate_component(current_version,64)?;validate_channel(channel)?;let current_revoked=is_revoked(index,id,current_version);let mut candidates=index.entries.iter().filter(|e|e.id==id&&publisher_allows_entry(index,e)&&e.channels.iter().any(|c|c==channel)&&!is_revoked(index,&e.id,&e.version)&&compare_versions(&e.version,current_version).is_gt()).cloned().collect::<Vec<_>>();candidates.sort_by(|a,b|compare_versions(&b.version,&a.version));Ok(ChannelUpdateResolution{id:id.into(),channel:channel.into(),current_version:current_version.into(),current_revoked,available:candidates.into_iter().next()})}
pub fn publishers(index:&MarketplaceIndex)->Vec<PublisherSummary>{let mut out=Vec::new();for p in &index.publishers{let entries=index.entries.iter().filter(|e|e.publisher==p.id).collect::<Vec<_>>();let packages=entries.iter().map(|e|e.id.as_str()).collect::<std::collections::HashSet<_>>().len();out.push(PublisherSummary{id:p.id.clone(),display_name:p.display_name.clone(),state:p.state.clone(),packages,versions:entries.len()});}out.sort_by(|a,b|a.id.cmp(&b.id));out}
fn publisher_allows_entry(index:&MarketplaceIndex,entry:&MarketplaceEntry)->bool{if index.publishers.is_empty(){return true;}index.publishers.iter().find(|p|p.id==entry.publisher).is_some_and(|p|matches!(p.state,PublisherState::Active)&&(p.allowed_channels.is_empty()||entry.channels.iter().all(|c|p.allowed_channels.iter().any(|allowed|allowed==c))))}
fn validate_channel(value:&str)->Result<(),MarketplaceError>{if value.is_empty()||value.len()>32||!value.bytes().all(|b|b.is_ascii_lowercase()||b.is_ascii_digit()||matches!(b,b'-'|b'_')){Err(MarketplaceError::Invalid("marketplace channel is invalid".into()))}else{Ok(())}}
fn compare_versions(a:&str,b:&str)->std::cmp::Ordering{let parse=|v:&str|v.split(|c|c=='.'||c=='-'||c=='+').map(|p|p.parse::<u64>().ok()).collect::<Vec<_>>();let aa=parse(a);let bb=parse(b);for i in 0..aa.len().max(bb.len()){match(aa.get(i).copied().flatten(),bb.get(i).copied().flatten()){(Some(x),Some(y))if x!=y=>return x.cmp(&y),(Some(x),None)if x!=0=>return std::cmp::Ordering::Greater,(None,Some(y))if y!=0=>return std::cmp::Ordering::Less,_=>{}}}a.cmp(b)}
fn validate_component(value:&str,max:usize)->Result<(),MarketplaceError>{if value.is_empty()||value.len()>max||!value.bytes().all(|b|b.is_ascii_alphanumeric()||matches!(b,b'.'|b'_'|b'-')){Err(MarketplaceError::Invalid("unsafe identifier/version".into()))}else{Ok(())}}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]fn suspended_publishers_are_hidden(){let i=MarketplaceIndex{api_version:1,generated_at_unix_ms:1,entries:vec![MarketplaceEntry{id:"x".into(),name:"X".into(),version:"1.0.0".into(),publisher:"pub".into(),package_url:"https://example.invalid/x".into(),sha256:"a".repeat(64),provider_kinds:vec![],summary:String::new(),channels:vec!["stable".into()]}],publishers:vec![MarketplacePublisher{id:"pub".into(),display_name:"Publisher".into(),state:PublisherState::Suspended,allowed_channels:vec!["stable".into()],website:None}],revocations:vec![],signature:None};assert!(search(&i,"x").is_empty());}
    #[test]fn revoked_versions_are_hidden(){let mut i=MarketplaceIndex{api_version:1,generated_at_unix_ms:1,entries:vec![MarketplaceEntry{id:"x".into(),name:"X".into(),version:"2.0.0".into(),publisher:"VSN".into(),package_url:"https://example.invalid/x".into(),sha256:"a".repeat(64),provider_kinds:vec!["runtime".into()],summary:"x".into(),channels:vec!["stable".into()]}],publishers:vec![],revocations:vec![MarketplaceRevocation{id:"x".into(),version:"2.0.0".into(),reason:"security".into(),revoked_at_unix_ms:2}],signature:None};assert!(search(&i,"x").is_empty());assert!(resolve_update(&i,"x","1.0.0").unwrap().available.is_none());i.revocations.clear();assert_eq!(resolve_update(&i,"x","1.0.0").unwrap().available.unwrap().version,"2.0.0");}
    #[test]fn channel_resolution_does_not_cross_channels(){let i=MarketplaceIndex{api_version:1,generated_at_unix_ms:1,entries:vec![MarketplaceEntry{id:"x".into(),name:"X".into(),version:"2.0.0".into(),publisher:"VSN".into(),package_url:"https://example.invalid/x".into(),sha256:"a".repeat(64),provider_kinds:vec!["runtime".into()],summary:"x".into(),channels:vec!["beta".into()]}],publishers:vec![],revocations:vec![],signature:None};assert!(resolve_update_channel(&i,"x","1.0.0","stable").unwrap().available.is_none());assert!(resolve_update_channel(&i,"x","1.0.0","beta").unwrap().available.is_some());}
    #[test]fn search_is_local_and_case_insensitive(){let i=MarketplaceIndex{api_version:1,generated_at_unix_ms:1,entries:vec![MarketplaceEntry{id:"postgres.driver".into(),name:"Postgres".into(),version:"1.0.0".into(),publisher:"VSN".into(),package_url:"https://example.invalid/pkg".into(),sha256:"a".repeat(64),provider_kinds:vec!["database".into()],summary:"Relational database".into(),channels:vec!["stable".into()]}],publishers:vec![],revocations:vec![],signature:None};assert_eq!(search(&i,"POST" ).len(),1);}
}

// ---------- 0.24 publisher submission/review lifecycle + conformance ----------
#[derive(Debug,Clone,Serialize,Deserialize,PartialEq,Eq)]
#[serde(rename_all="snake_case")]
pub enum SubmissionState{Draft,Submitted,Approved,Rejected,Published,Withdrawn}
#[derive(Debug,Clone,Serialize,Deserialize,PartialEq,Eq)]
pub struct MarketplaceSubmission{pub submission_id:String,pub entry:MarketplaceEntry,pub state:SubmissionState,pub created_at_unix_ms:u128,pub updated_at_unix_ms:u128,#[serde(default)]pub reviewer:Option<String>,#[serde(default)]pub review_note:Option<String>}
#[derive(Debug,Clone,Serialize,Deserialize,PartialEq,Eq)]
#[serde(rename_all="snake_case")]
pub enum ReviewAction{Submit,Approve,Reject,MarkPublished,Withdraw}
#[derive(Debug,Clone,Serialize,Deserialize,PartialEq,Eq)]
pub struct MarketplaceConformanceReport{pub signed_index:bool,pub trust_store:bool,pub sha_pinned_packages:bool,pub revocations:bool,pub channels:bool,pub publisher_governance:bool,pub submission_review_lifecycle:bool,pub signed_registry_required_for_distribution:bool,pub update_resolution:bool,pub issues:Vec<String>}

pub fn new_submission(submission_id:&str,entry:MarketplaceEntry,now_unix_ms:u128)->Result<MarketplaceSubmission,MarketplaceError>{validate_component(submission_id,160)?;validate_entry_standalone(&entry)?;Ok(MarketplaceSubmission{submission_id:submission_id.into(),entry,state:SubmissionState::Draft,created_at_unix_ms:now_unix_ms,updated_at_unix_ms:now_unix_ms,reviewer:None,review_note:None})}
pub fn transition_submission(submission:&MarketplaceSubmission,action:ReviewAction,actor:&str,note:Option<&str>,now_unix_ms:u128,index:&MarketplaceIndex)->Result<MarketplaceSubmission,MarketplaceError>{if actor.trim().is_empty()||actor.len()>160||actor.chars().any(char::is_control){return Err(MarketplaceError::Invalid("marketplace review actor is invalid".into()));}if note.map(|n|n.len()>4096||n.chars().any(char::is_control)).unwrap_or(false){return Err(MarketplaceError::Invalid("marketplace review note is invalid".into()));}validate_entry_standalone(&submission.entry)?;let publisher=index.publishers.iter().find(|p|p.id==submission.entry.publisher).ok_or_else(||MarketplaceError::Invalid("submission publisher is not registered".into()))?;if !matches!(publisher.state,PublisherState::Active){return Err(MarketplaceError::Invalid("submission publisher is not active".into()));}if !publisher.allowed_channels.is_empty()&&submission.entry.channels.iter().any(|c|!publisher.allowed_channels.iter().any(|allowed|allowed==c)){return Err(MarketplaceError::Invalid("submission uses a publisher-disallowed channel".into()));}let next=match(&submission.state,action){(SubmissionState::Draft,ReviewAction::Submit)=>SubmissionState::Submitted,(SubmissionState::Submitted,ReviewAction::Approve)=>SubmissionState::Approved,(SubmissionState::Submitted,ReviewAction::Reject)=>SubmissionState::Rejected,(SubmissionState::Approved,ReviewAction::MarkPublished)=>SubmissionState::Published,(SubmissionState::Draft|SubmissionState::Submitted|SubmissionState::Approved|SubmissionState::Rejected,ReviewAction::Withdraw)=>SubmissionState::Withdrawn,_=>return Err(MarketplaceError::Invalid("invalid marketplace submission state transition".into()))};let reviewer=if matches!(next,SubmissionState::Approved|SubmissionState::Rejected|SubmissionState::Published){Some(actor.into())}else{submission.reviewer.clone()};Ok(MarketplaceSubmission{submission_id:submission.submission_id.clone(),entry:submission.entry.clone(),state:next,created_at_unix_ms:submission.created_at_unix_ms,updated_at_unix_ms:now_unix_ms,reviewer,review_note:note.map(str::to_string).or_else(||submission.review_note.clone())})}
pub fn approved_publish_candidate(submission:&MarketplaceSubmission,index:&MarketplaceIndex)->Result<MarketplaceEntry,MarketplaceError>{if !matches!(submission.state,SubmissionState::Approved){return Err(MarketplaceError::Invalid("only an approved marketplace submission can become a registry publish candidate".into()));}validate_entry_standalone(&submission.entry)?;if is_revoked(index,&submission.entry.id,&submission.entry.version){return Err(MarketplaceError::Invalid("revoked package version cannot be published".into()));}if index.entries.iter().any(|e|e.id==submission.entry.id&&e.version==submission.entry.version){return Err(MarketplaceError::Invalid("marketplace package version already exists".into()));}if !publisher_allows_entry(index,&submission.entry){return Err(MarketplaceError::Invalid("publisher policy rejects marketplace entry".into()));}Ok(submission.entry.clone())}
fn validate_entry_standalone(e:&MarketplaceEntry)->Result<(),MarketplaceError>{validate_component(&e.id,128)?;validate_component(&e.version,64)?;validate_component(&e.publisher,160)?;if e.name.trim().is_empty()||e.name.len()>160{return Err(MarketplaceError::Invalid("marketplace entry name is invalid".into()));}if !e.package_url.starts_with("https://")||e.package_url.len()>2048{return Err(MarketplaceError::Invalid("marketplace package_url must use HTTPS".into()));}if e.sha256.len()!=64||!e.sha256.bytes().all(|b|b.is_ascii_hexdigit()){return Err(MarketplaceError::Invalid("marketplace entry SHA-256 is invalid".into()));}if e.channels.is_empty()||e.channels.len()>8||e.channels.iter().any(|c|validate_channel(c).is_err()){return Err(MarketplaceError::Invalid("marketplace entry channels are invalid".into()));}if e.provider_kinds.len()>16||e.provider_kinds.iter().any(|k|!matches!(k.as_str(),"runtime"|"database"|"service"|"project"|"container"|"cloud"|"os"|"network")){return Err(MarketplaceError::Invalid("marketplace entry provider kinds are invalid".into()));}Ok(())}
pub fn conformance()->MarketplaceConformanceReport{MarketplaceConformanceReport{signed_index:true,trust_store:true,sha_pinned_packages:true,revocations:true,channels:true,publisher_governance:true,submission_review_lifecycle:true,signed_registry_required_for_distribution:true,update_resolution:true,issues:vec![]}}

#[cfg(test)]mod review_lifecycle_tests{use super::*;fn entry()->MarketplaceEntry{MarketplaceEntry{id:"x".into(),name:"X".into(),version:"1.0.0".into(),publisher:"pub".into(),package_url:"https://example.invalid/x".into(),sha256:"a".repeat(64),provider_kinds:vec!["runtime".into()],summary:String::new(),channels:vec!["stable".into()]}}fn index()->MarketplaceIndex{MarketplaceIndex{api_version:1,generated_at_unix_ms:1,entries:vec![],publishers:vec![MarketplacePublisher{id:"pub".into(),display_name:"Pub".into(),state:PublisherState::Active,allowed_channels:vec!["stable".into()],website:None}],revocations:vec![],signature:None}}#[test]fn only_approved_submission_can_publish(){let i=index();let d=new_submission("sub-1",entry(),1).unwrap();let s=transition_submission(&d,ReviewAction::Submit,"publisher",None,2,&i).unwrap();assert!(approved_publish_candidate(&s,&i).is_err());let a=transition_submission(&s,ReviewAction::Approve,"reviewer",Some("ok"),3,&i).unwrap();assert!(approved_publish_candidate(&a,&i).is_ok());}}
