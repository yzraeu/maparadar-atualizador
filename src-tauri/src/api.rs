use serde::Deserialize;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::error::AppError;

const X_APP: &str = "Atualizador";

#[derive(Debug, Clone)]
pub struct ApiClient {
    client: reqwest::Client,
    base_url: String,
}

#[derive(Debug, Clone)]
pub struct AuthResponse {
    pub access_token: String,
    pub expires_in: i64,
    #[allow(dead_code)]
    pub username: String,
    #[allow(dead_code)]
    pub group_id: i64,
    pub refresh_token: String,
}

#[derive(Debug, Deserialize)]
struct LoginBody {
    access_token: String,
    expires_in: i64,
    user: LoginUser,
}

#[derive(Debug, Deserialize)]
struct LoginUser {
    username: String,
    #[serde(rename = "groupId")]
    group_id: i64,
}

#[derive(Debug, Deserialize)]
struct PreviewBody {
    count: u32,
}

impl ApiClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .connect_timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("failed to build http client"),
            base_url: base_url.into().trim_end_matches('/').to_string(),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}/{}", self.base_url, path.trim_start_matches('/'))
    }

    pub async fn login(&self, username: &str, password: &str) -> Result<AuthResponse, AppError> {
        let resp = self
            .client
            .post(self.url("auth/login"))
            .header("X-Application", X_APP)
            .json(&serde_json::json!({ "username": username, "password": password }))
            .send()
            .await?;

        if resp.status().as_u16() == 401 {
            return Err(AppError::InvalidLogin);
        }
        if resp.status().as_u16() == 429 {
            return Err(AppError::Api("Muitas tentativas de login. Aguarde e tente novamente.".into()));
        }
        if !resp.status().is_success() {
            return Err(AppError::Api(format!("login retornou {}", resp.status())));
        }

        let refresh_token = extract_cookie(&resp, "maparadar_refresh").unwrap_or_default();
        let body: LoginBody = resp.json().await?;

        Ok(AuthResponse {
            access_token: body.access_token,
            expires_in: body.expires_in,
            username: body.user.username,
            group_id: body.user.group_id,
            refresh_token,
        })
    }

    pub async fn refresh(&self, refresh_token: &str) -> Result<AuthResponse, AppError> {
        let resp = self
            .client
            .post(self.url("auth/refresh"))
            .header("X-Application", X_APP)
            .header("Cookie", format!("maparadar_refresh={refresh_token}"))
            .json(&serde_json::json!({}))
            .send()
            .await?;

        if resp.status().as_u16() == 401 {
            return Err(AppError::Unauthorized);
        }
        if !resp.status().is_success() {
            return Err(AppError::Api(format!("refresh retornou {}", resp.status())));
        }

        let new_refresh = extract_cookie(&resp, "maparadar_refresh")
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| refresh_token.to_string());
        let body: LoginBody = resp.json().await?;

        Ok(AuthResponse {
            access_token: body.access_token,
            expires_in: body.expires_in,
            username: body.user.username,
            group_id: body.user.group_id,
            refresh_token: new_refresh,
        })
    }

    pub async fn export_updater(
        &self,
        access_token: &str,
        export_type: &str,
        radar_types: &str,
    ) -> Result<Vec<u8>, AppError> {
        let resp = self
            .client
            .post(self.url("export/updater"))
            .header("X-Application", X_APP)
            .bearer_auth(access_token)
            .json(&serde_json::json!({ "exportType": export_type, "radarTypes": radar_types }))
            .send()
            .await?;

        if resp.status().as_u16() == 401 {
            return Err(AppError::Unauthorized);
        }
        if resp.status().as_u16() == 204 {
            return Err(AppError::EmptyExport);
        }
        if !resp.status().is_success() {
            return Err(AppError::Api(format!("export retornou {}", resp.status())));
        }
        Ok(resp.bytes().await?.to_vec())
    }

    pub async fn preview_count(&self, radar_types: &str) -> Result<u32, AppError> {
        let resp = self
            .client
            .get(self.url("export/preview"))
            .header("X-Application", X_APP)
            .query(&[("radarTypes", radar_types)])
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(AppError::Api(format!("preview retornou {}", resp.status())));
        }
        let body: PreviewBody = resp.json().await?;
        Ok(body.count)
    }
}

fn extract_cookie(resp: &reqwest::Response, name: &str) -> Option<String> {
    resp.headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find_map(|h| {
            let (key, value) = h.split_once('=')?;
            if key.trim() == name {
                Some(value.split(';').next()?.trim().to_string())
            } else {
                None
            }
        })
}

