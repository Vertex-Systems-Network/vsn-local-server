use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use ed25519_dalek::{Signer, SigningKey};
use serde::{Deserialize, Serialize};
use std::{env, fs};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RuntimeCatalog {
    schema_version: u32,
    provider: String,
    runtimes: Vec<RuntimeRelease>,
    #[serde(default)]
    signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RuntimeRelease {
    runtime: String,
    version: String,
    artifacts: Vec<RuntimeArtifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RuntimeArtifact {
    os: String,
    arch: String,
    url: String,
    sha256: String,
    archive: String,
    executable_relpath: String,
}

#[derive(Debug, Serialize)]
struct SignedFixture {
    public_key: String,
    signature: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::args().nth(1).ok_or("usage: pkg02_catalog_sign <catalog.json>")?;
    let mut catalog: RuntimeCatalog = serde_json::from_slice(&fs::read(path)?)?;
    catalog.signature = None;

    // Fixed test-only seed. This key is intentionally public test material and must never be
    // used by production signing or trust configuration.
    let signing_key = SigningKey::from_bytes(&[0x42; 32]);
    let bytes = serde_json::to_vec(&catalog)?;
    let signature = signing_key.sign(&bytes);
    let result = SignedFixture {
        public_key: B64.encode(signing_key.verifying_key().to_bytes()),
        signature: B64.encode(signature.to_bytes()),
    };
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}
