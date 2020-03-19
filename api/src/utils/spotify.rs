use crate::errors::{ApiError, ApplicationError, ApplicationErrorType};
use crate::models::CreateArtistRequest;
use log::Level::*;
use reqwest::Client;
use serde_json;
use serde_json::Value;
use std::default::Default;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use url::percent_encoding::{utf8_percent_encode, PATH_SEGMENT_ENCODE_SET};

const SPOTIFY_URL_AUTH: &'static str = "https://accounts.spotify.com/api/token";
const LOG_TARGET: &'static str = "bigneon::utils::spotify";

lazy_static! {
    pub static ref SINGLETON: Spotify = Spotify { ..Default::default() };
}

#[derive(Deserialize)]
struct AuthResponse {
    access_token: String,
    expires_in: u64,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SpotifyArtist {
    #[serde(rename = "spotify_id")]
    pub id: Option<String>,
    pub name: Option<String>,
    pub href: Option<String>,
}

#[derive(Debug, Default)]
pub struct Spotify {
    pub auth_token: Arc<RwLock<Option<String>>>,
    access_token: Arc<RwLock<SpotifyAccessToken>>,
}

impl Spotify {
    pub fn set_auth_token(&self, token: &str) {
        let mut auth_token = self.auth_token.write().unwrap();
        *auth_token = Some(token.to_owned());
    }

    pub async fn connect(&self) -> Result<(), ApiError> {
        match *self.auth_token.read().unwrap() {
            Some(ref token) => {
                let mut access_token = self.access_token.write().unwrap();
                // This code must have no panics as that will poison the RwLock
                // which is a case we don't expect/handle when read locking (`read().unwrap()`)
                if (*access_token).is_expired() {
                    *access_token = self.authenticate(token).await?;
                    jlog!(Info, LOG_TARGET, "Fetching new Spotify access token", {
                        "expires_at": (*access_token).expires_at
                    });
                } else {
                    jlog!(Info, LOG_TARGET, "Reusing valid Spotify access token", {
                        "expires_at": (* access_token).expires_at
                    });
                }
                Ok(())
            }
            None => Err(ApplicationError::new_with_type(
                ApplicationErrorType::ServerConfigError,
                "No Spotify auth key provided".to_owned(),
            )
            .into()),
        }
    }

    async fn authenticate(&self, auth_token: &str) -> Result<SpotifyAccessToken, reqwest::Error> {
        let reqwest_client = Client::new();
        let params = [("grant_type", "client_credentials")];
        let res: AuthResponse = reqwest_client
            .post(SPOTIFY_URL_AUTH)
            .header("Authorization", format!("Basic {}", auth_token))
            .form(&params)
            .send()
            .await?
            .json()
            .await?;

        let start = SystemTime::now();
        let since_the_epoch = start.duration_since(UNIX_EPOCH).expect("Time went backwards");
        let expires_at = since_the_epoch.as_secs() + res.expires_in;

        Ok(SpotifyAccessToken {
            token: res.access_token,
            expires_at,
        })
    }

    pub async fn search(&self, q: String) -> Result<Vec<CreateArtistRequest>, ApiError> {
        {
            // For search we ignore not having an auth token
            if self.auth_token.read().unwrap().is_none() {
                return Ok(vec![]);
            }
        }

        self.connect().await?;

        // Lock access token for reading
        let access_token = self.access_token.read().unwrap();

        let reqwest_client = Client::new();
        let access_token = &*access_token.token;
        let encoded_q = utf8_percent_encode(&q, PATH_SEGMENT_ENCODE_SET);
        let url = format!(
            "https://api.spotify.com/v1/search?q={}&type=artist&access_token={}",
            encoded_q, access_token
        );
        let res = reqwest_client.get(&url).send().await?.text().await?;
        let result: Value = serde_json::from_str(&res)?;
        if result.get("error").is_some() {
            return Err(ApplicationError::new(
                result["error"]["message"]
                    .as_str()
                    .unwrap_or("Invalid Spotify Response")
                    .to_string(),
            )
            .into());
        }
        let spotify_artists = result["artists"]["items"]
            .as_array()
            .unwrap()
            .into_iter()
            .map(|item| {
                let artist = item;
                let image_url = Spotify::get_image_from_artist(&artist["images"], Some(600), None);
                CreateArtistRequest {
                    name: artist["name"].as_str().map(|s| s.to_string()),
                    bio: Some("".to_string()),
                    spotify_id: artist["id"].as_str().map(|s| s.to_string()),
                    genres: artist["genres"].as_array().map(|a| {
                        a.iter()
                            .map(|s| s.as_str().map(|g| g.to_string()))
                            .filter(|s| s.is_some())
                            .map(|s| s.unwrap())
                            .collect()
                    }),
                    image_url,
                    other_image_urls: artist["images"].as_array().map(|a| {
                        a.iter()
                            .map(|i| i["url"].as_str().map(|s| s.to_string()))
                            .filter(|i| i.is_some())
                            .map(|i| i.unwrap())
                            .collect()
                    }),
                    ..Default::default()
                }
            })
            .collect();
        Ok(spotify_artists)
    }

