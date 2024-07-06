use serde::Deserialize;

#[derive(Deserialize)]
pub struct AuthResponse {
    pub token: String,
}

#[derive(Deserialize)]
pub struct ImageManifest {
    pub layers: Vec<ManifestLayer>,
}

#[derive(Deserialize)]
pub struct ManifestLayer {
    pub digest: String,
}
