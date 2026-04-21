use std::time::Duration;

use anyhow::{Context, Result, bail};
use reqwest::{
    Url,
    blocking::Client,
    header::{ACCEPT, CONTENT_TYPE, HeaderMap, HeaderValue},
};
use serde::{Deserialize, Serialize};

use crate::config::Config;

#[derive(Debug, Clone)]
pub struct Session {
    client: Client,
    server_url: String,
    auth_token: String,
    pub user_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MediaItem {
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Type")]
    pub kind: String,
    #[serde(rename = "IsFolder", default)]
    pub is_folder: bool,
    #[serde(rename = "Overview")]
    pub overview: Option<String>,
    #[serde(rename = "ProductionYear")]
    pub production_year: Option<u16>,
    #[serde(rename = "RunTimeTicks")]
    pub runtime_ticks: Option<u64>,
    #[serde(rename = "ChildCount")]
    pub child_count: Option<u32>,
    #[serde(rename = "MediaType")]
    pub media_type: Option<String>,
    #[serde(rename = "CollectionType")]
    pub collection_type: Option<String>,
    #[serde(rename = "SeriesName")]
    pub series_name: Option<String>,
    #[serde(rename = "IndexNumber")]
    pub index_number: Option<u32>,
    #[serde(rename = "ParentIndexNumber")]
    pub parent_index_number: Option<u32>,
    #[serde(rename = "CommunityRating")]
    pub community_rating: Option<f32>,
    #[serde(rename = "OfficialRating")]
    pub official_rating: Option<String>,
    #[serde(rename = "Genres", default)]
    pub genres: Vec<String>,
    #[serde(rename = "UserData", default)]
    pub user_data: UserData,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct UserData {
    #[serde(rename = "Played", default)]
    pub played: bool,
    #[serde(rename = "IsFavorite", default)]
    pub is_favorite: bool,
}

impl MediaItem {
    pub fn is_playable(&self) -> bool {
        if self.is_folder {
            return false;
        }

        matches!(
            self.kind.as_str(),
            "Movie" | "Episode" | "Video" | "Audio" | "MusicVideo"
        ) || matches!(self.media_type.as_deref(), Some("Video" | "Audio"))
    }


}

impl Session {
    pub fn connect(config: &Config) -> Result<Self> {
        let auth_token = match config.api_key.as_deref() {
            Some(token) if !token.trim().is_empty() => token.to_string(),
            _ => authenticate_with_password(config)?,
        };

        let client = build_client(&auth_token)?;
        let user_id = match config.user_id.as_deref() {
            Some(user_id) if !user_id.trim().is_empty() => user_id.to_string(),
            _ => resolve_user_id(&client, &config.server_url)?,
        };

        Ok(Self {
            client,
            server_url: config.server_url.clone(),
            auth_token,
            user_id,
        })
    }

    pub fn fetch_root(&self) -> Result<Vec<MediaItem>> {
        let url = format!("{}/Users/{}/Views", self.server_url, self.user_id);
        let response = self
            .client
            .get(url)
            .send()
            .context("failed to request Jellyfin root views")?
            .error_for_status()
            .context("Jellyfin rejected the root views request")?;

        let payload: ItemQueryResult = response
            .json()
            .context("failed to decode Jellyfin root views response")?;
        Ok(payload.items)
    }

    pub fn fetch_children(&self, parent_id: &str) -> Result<Vec<MediaItem>> {
        let url = format!("{}/Users/{}/Items", self.server_url, self.user_id);
        let response = self
            .client
            .get(url)
            .query(&[
                ("ParentId", parent_id),
                ("Recursive", "false"),
                ("SortBy", "SortName"),
                ("SortOrder", "Ascending"),
                (
                    "Fields",
                    "Overview,ChildCount,ProductionYear,RunTimeTicks,MediaType,CollectionType,Genres,UserData",
                ),
            ])
            .send()
            .context("failed to request Jellyfin children")?
            .error_for_status()
            .context("Jellyfin rejected the child item request")?;

        let payload: ItemQueryResult = response
            .json()
            .context("failed to decode Jellyfin child item response")?;
        Ok(payload.items)
    }

    pub fn fetch_shuffled_episodes(
        &self,
        series_id: &str,
        limit: u32,
    ) -> Result<Vec<MediaItem>> {
        let url = format!("{}/Users/{}/Items", self.server_url, self.user_id);
        let limit = limit.to_string();
        let response = self
            .client
            .get(url)
            .query(&[
                ("ParentId", series_id),
                ("Recursive", "true"),
                ("IncludeItemTypes", "Episode"),
                ("SortBy", "Random"),
                ("Limit", limit.as_str()),
                (
                    "Fields",
                    "Overview,ProductionYear,RunTimeTicks,MediaType,Genres,UserData",
                ),
            ])
            .send()
            .context("failed to request shuffled episodes")?
            .error_for_status()
            .context("Jellyfin rejected the shuffle request")?;

        let payload: ItemQueryResult = response
            .json()
            .context("failed to decode shuffled episode response")?;
        Ok(payload.items)
    }

    pub fn playback_url(&self, item_id: &str) -> Result<String> {
        let mut url = Url::parse(&format!("{}/Items/{item_id}/Download", self.server_url))
            .context("failed to build playback URL")?;
        url.query_pairs_mut().append_pair("download", "false");
        Ok(url.to_string())
    }

    pub fn auth_token(&self) -> &str {
        &self.auth_token
    }

    pub fn set_watched(&self, item_id: &str, watched: bool) -> Result<()> {
        let url = format!(
            "{}/Users/{}/PlayedItems/{item_id}",
            self.server_url, self.user_id
        );
        let request = if watched {
            self.client.post(url)
        } else {
            self.client.delete(url)
        };
        request
            .send()
            .context("failed to toggle watched state")?
            .error_for_status()
            .context("Jellyfin rejected the watched-state update")?;
        Ok(())
    }

    pub fn set_favorite(&self, item_id: &str, favorite: bool) -> Result<()> {
        let url = format!(
            "{}/Users/{}/FavoriteItems/{item_id}",
            self.server_url, self.user_id
        );
        let request = if favorite {
            self.client.post(url)
        } else {
            self.client.delete(url)
        };
        request
            .send()
            .context("failed to toggle favorite state")?
            .error_for_status()
            .context("Jellyfin rejected the favorite-state update")?;
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct ItemQueryResult {
    #[serde(rename = "Items", default)]
    items: Vec<MediaItem>,
}

#[derive(Debug, Deserialize)]
struct UserResponse {
    #[serde(rename = "Id")]
    id: String,
}

#[derive(Debug, Deserialize)]
struct AuthResponse {
    #[serde(rename = "AccessToken")]
    access_token: String,
}

#[derive(Debug, Serialize)]
struct AuthRequest<'a> {
    #[serde(rename = "Username")]
    username: &'a str,
    #[serde(rename = "Pw")]
    password: &'a str,
}

fn authenticate_with_password(config: &Config) -> Result<String> {
    let username = config
        .username
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .context("missing `username` in config")?;
    let password = config
        .password
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .context("missing `password` in config")?;

    let response = Client::new()
        .post(format!("{}/Users/AuthenticateByName", config.server_url))
        .header("X-Emby-Authorization", auth_header(None)?)
        .header(ACCEPT, HeaderValue::from_static("application/json"))
        .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
        .json(&AuthRequest { username, password })
        .send()
        .context("failed to authenticate with Jellyfin")?
        .error_for_status()
        .context("Jellyfin rejected the supplied username/password")?;

    let payload: AuthResponse = response
        .json()
        .context("failed to decode Jellyfin authentication response")?;

    if payload.access_token.trim().is_empty() {
        bail!("Jellyfin authentication returned an empty access token");
    }

    Ok(payload.access_token)
}

fn resolve_user_id(client: &Client, server_url: &str) -> Result<String> {
    let response = client
        .get(format!("{server_url}/Users/Me"))
        .send()
        .context("failed to resolve current Jellyfin user")?
        .error_for_status()
        .context("Jellyfin rejected the current-user lookup")?;

    let payload: UserResponse = response
        .json()
        .context("failed to decode Jellyfin current-user response")?;
    Ok(payload.id)
}

fn build_client(auth_token: &str) -> Result<Client> {
    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
    headers.insert("X-MediaBrowser-Token", HeaderValue::from_str(auth_token)?);
    headers.insert(
        "X-Emby-Authorization",
        HeaderValue::from_str(&auth_header(Some(auth_token))?)?,
    );

    Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(15))
        .connect_timeout(Duration::from_secs(5))
        .build()
        .context("failed to build Jellyfin HTTP client")
}

fn auth_header(token: Option<&str>) -> Result<String> {
    let mut header = format!(
        r#"MediaBrowser Client="geltui", Device="geltui", DeviceId="geltui", Version="{}""#,
        env!("CARGO_PKG_VERSION"),
    );

    if let Some(token) = token {
        if token.trim().is_empty() {
            bail!("Jellyfin auth token was empty");
        }

        header.push_str(&format!(r#", Token="{token}""#));
    }

    Ok(header)
}

pub(crate) fn format_runtime(ticks: u64) -> String {
    let total_seconds = ticks / 10_000_000;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;

    if hours > 0 {
        format!("{hours}h {minutes}m")
    } else {
        format!("{minutes}m")
    }
}
