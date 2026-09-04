use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct LikeState {
    #[serde(default)]
    pub state: String,
}

#[derive(Debug, Clone, Copy)]
pub enum HotkeyAction {
    Like,
    Dislike,
}

pub async fn get_like_state(client: &reqwest::Client, base_url: &str) -> Result<LikeState> {
    Ok(client
        .get(format!("{base_url}/api/v1/like-state"))
        .send().await?
        .error_for_status()?
        .json::<LikeState>().await?)
}

pub async fn post_like(client: &reqwest::Client, base_url: &str) -> Result<()> {
    client.post(format!("{base_url}/api/v1/like")).send().await?.error_for_status()?;
    Ok(())
}

pub async fn post_dislike(client: &reqwest::Client, base_url: &str) -> Result<()> {
    client.post(format!("{base_url}/api/v1/dislike")).send().await?.error_for_status()?;
    Ok(())
}
