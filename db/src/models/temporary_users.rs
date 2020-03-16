use chrono::NaiveDateTime;
use diesel;
use diesel::prelude::*;
use models::*;
use schema;
use schema::{temporary_user_links, temporary_users};
use utils::errors::*;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Eq, Hash, PartialEq, Identifiable, Queryable, QueryableByName)]
#[table_name = "temporary_users"]
pub struct TemporaryUser {
    pub id: Uuid,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl TemporaryUser {
    pub fn find_or_build_from_transfer(
        transfer: &Transfer,
        conn: &PgConnection,
    ) -> Result<Option<TemporaryUser>, DatabaseError> {
        if let Some(id) = Transfer::temporary_user_id(transfer.transfer_address.clone()) {
            if let Some(temporary_user) = TemporaryUser::find(id, conn).optional()? {
                return Ok(Some(temporary_user));
            }

            let mut email: Option<String> = None;
            let mut phone: Option<String> = None;

            if let Some(transfer_message_type) = transfer.transfer_message_type {
                match transfer_message_type {
                    TransferMessageType::Email => {
                        email = transfer.transfer_address.clone();
                    }
                    TransferMessageType::Phone => {
                        phone = transfer.transfer_address.clone();
                    }
                }
            }

            let mut temporary_user = TemporaryUser::create(id, email, phone);
            temporary_user.created_at = Some(transfer.created_at);
            return Ok(Some(temporary_user.commit(transfer.source_user_id, conn)?));
        }

        Ok(None)
    }

    pub fn associate_user(&self, user_id: Uuid, conn: &PgConnection) -> Result<(), DatabaseError> {
        diesel::insert_into(temporary_user_links::table)
            .values((
                temporary_user_links::user_id.eq(user_id),
                temporary_user_links::temporary_user_id.eq(self.id),
            ))
            .on_conflict_do_nothing()
            .execute(conn)
            .to_db_error(ErrorCode::InsertError, "Could not insert domain event published")?;
        Ok(())
    }

    pub fn create(id: Uuid, email: Option<String>, phone: Option<String>) -> NewTemporaryUser {
        NewTemporaryUser {
            id,
            email,
            phone,
            created_at: None,
        }
    }

    pub fn users(&self, conn: &PgConnection) -> Result<Vec<User>, DatabaseError> {
        temporary_user_links::table
            .inner_join(schema::users::table)
            .filter(temporary_user_links::temporary_user_id.eq(self.id))
            .select(schema::users::all_columns)
            .order_by(temporary_user_links::created_at.desc())
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not find users for temporary user")
    }
    pub fn find_by_user_id(user_id: Uuid, conn: &PgConnection) -> Result<Vec<TemporaryUser>, DatabaseError> {
        temporary_user_links::table
            .inner_join(temporary_users::table.on(temporary_users::id.eq(temporary_user_links::temporary_user_id)))
            .filter(temporary_user_links::user_id.eq(user_id))
            .select(temporary_users::all_columns)
            .distinct()
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Error loading temporary users")
    }

    pub fn find(id: Uuid, conn: &PgConnection) -> Result<TemporaryUser, DatabaseError> {
        temporary_users::table
            .find(id)
            .first::<TemporaryUser>(conn)
            .to_db_error(ErrorCode::QueryError, "Error loading temporary user")
    }
}

#[derive(Clone, Deserialize, Insertable)]
#[table_name = "temporary_users"]
pub struct NewTemporaryUser {
    pub id: Uuid,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub created_at: Option<NaiveDateTime>,
}

impl NewTemporaryUser {
    pub fn commit(self, user_id: Uuid, conn: &PgConnection) -> Result<TemporaryUser, DatabaseError> {
        let temporary_user: TemporaryUser = diesel::insert_into(temporary_users::table)
            .values(self)
            .get_result(conn)
            .to_db_error(ErrorCode::InsertError, "Could not insert domain event publisher")?;

        let mut domain_event = DomainEvent::create(
            DomainEventTypes::TemporaryUserCreated,
            "Temporary user created".to_string(),
            Tables::TemporaryUsers,
            Some(temporary_user.id),
            Some(user_id),
            None,
        );
        domain_event.created_at = Some(temporary_user.created_at);
        domain_event.commit(conn)?;

        Ok(temporary_user)
    }
}
