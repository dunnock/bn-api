use db::models::ArtistEditableAttributes;
use db::models::{deserialize_unless_blank, double_option_deserialize_unless_blank};

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone, Default)]
pub struct UpdateArtistRequest {
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub bio: Option<String>,
    #[serde(default, deserialize_with = "double_option_deserialize_unless_blank")]
    pub image_url: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option_deserialize_unless_blank")]
    pub thumb_image_url: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option_deserialize_unless_blank")]
    pub website_url: Option<Option<String>>,
    pub youtube_video_urls: Option<Vec<String>>,
    #[serde(default, deserialize_with = "double_option_deserialize_unless_blank")]
    pub facebook_username: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option_deserialize_unless_blank")]
    pub instagram_username: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option_deserialize_unless_blank")]
    pub snapchat_username: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option_deserialize_unless_blank")]
    pub soundcloud_username: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option_deserialize_unless_blank")]
    pub bandcamp_username: Option<Option<String>>,
    pub genres: Option<Vec<String>>,
    #[serde(default, deserialize_with = "double_option_deserialize_unless_blank")]
    pub main_genre: Option<Option<String>>,
}

impl From<UpdateArtistRequest> for ArtistEditableAttributes {
    fn from(attributes: UpdateArtistRequest) -> Self {
        ArtistEditableAttributes {
            name: attributes.name.clone(),
            bio: attributes.bio.clone(),
            image_url: attributes.image_url.clone(),
            thumb_image_url: attributes.thumb_image_url.clone(),
            website_url: attributes.website_url.clone(),
            youtube_video_urls: attributes.youtube_video_urls.clone(),
            facebook_username: attributes.facebook_username.clone(),
            instagram_username: attributes.instagram_username.clone(),
            snapchat_username: attributes.snapchat_username.clone(),
            soundcloud_username: attributes.soundcloud_username.clone(),
            bandcamp_username: attributes.bandcamp_username.clone(),
            main_genre_id: None,
        }
    }
}
