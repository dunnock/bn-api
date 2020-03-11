use crate::config::Config;
use crate::errors::*;
use bigneon_db::models::*;
use diesel::PgConnection;

pub fn user_registered(
    user_first_name: String,
    user_email: String,
    config: &Config,
    conn: &PgConnection,
) -> Result<(), BigNeonError> {
    let source = CommAddress::from(config.communication_default_source_email.clone());
    let destinations = CommAddress::from(user_email);
    let title = "BigNeon Registration".to_string();
    let template_id = config.sendgrid_template_bn_user_registered.clone();
    let mut template_data = TemplateData::new();
    template_data.insert("name".to_string(), user_first_name.clone());
    Communication::new(
        CommunicationType::EmailTemplate,
        title,
        None,
        Some(source),
        destinations,
        Some(template_id),
        Some(vec![template_data]),
        Some(vec!["user_registered", "account"]),
        None,
    )
    .queue(conn)?;

    Ok(())
}

pub fn password_reset_email(config: &Config, user: &User) -> Communication {
    let password_reset_link = format!(
        "{}/password-reset?token={}",
        config.front_end_url.clone(),
        user.password_reset_token.expect("Password reset token is not set")
    );
    let email: &str = user.email.as_ref().expect("Email is not set");
    let source = CommAddress::from(config.communication_default_source_email.clone());
    let destinations = CommAddress::from(email.to_string());
    let title = "BigNeon Password reset request".to_string();
    let template_id = config.email_templates.password_reset.to_string();
    let mut template_data = TemplateData::new();
    template_data.insert("name".to_string(), user.full_name());
    template_data.insert("password_reset_link".to_string(), password_reset_link);
    Communication::new(
        CommunicationType::EmailTemplate,
        title,
        None,
        Some(source),
        destinations,
        Some(template_id),
        Some(vec![template_data]),
        Some(vec!["password_reset", "account"]),
        None,
    )
}

pub fn invite_user_email(config: &Config, user: &User, conn: &PgConnection) -> Result<(), BigNeonError> {
    let invite_link = format!(
        "{}/password-reset?token={}&invite=true",
        config.front_end_url.clone(),
        user.password_reset_token.expect("Password reset token is not set")
    );

    let email: &str = user.email.as_ref().expect("Email is not set");
    let source = CommAddress::from(config.communication_default_source_email.clone());
    let destinations = CommAddress::from(email.to_string());
    let title = "BigNeon Invite".to_string();
    let template_id = config.sendgrid_template_bn_user_invite.clone();
    let mut template_data = TemplateData::new();
    template_data.insert("name".to_string(), user.full_name());
    template_data.insert("invite_link".to_string(), invite_link);
    Communication::new(
        CommunicationType::EmailTemplate,
        title,
        None,
        Some(source),
        destinations,
        Some(template_id),
        Some(vec![template_data]),
        Some(vec!["user_invite", "account"]),
        None,
    )
    .queue(conn)?;

    Ok(())
}
