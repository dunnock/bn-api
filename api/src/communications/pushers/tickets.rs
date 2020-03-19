use crate::errors::*;
use db::models::*;
use diesel::pg::PgConnection;
use itertools::Itertools;

pub fn tickets_received(to_user: &User, from_user: &User, conn: &PgConnection) -> Result<(), ApiError> {
    let tokens = to_user
        .push_notification_tokens(conn)?
        .into_iter()
        .map(|pt| pt.token)
        .collect_vec();

    if tokens.len() > 0 {
        let body = format!("{} has sent you some tickets.", from_user.full_name(),);

        Communication::new(
            CommunicationType::Push,
            body,
            None,
            None,
            CommAddress::from_vec(tokens),
            None,
            None,
            Some(vec!["transfers"]),
            None,
        )
        .queue(conn)?;
    }
    Ok(())
}
