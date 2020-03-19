use api::models::UserProfileAttributes;
use std::borrow::Cow;
use validator::Validate;

#[test]
fn user_profile_attributes_validate() {
    let mut user_parameters: UserProfileAttributes = Default::default();
    user_parameters.phone = Some(Some("abc".into()));
    user_parameters.email = Some("abc".into());
    user_parameters.profile_pic_url = Some(Some("abc".into()));
    user_parameters.thumb_profile_pic_url = Some(Some("abc".into()));
    user_parameters.cover_photo_url = Some(Some("abc".into()));

    let result = user_parameters.validate();
    assert!(result.is_err());
    let errors = result.unwrap_err().field_errors();

    assert!(errors.contains_key("email"));
    assert_eq!(errors["email"].len(), 1);
    assert_eq!(errors["email"][0].code, "email");
    assert_eq!(errors["email"][0].message, Some(Cow::from("Email is invalid")));

    assert!(errors.contains_key("profile_pic_url"));
    assert_eq!(errors["profile_pic_url"].len(), 1);
    assert_eq!(errors["profile_pic_url"][0].code, "url");
    assert_eq!(
        errors["profile_pic_url"][0].message,
        Some(Cow::from("Profile pic URL is invalid"))
    );

    assert!(errors.contains_key("thumb_profile_pic_url"));
    assert_eq!(errors["thumb_profile_pic_url"].len(), 1);
    assert_eq!(errors["thumb_profile_pic_url"][0].code, "url");
    assert_eq!(
        errors["thumb_profile_pic_url"][0].message,
        Some(Cow::from("Thumb profile pic URL is invalid"))
    );

    assert!(errors.contains_key("cover_photo_url"));
    assert_eq!(errors["cover_photo_url"].len(), 1);
    assert_eq!(errors["cover_photo_url"][0].code, "url");
    assert_eq!(
        errors["cover_photo_url"][0].message,
        Some(Cow::from("Cover photo URL is invalid"))
    );
}
