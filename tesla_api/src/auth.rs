use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{auth_url, TeslaError};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub id_token: String,
    pub expires_in: i32,
    pub token_type: String,
}

// TODO: https://stateful.com/blog/oauth-refresh-token-best-practices
pub async fn refresh_access_token(refresh_token: &str) -> Result<AuthResponse, TeslaError> {
    let mut map = HashMap::new();
    map.insert("grant_type", "refresh_token");
    map.insert("client_id", "ownerapi");
    map.insert("refresh_token", refresh_token);
    map.insert("scope", "openid email offline_access");

    let client = reqwest::ClientBuilder::new()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;
    let res = client.post(auth_url()).json(&map).send().await?;

    if res.status().is_success() {
        let token = res.json::<AuthResponse>().await?;
        Ok(token)
    } else {
        Err(TeslaError::Request(res.status()))
    }
}