pub fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;

    #[tokio::test]
    async fn login_success_parses_token_and_refresh_cookie() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST)
                .path("/auth/login")
                .header("X-Application", "Atualizador");
            then.status(200)
                .header("Content-Type", "application/json")
                .header("Set-Cookie", "maparadar_refresh=abc123; Path=/; HttpOnly")
                .json_body(serde_json::json!({
                    "access_token": "jwt-token",
                    "expires_in": 86400,
                    "user": { "id": 1, "username": "izzy", "groupId": 2 }
                }));
        });

        let api = ApiClient::new(server.base_url());
        let auth = api.login("izzy", "pw").await.unwrap();
        assert_eq!(auth.access_token, "jwt-token");
        assert_eq!(auth.expires_in, 86400);
        assert_eq!(auth.username, "izzy");
        assert_eq!(auth.group_id, 2);
        assert_eq!(auth.refresh_token, "abc123");
    }

    #[tokio::test]
    async fn login_invalid_credentials_returns_invalid_login() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/auth/login");
            then.status(401);
        });
        let api = ApiClient::new(server.base_url());
        let err = api.login("izzy", "wrong").await.unwrap_err();
        assert!(matches!(err, AppError::InvalidLogin));
    }

    #[tokio::test]
    async fn refresh_sends_cookie_and_rotates() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST)
                .path("/auth/refresh")
                .header("Cookie", "maparadar_refresh=old-token");
            then.status(200)
                .header("Content-Type", "application/json")
                .header("Set-Cookie", "maparadar_refresh=new-token; Path=/; HttpOnly")
                .json_body(serde_json::json!({
                    "access_token": "new-jwt",
                    "expires_in": 86400,
                    "user": { "id": 1, "username": "izzy", "groupId": 2 }
                }));
        });
        let api = ApiClient::new(server.base_url());
        let auth = api.refresh("old-token").await.unwrap();
        assert_eq!(auth.access_token, "new-jwt");
        assert_eq!(auth.refresh_token, "new-token");
    }

    #[tokio::test]
    async fn export_updater_returns_file_bytes() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST)
                .path("/export/updater")
                .header("Authorization", "Bearer jwt-token")
                .json_body(serde_json::json!({ "exportType": "igo8", "radarTypes": "1,2,4" }));
            then.status(200).body("X,Y,TYPE,SPEED");
        });
        let api = ApiClient::new(server.base_url());
        let bytes = api.export_updater("jwt-token", "igo8", "1,2,4").await.unwrap();
        assert_eq!(String::from_utf8(bytes).unwrap(), "X,Y,TYPE,SPEED");
    }

    #[tokio::test]
    async fn export_updater_204_maps_to_empty_export() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/export/updater");
            then.status(204);
        });
        let api = ApiClient::new(server.base_url());
        let err = api.export_updater("jwt", "igo8", "1").await.unwrap_err();
        assert!(matches!(err, AppError::EmptyExport));
    }

    #[tokio::test]
    async fn export_updater_401_maps_to_unauthorized() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/export/updater");
            then.status(401);
        });
        let api = ApiClient::new(server.base_url());
        let err = api.export_updater("expired", "igo8", "1").await.unwrap_err();
        assert!(matches!(err, AppError::Unauthorized));
    }

    #[tokio::test]
    async fn preview_count_parses_count() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET)
                .path("/export/preview")
                .query_param("radarTypes", "1,2");
            then.status(200).json_body(serde_json::json!({ "count": 42 }));
        });
        let api = ApiClient::new(server.base_url());
        assert_eq!(api.preview_count("1,2").await.unwrap(), 42);
    }

    #[tokio::test]
    async fn refresh_401_maps_to_unauthorized() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/auth/refresh");
            then.status(401);
        });
        let api = ApiClient::new(server.base_url());
        let err = api.refresh("stale").await.unwrap_err();
        assert!(matches!(err, AppError::Unauthorized));
    }

    #[tokio::test]
    async fn login_429_maps_to_rate_limit_message() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/auth/login");
            then.status(429);
        });
        let api = ApiClient::new(server.base_url());
        let err = api.login("izzy", "pw").await.unwrap_err();
        assert!(matches!(err, AppError::Api(_)));
    }

    #[tokio::test]
    async fn login_server_error_maps_to_api_error() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/auth/login");
            then.status(500);
        });
        let api = ApiClient::new(server.base_url());
        let err = api.login("izzy", "pw").await.unwrap_err();
        assert!(matches!(err, AppError::Api(_)));
    }

    #[tokio::test]
    async fn refresh_keeps_old_token_when_no_rotation_cookie() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/auth/refresh");
            then.status(200)
                .header("Content-Type", "application/json")
                .json_body(serde_json::json!({
                    "access_token": "new-jwt",
                    "expires_in": 86400,
                    "user": { "id": 1, "username": "izzy", "groupId": 2 }
                }));
        });
        let api = ApiClient::new(server.base_url());
        let auth = api.refresh("old-token").await.unwrap();
        assert_eq!(auth.refresh_token, "old-token");
    }

    #[tokio::test]
    async fn extract_cookie_picks_named_cookie_among_many() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/auth/login");
            then.status(200)
                .header("Content-Type", "application/json")
                .header("Set-Cookie", "session_id=xyz; Path=/; HttpOnly")
                .header("Set-Cookie", "maparadar_refresh=abc123; Path=/; HttpOnly")
                .json_body(serde_json::json!({
                    "access_token": "jwt-token",
                    "expires_in": 86400,
                    "user": { "id": 1, "username": "izzy", "groupId": 2 }
                }));
        });
        let api = ApiClient::new(server.base_url());
        let auth = api.login("izzy", "pw").await.unwrap();
        assert_eq!(auth.refresh_token, "abc123");
    }
}
