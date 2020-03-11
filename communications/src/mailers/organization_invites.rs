use crate::config::Config;
use crate::errors::*;
use bigneon_db::models::*;
use diesel::pg::PgConnection;

pub fn invite_user_to_organization_email(
    config: &Config,
    invite: &OrganizationInvite,
    org: &Organization,
    recipient_name: &str,
    conn: &PgConnection,
) -> Result<(), BigNeonError> {
    let invite_link_accept = format!(
        "{}/invites/accept?token={}",
        config.front_end_url.clone(),
        invite.security_token.expect("Security token is not set")
    );

    let source = CommAddress::from(config.communication_default_source_email.clone());
    let destinations = CommAddress::from(invite.user_email.clone());
    let title = "BigNeon Invites".to_string();
    let template_id = config.email_templates.org_invite.to_string();
    let mut template_data = TemplateData::new();
    template_data.insert("name".to_string(), recipient_name.into());
    template_data.insert("org".to_string(), org.name.clone());
    template_data.insert("invite_link_accept".to_string(), invite_link_accept);
    Communication::new(
        CommunicationType::EmailTemplate,
        title,
        None,
        Some(source),
        destinations,
        Some(template_id),
        Some(vec![template_data]),
        Some(vec!["org_invites".to_string()]),
        None,
    )
    .queue(conn)?;

    Ok(())
}