    pub async fn read_artist(&self, artist_id: &str) -> Result<Option<CreateArtistRequest>, ApiError> {
        self.connect().await?;

        // Lock access token for reading
        let access_token = self.access_token.read().unwrap();

        let reqwest_client = Client::new();

        let url = format!("https://api.spotify.com/v1/artists/{}", artist_id);

        let res = reqwest_client
            .get(&url)
            .header("Authorization", format!("Bearer {}", &*access_token.token))
            .send()
            .await?
            .text()
            .await?;

        let artist: Value = serde_json::from_str(&res)?;
        if artist.get("error").is_some() {
            return Err(ApplicationError::new(
                artist["error"]["message"]
                    .as_str()
                    .unwrap_or("Invalid Spotify Response")
                    .to_string(),
            )
            .into());
        } else {
            let image_url = Spotify::get_image_from_artist(&artist["images"], Some(600), None);
            let thumb_image_url = Spotify::get_image_from_artist(&artist["images"], None, Some(300));

            let create_artist = CreateArtistRequest {
                name: artist["name"].as_str().map(|s| s.to_string()),
                bio: Some("".to_string()),
                spotify_id: artist["id"].as_str().map(|s| s.to_string()),
                genres: artist["genres"].as_array().map(|a| {
                    a.iter()
                        .map(|s| s.as_str().map(|g| g.to_string()))
                        .filter(|s| s.is_some())
                        .map(|s| s.unwrap())
                        .collect()
                }),
                image_url,
                thumb_image_url,
                other_image_urls: artist["images"].as_array().map(|a| {
                    a.iter()
                        .map(|i| i["url"].as_str().map(|s| s.to_string()))
                        .filter(|i| i.is_some())
                        .map(|i| i.unwrap())
                        .collect()
                }),

                ..Default::default()
            };
            Ok(Some(create_artist))
        }
    }

    pub fn get_image_from_artist(
        image_array: &Value,
        min_width: Option<i64>,
        max_width: Option<i64>,
    ) -> Option<String> {
        let array = match image_array.as_array() {
            Some(u) => u,
            None => return None,
        };

        if let Some(width) = min_width {
            for i in 0..array.len() {
                let value = array.get(i);
                if value.is_none() {
                    return None;
                }
                let value = value.unwrap();
                if value["width"].as_i64().unwrap_or(0) < width {
                    continue;
                }
                return value["url"].as_str().map(|s| s.to_string());
            }
        };

        if let Some(width) = max_width {
            for i in 0..array.len() {
                let value = array.get(i);
                if value.is_none() {
                    return None;
                }
                let value = value.unwrap();
                if value["width"].as_i64().unwrap_or(99999999) > width {
                    continue;
                }
                return value["url"].as_str().map(|s| s.to_string());
            }
        }

        None
    }
}

#[derive(Debug, Default)]
struct SpotifyAccessToken {
    /// Ephemeral spotify access token
    token: String,
    /// Expiry of access token (Unix Epoch)
    expires_at: u64,
}

impl SpotifyAccessToken {
    pub fn is_expired(&self) -> bool {
        let since_the_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");

        // If we're a minute or less away from expiring,
        // get a new token
        self.expires_at < since_the_epoch.as_secs() + 60
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn spotify_access_token_is_expired() {
        let subject = SpotifyAccessToken {
            token: "dummy".to_string(),
            expires_at: 0,
        };

        assert!(subject.is_expired());

        let since_the_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");

        let subject = SpotifyAccessToken {
            token: "dummy".to_string(),
            expires_at: since_the_epoch.as_secs(),
        };

        assert!(subject.is_expired());

        let subject = SpotifyAccessToken {
            token: "dummy".to_string(),
            expires_at: since_the_epoch.as_secs() + 1000,
        };

        assert!(!subject.is_expired());
    }
}
