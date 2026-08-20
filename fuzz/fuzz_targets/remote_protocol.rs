#![no_main]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data:&[u8]|{
    let _=serde_json::from_slice::<vsn_remote::RemoteCommandV1>(data);
    let _=serde_json::from_slice::<vsn_remote::AgentStreamServerMessageV1>(data);
    let _=serde_json::from_slice::<vsn_remote::AgentStreamClientMessageV1>(data);
    let _=serde_json::from_slice::<vsn_remote::BrowserStreamClientMessageV1>(data);
    let _=serde_json::from_slice::<vsn_remote::BrowserStreamServerMessageV1>(data);
});
