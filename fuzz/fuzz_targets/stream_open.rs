#![no_main]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data:&[u8]|{
    if let Ok(request)=serde_json::from_slice::<vsn_stream::StreamOpenRequest>(data){let _=vsn_stream::validate_open_request(&request);}
});
