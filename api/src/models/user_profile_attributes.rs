use db::models::{deserialize_unless_blank, double_option_deserialize_unless_blank, UserEditableAttributes};
use validator::Validate;

#[derive(Default, Deserialize, Validate)]
pub struct UserProfileAttributes {
    #[serde(default, deserialize_with = "double_option_deserialize_unless_blank")]
    pub first_name: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option_deserialize_unless_blank")]
    pub last_name: Option<Option<String>>,
    #[validate(email(message = "Email is invalid"))]
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub email: Option<String>,
    #[serde(default, deserialize_with = "double_option_deserialize_unless_blank")]
    pub phone: Option<Option<String>>,
    #[validate(url(message = "Profile pic URL is invalid"))]
    #[serde(default, deserialize_with = "double_option_deserialize_unless_blank")]
    pub profile_pic_url: Option<Option<String>>,
    #[validate(url(message = "Thumb profile pic URL is invalid"))]
    #[serde(default, deserialize_with = "double_option_deserialize_unless_blank")]
    pub thumb_profile_pic_url: Option<Option<String>>,
    #[validate(url(message = "Cover photo URL is invalid"))]
    #[serde(default, deserialize_with = "double_option_deserialize_unless_blank")]
    pub cover_photo_url: Option<Option<String>>,
}

impl From<UserProfileAttributes> for UserEditableAttributes {
    fn from(attributes: UserProfileAttributes) -> Self {
        UserEditableAttributes {
            first_name: attributes.first_name,
            last_name: attributes.last_name,
            email: attributes.email,
            phone: attributes.phone,
            profile_pic_url: attributes.profile_pic_url,
            thumb_profile_pic_url: attributes.thumb_profile_pic_url,
            cover_photo_url: attributes.cover_photo_url,
            ..Default::default()
        }
    }
}
