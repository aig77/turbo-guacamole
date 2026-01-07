#[derive(Debug, serde::Deserialize)]
pub struct ShortenPayload {
    pub url: String,
}
