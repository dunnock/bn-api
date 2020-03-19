use chrono::NaiveDateTime;
use db::models::{DisplayArtist, NewArtist};
use uuid::Uuid;

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone, Default)]
pub struct CreateArtistRequest {
    pub id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub is_private: Option<bool>,
    pub name: Option<String>,
    pub bio: Option<String>,
    pub image_url: Option<String>,
    pub thumb_image_url: Option<String>,
    pub website_url: Option<String>,
    pub youtube_video_urls: Option<Vec<String>>,
    pub facebook_username: Option<String>,
    pub instagram_username: Option<String>,
    pub snapchat_username: Option<String>,
    pub soundcloud_username: Option<String>,
    pub bandcamp_username: Option<String>,
    pub spotify_id: Option<String>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub other_image_urls: Option<Vec<String>>,
    pub genres: Option<Vec<String>>,
}

impl Into<NewArtist> for CreateArtistRequest {
    fn into(self) -> NewArtist {
        let create_artist = self.clone();
        NewArtist {
            organization_id: create_artist.organization_id,
            name: create_artist.name.unwrap_or("".to_string()),
            bio: create_artist.bio.unwrap_or("".to_string()),
            image_url: create_artist.image_url,
            thumb_image_url: create_artist.thumb_image_url,
            website_url: create_artist.website_url,
            youtube_video_urls: create_artist.youtube_video_urls,
            facebook_username: create_artist.facebook_username,
            instagram_username: create_artist.instagram_username,
            snapchat_username: create_artist.snapchat_username,
            soundcloud_username: create_artist.soundcloud_username,
            bandcamp_username: create_artist.bandcamp_username,
            spotify_id: create_artist.spotify_id,
            other_image_urls: create_artist.other_image_urls,
        }
    }
}

impl From<DisplayArtist> for CreateArtistRequest {
    fn from(artist: DisplayArtist) -> Self {
        CreateArtistRequest {
            id: Some(artist.id),
            organization_id: artist.organization_id,
            is_private: Some(artist.is_private),
            name: Some(artist.name),
            bio: Some(artist.bio),
            image_url: artist.image_url,
            thumb_image_url: artist.thumb_image_url,
            website_url: artist.website_url,
            youtube_video_urls: Some(artist.youtube_video_urls),
            facebook_username: artist.facebook_username,
            instagram_username: artist.instagram_username,
            snapchat_username: artist.snapchat_username,
            soundcloud_username: artist.soundcloud_username,
            bandcamp_username: artist.bandcamp_username,
            genres: Some(artist.genres),
            spotify_id: artist.spotify_id,
            created_at: Some(artist.created_at),
            updated_at: Some(artist.updated_at),
            other_image_urls: artist.other_image_urls,
        }
    }
}
