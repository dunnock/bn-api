use chrono::prelude::Utc;
use chrono::Duration;
use chrono::NaiveDateTime;
use diesel;
use diesel::dsl::{exists, select};
use diesel::expression::dsl;
use diesel::expression::sql_literal::sql;
use diesel::pg::types::sql_types::{Array, Jsonb};
use diesel::prelude::*;
use diesel::sql_query;
use diesel::sql_types::{BigInt, Bool, Nullable, Text, Timestamp, Uuid as dUuid};
use models::*;
use schema::{event_users, events, genres, organization_users, organizations, user_genres, users};
use serde_json::Value;
use std::cmp::Ordering;
use std::collections::HashMap;
use utils::errors::Optional;
use utils::errors::{ConvertToDatabaseError, DatabaseError, ErrorCode};
use utils::pagination::Paginate;
use utils::passwords::PasswordHash;
use utils::rand::random_alpha_string;
use uuid::Uuid;
use validator::*;
use validators::{self, *};

#[derive(Insertable, PartialEq, Debug, Validate)]
#[table_name = "users"]
pub struct NewUser {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    #[validate(email(message = "Email is invalid"))]
    pub email: Option<String>,
    pub phone: Option<String>,
    pub hashed_pw: String,
    role: Vec<Roles>,
}

#[derive(Queryable, Identifiable, PartialEq, Debug, Clone, QueryableByName)]
#[table_name = "users"]
pub struct User {
    pub id: Uuid,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub profile_pic_url: Option<String>,
    pub thumb_profile_pic_url: Option<String>,
    pub cover_photo_url: Option<String>,
    pub hashed_pw: String,
    pub password_modified_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
    pub last_used: Option<NaiveDateTime>,
    pub active: bool,
    pub role: Vec<Roles>,
    pub password_reset_token: Option<Uuid>,
    pub password_reset_requested_at: Option<NaiveDateTime>,
    pub updated_at: NaiveDateTime,
    pub last_cart_id: Option<Uuid>,
    pub accepted_terms_date: Option<NaiveDateTime>,
    pub invited_at: Option<NaiveDateTime>,
    pub deleted_at: Option<NaiveDateTime>,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct DisplayUser {
    pub id: Uuid,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub profile_pic_url: Option<String>,
    pub thumb_profile_pic_url: Option<String>,
    pub cover_photo_url: Option<String>,
    pub is_org_owner: bool,
}

#[derive(AsChangeset, Default, Deserialize, Validate, Clone, Serialize)]
#[table_name = "users"]
pub struct UserEditableAttributes {
    pub first_name: Option<Option<String>>,
    pub last_name: Option<Option<String>>,
    #[validate(email(message = "Email is invalid"))]
    pub email: Option<String>,
    pub phone: Option<Option<String>>,
    pub active: Option<bool>,
    pub role: Option<Vec<Roles>>,
    #[validate(url(message = "Profile pic URL is invalid"))]
    pub profile_pic_url: Option<Option<String>>,
    #[validate(url(message = "Thumb profile pic URL is invalid"))]
    pub thumb_profile_pic_url: Option<Option<String>>,
    #[validate(url(message = "Cover photo URL is invalid"))]
    pub cover_photo_url: Option<Option<String>>,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct FanProfile {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub facebook_linked: bool,
    pub revenue_in_cents: u32,
    pub tickets_owned: u32,
    pub ticket_sales: u32,
    pub profile_pic_url: Option<String>,
    pub thumb_profile_pic_url: Option<String>,
    pub cover_photo_url: Option<String>,
    pub created_at: NaiveDateTime,
    pub attendance_information: Vec<AttendanceInformation>,
    pub deleted_at: Option<NaiveDateTime>,
}

#[derive(Debug, Deserialize, PartialEq, Queryable, QueryableByName, Serialize)]
pub struct AttendanceInformation {
    #[sql_type = "dUuid"]
    pub event_id: Uuid,
    #[sql_type = "Text"]
    pub event_name: String,
    #[sql_type = "Nullable<Timestamp>"]
    pub event_start: Option<NaiveDateTime>,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct UserTransferActivitySummary {
    pub event: DisplayEvent,
    pub ticket_activity_items: HashMap<Uuid, Vec<ActivityItem>>,
}

impl PartialOrd for User {
    fn partial_cmp(&self, other: &User) -> Option<Ordering> {
        Some(self.id.cmp(&other.id))
    }
}

impl NewUser {
    pub fn commit(&self, current_user_id: Option<Uuid>, conn: &PgConnection) -> Result<User, DatabaseError> {
        self.validate()?;
        let user: User = diesel::insert_into(users::table)
            .values(self)
            .get_result(conn)
            .to_db_error(ErrorCode::InsertError, "Could not create new user")?;

        DomainEvent::create(DomainEventTypes::UserCreated, "User created".to_string(), Tables::Users, Some(user.id), current_user_id, Some(json!({"first_name": self.first_name, "last_name": self.last_name, "email": self.email, "phone": self.phone}))).commit(conn)?;
        Wallet::create_for_user(user.id, "Default".to_string(), true, conn)?;

        Ok(user)
    }
}

impl User {
    pub fn is_attending_event(user_id: Uuid, event_id: Uuid, conn: &PgConnection) -> Result<bool, DatabaseError> {
        use schema::*;
        select(exists(
            wallets::table
                .inner_join(ticket_instances::table.on(ticket_instances::wallet_id.eq(wallets::id)))
                .inner_join(assets::table.on(ticket_instances::asset_id.eq(assets::id)))
                .inner_join(ticket_types::table.on(assets::ticket_type_id.eq(ticket_types::id)))
                .filter(
                    ticket_instances::status
                        .eq_any(vec![TicketInstanceStatus::Redeemed, TicketInstanceStatus::Purchased]),
                )
                .filter(wallets::user_id.eq(user_id))
                .filter(ticket_types::event_id.eq(event_id)),
        ))
        .get_result(conn)
        .to_db_error(ErrorCode::QueryError, "Could not check if user is attending event")
    }

    pub fn all(paging: &PagingParameters, conn: &PgConnection) -> Result<(Vec<User>, i64), DatabaseError> {
        DatabaseError::wrap(
            ErrorCode::QueryError,
            "Unable to load all users",
            users::table
                .order_by(users::created_at)
                .paginate(paging.page.unwrap_or(0) as i64)
                .per_page(paging.limit.unwrap_or(100) as i64)
                .load_and_count_pages(conn),
        )
    }

    pub fn admins(conn: &PgConnection) -> Result<Vec<User>, DatabaseError> {
        users::table
            .filter(users::role.overlaps_with(vec![Roles::Admin, Roles::Super]))
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load admin users")
    }

    pub fn genres(&self, conn: &PgConnection) -> Result<Vec<String>, DatabaseError> {
        genres::table
            .inner_join(user_genres::table.on(user_genres::genre_id.eq(genres::id)))
            .filter(user_genres::user_id.eq(self.id))
            .select(genres::name)
            .order_by(genres::name)
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not get genres for user")
    }

    pub fn update_genre_info(&self, conn: &PgConnection) -> Result<(), DatabaseError> {
        let query = r#"
            INSERT INTO user_genres (user_id, genre_id)
            SELECT DISTINCT w.user_id, eg.genre_id
            FROM event_genres eg
            JOIN ticket_types tt ON tt.event_id = eg.event_id
            JOIN assets a ON a.ticket_type_id = tt.id
            JOIN ticket_instances ti ON ti.asset_id = a.id
            JOIN wallets w ON w.id = ti.wallet_id
            LEFT JOIN user_genres ug ON ug.genre_id = eg.genre_id AND ug.user_id = w.user_id
            WHERE w.user_id = $1
            AND ug.id IS NULL;
        "#;
        diesel::sql_query(query)
            .bind::<dUuid, _>(self.id)
            .execute(conn)
            .to_db_error(ErrorCode::QueryError, "Could not set genres on user")?;

        let query = r#"
            DELETE FROM user_genres
            WHERE NOT genre_id = ANY(
                SELECT DISTINCT eg.genre_id
                FROM event_genres eg
                JOIN ticket_types tt ON tt.event_id = eg.event_id
                JOIN assets a ON a.ticket_type_id = tt.id
                JOIN ticket_instances ti ON ti.asset_id = a.id
                JOIN wallets w ON w.id = ti.wallet_id
                WHERE w.user_id = $1
            ) AND user_id = $1;
        "#;
        diesel::sql_query(query)
            .bind::<dUuid, _>(self.id)
            .execute(conn)
            .to_db_error(ErrorCode::QueryError, "Could not clear old genres on user")?;

        Ok(())
    }

    pub(crate) fn update_genre_info_for_associated_event_users(
        event_id: Uuid,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        let query = r#"
            INSERT INTO user_genres (user_id, genre_id)
            SELECT DISTINCT w.user_id, eg.genre_id
            FROM event_genres eg
            JOIN ticket_types tt ON tt.event_id = eg.event_id
            JOIN assets a ON a.ticket_type_id = tt.id
            JOIN ticket_instances ti ON ti.asset_id = a.id
            JOIN wallets w ON w.id = ti.wallet_id
            LEFT JOIN user_genres ug ON ug.genre_id = eg.genre_id AND ug.user_id = w.user_id
            WHERE eg.event_id = $1
            AND ti.status IN ('Purchased', 'Redeemed')
            AND ug.id IS NULL;
        "#;

        diesel::sql_query(query)
            .bind::<dUuid, _>(event_id)
            .execute(conn)
            .to_db_error(ErrorCode::QueryError, "Could not set genres on users")?;

        let query = r#"
            DELETE FROM user_genres
            WHERE id = ANY(
                SELECT DISTINCT ug.id
                FROM user_genres ug
                WHERE ug.user_id = ANY (
                    SELECT w.user_id from wallets w
                    JOIN ticket_instances ti on ti.wallet_id = w.id
                    JOIN assets a ON ti.asset_id = a.id
                    JOIN ticket_types tt ON tt.id = a.ticket_type_id
                    WHERE tt.event_id = $1
                    AND ti.status IN ('Purchased', 'Redeemed')
                )
                AND NOT ug.genre_id = ANY (
                    SELECT DISTINCT eg.genre_id
                    FROM event_genres eg
                    JOIN ticket_types tt ON tt.event_id = eg.event_id
                    JOIN assets a ON a.ticket_type_id = tt.id
                    JOIN ticket_instances ti ON ti.asset_id = a.id
                    JOIN wallets w ON w.id = ti.wallet_id
                    WHERE w.user_id = ug.user_id
                )
            );
        "#;

        diesel::sql_query(query)
            .bind::<dUuid, _>(event_id)
            .execute(conn)
            .to_db_error(ErrorCode::QueryError, "Could not clear old genres on user")?;

        Ok(())
    }

    pub fn create(
        first_name: Option<String>,
        last_name: Option<String>,
        email: Option<String>,
        phone: Option<String>,
        password: &str,
    ) -> NewUser {
        let hash = PasswordHash::generate(password, None);
        let lower_email = email.clone().map(|e| e.to_lowercase());
        NewUser {
            first_name: first_name.clone(),
            last_name: last_name.clone(),
            email: lower_email,
            phone: phone.clone(),
            hashed_pw: hash.to_string(),
            role: vec![Roles::User],
        }
    }

    pub fn new_stub(
        first_name: Option<String>,
        last_name: Option<String>,
        email: Option<String>,
        phone: Option<String>,
    ) -> NewUser {
        let rand_password = random_alpha_string(16);
        Self::create(first_name, last_name, email, phone, rand_password.as_str())
    }

    pub fn create_from_external_login(
        external_user_id: String,
        first_name: String,
        last_name: String,
        email: Option<String>,
        site: String,
        access_token: String,
        scopes: Vec<String>,
        current_user_id: Option<Uuid>,
        conn: &PgConnection,
    ) -> Result<User, DatabaseError> {
        let rand_password = random_alpha_string(16);
        let hash = PasswordHash::generate(rand_password.as_str(), None);
        let lower_email = email.map(|s| s.to_lowercase());
        let new_user = NewUser {
            first_name: Some(first_name),
            last_name: Some(last_name),
            email: lower_email,
            phone: None,
            hashed_pw: hash.to_string(),
            role: vec![Roles::User],
        };
        new_user.commit(current_user_id, conn).and_then(|user| {
            user.add_external_login(current_user_id, external_user_id, site, access_token, scopes, conn)?;
            Ok(user)
        })
    }

    pub fn login_domain_event(&self, json: Value, conn: &PgConnection) -> Result<(), DatabaseError> {
        DomainEvent::create(
            DomainEventTypes::UserLogin,
            "User login".to_string(),
            Tables::Users,
            Some(self.id),
            Some(self.id),
            Some(json),
        )
        .commit(conn)?;
        Ok(())
    }

    pub fn create_stub(
        first_name: String,
        last_name: String,
        email: Option<String>,
        phone: Option<String>,
        current_user_id: Option<Uuid>,
        conn: &PgConnection,
    ) -> Result<User, DatabaseError> {
        Self::new_stub(Some(first_name), Some(last_name), email, phone).commit(current_user_id, conn)
    }

    pub fn transfer_activity_by_event_tickets(
        &self,
        page: u32,
        limit: u32,
        sort_direction: SortingDir,
        past_or_upcoming: PastOrUpcoming,
        conn: &PgConnection,
    ) -> Result<Payload<UserTransferActivitySummary>, DatabaseError> {
        use schema::*;
        let (start_time, end_time) = Event::dates_by_past_or_upcoming(None, None, past_or_upcoming);

        let (events, total): (Vec<Event>, i64) = transfers::table
            .inner_join(transfer_tickets::table.on(transfer_tickets::transfer_id.eq(transfers::id)))
            .inner_join(ticket_instances::table.on(transfer_tickets::ticket_instance_id.eq(ticket_instances::id)))
            .inner_join(assets::table.on(ticket_instances::asset_id.eq(assets::id)))
            .inner_join(ticket_types::table.on(assets::ticket_type_id.eq(ticket_types::id)))
            .inner_join(events::table.on(ticket_types::event_id.eq(events::id)))
            .filter(transfers::source_user_id.eq(self.id))
            .filter(events::event_end.ge(start_time))
            .filter(events::event_end.le(end_time))
            .select(events::all_columns)
            .distinct()
            .order_by(sql::<()>(&format!("events.event_start {}", sort_direction)))
            .paginate(page as i64)
            .per_page(limit as i64)
            .load_and_count_pages(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load transfer events for user")?;

        let mut result: Vec<UserTransferActivitySummary> = Vec::new();
        for event in events {
            let mut ticket_activity_items: HashMap<Uuid, Vec<ActivityItem>> = HashMap::new();
            let activity_items = ActivityItem::load_transfers(None, Some(event.id), Some(self.id), true, conn)?;

            // For each activity item, associate with tickets associated
            for activity_item in activity_items {
                if let ActivityItem::Transfer { ref ticket_ids, .. } = activity_item {
                    for ticket_id in ticket_ids {
                        ticket_activity_items
                            .entry(*ticket_id)
                            .or_insert(Vec::new())
                            .push(activity_item.clone())
                    }
                }
            }

            // Only retain activity where the transfer count of initiated is greater than that of the cancelled
            ticket_activity_items.retain(|_, ai| {
                ai.iter()
                    .filter(|a| {
                        if let ActivityItem::Transfer { action, .. } = a {
                            action.as_str() == "Started"
                        } else {
                            false
                        }
                    })
                    .count()
                    > ai.iter()
                        .filter(|a| {
                            if let ActivityItem::Transfer { action, .. } = a {
                                action.as_str() == "Cancelled"
                            } else {
                                false
                            }
                        })
                        .count()
            });
            ticket_activity_items.retain(|_, ai| ai.len() > 0);
            if ticket_activity_items.len() > 0 {
                result.push(UserTransferActivitySummary {
                    ticket_activity_items,
                    event: event.for_display(conn)?,
                });
            }
        }

        let mut payload = Payload::new(result, Paging::new(page, limit));

        payload.paging.total = total as u64;
        payload.paging.dir = sort_direction;
        Ok(payload)
    }

    pub fn activity(
        &self,
        organization: &Organization,
        page: u32,
        limit: u32,
        sort_direction: SortingDir,
        past_or_upcoming: PastOrUpcoming,
        activity_type: Option<ActivityType>,
        conn: &PgConnection,
    ) -> Result<Payload<ActivitySummary>, DatabaseError> {
        let (start_time, end_time) = Event::dates_by_past_or_upcoming(None, None, past_or_upcoming);

        #[derive(Queryable, QueryableByName)]
        struct R {
            #[sql_type = "dUuid"]
            id: Uuid,
            #[sql_type = "Text"]
            name: String,
            #[sql_type = "dUuid"]
            organization_id: Uuid,
            #[sql_type = "Nullable<dUuid>"]
            venue_id: Option<Uuid>,
            #[sql_type = "Timestamp"]
            created_at: NaiveDateTime,
            #[sql_type = "Nullable<Timestamp>"]
            event_start: Option<NaiveDateTime>,
            #[sql_type = "Nullable<Timestamp>"]
            door_time: Option<NaiveDateTime>,
            #[sql_type = "Text"]
            status: EventStatus,
            #[sql_type = "Nullable<Timestamp>"]
            publish_date: Option<NaiveDateTime>,
            #[sql_type = "Nullable<Timestamp>"]
            redeem_date: Option<NaiveDateTime>,
            #[sql_type = "Nullable<Text>"]
            promo_image_url: Option<String>,
            #[sql_type = "Nullable<Text>"]
            additional_info: Option<String>,
            #[sql_type = "Nullable<Text>"]
            age_limit: Option<String>,
            #[sql_type = "Nullable<Text>"]
            top_line_info: Option<String>,
            #[sql_type = "Nullable<Timestamp>"]
            cancelled_at: Option<NaiveDateTime>,
            #[sql_type = "Timestamp"]
            updated_at: NaiveDateTime,
            #[sql_type = "Nullable<Text>"]
            video_url: Option<String>,
            #[sql_type = "Bool"]
            is_external: bool,
            #[sql_type = "Nullable<Text>"]
            external_url: Option<String>,
            #[sql_type = "Nullable<Text>"]
            override_status: Option<EventOverrideStatus>,
            #[sql_type = "Nullable<BigInt>"]
            client_fee_in_cents: Option<i64>,
            #[sql_type = "Nullable<BigInt>"]
            company_fee_in_cents: Option<i64>,
            #[sql_type = "Nullable<BigInt>"]
            settlement_amount_in_cents: Option<i64>,
            #[sql_type = "Nullable<Timestamp>"]
            event_end: Option<NaiveDateTime>,
            #[sql_type = "Nullable<BigInt>"]
            sendgrid_list_id: Option<i64>,
            #[sql_type = "Text"]
            event_type: EventTypes,
            #[sql_type = "Nullable<Text>"]
            cover_image_url: Option<String>,
            #[sql_type = "Nullable<Text>"]
            private_access_code: Option<String>,
            #[sql_type = "Nullable<Text>"]
            facebook_pixel_key: Option<String>,
            #[sql_type = "Nullable<Timestamp>"]
            deleted_at: Option<NaiveDateTime>,
            #[sql_type = "Nullable<Jsonb>"]
            extra_admin_data: Option<Value>,
            #[sql_type = "dUuid"]
            slug_id: Uuid,
            #[sql_type = "Nullable<Timestamp>"]
            settled_at: Option<NaiveDateTime>,
            #[sql_type = "BigInt"]
            total: i64,
            #[sql_type = "Nullable<Text>"]
            facebook_event_id: Option<String>,
            #[sql_type = "Nullable<dUuid>"]
            cloned_from_event_id: Option<Uuid>,
        }

        let mut query = sql_query(
            r#"
        SELECT DISTINCT
            e.*, COUNT(*) OVER () as total
        FROM users u
        LEFT JOIN orders o ON COALESCE(o.on_behalf_of_user_id, o.user_id) = u.id
        LEFT JOIN order_items oi ON o.id = oi.order_id
        LEFT JOIN transfers t ON t.destination_user_id = u.id OR t.source_user_id = u.id
        LEFT JOIN transfer_tickets tt ON tt.transfer_id = t.id
        LEFT JOIN wallets w ON w.user_id = u.id
        LEFT JOIN ticket_instances ti ON w.id = ti.wallet_id OR tt.ticket_instance_id = ti.id
        LEFT JOIN assets a ON a.id = ti.asset_id
        LEFT JOIN ticket_types tt2 ON tt2.id = a.ticket_type_id
        JOIN events e ON oi.event_id = e.id OR tt2.event_id = e.id
        WHERE (o.status = 'Paid' OR o.status is NULL)
        "#,
        )
        .into_boxed();

        let mut bind_no = 1;
        query = query
            .sql(format!(" AND u.id = ${} ", bind_no))
            .bind::<dUuid, _>(self.id);

        bind_no += 1;
        query = query
            .sql(format!(" AND e.organization_id = ${} ", bind_no))
            .bind::<dUuid, _>(organization.id);

        bind_no += 1;
        query = query
            .sql(format!(" AND e.event_end >= ${} ", bind_no))
            .bind::<Timestamp, _>(start_time);

        bind_no += 1;
        query = query
            .sql(format!(" AND e.event_end <= ${} ", bind_no))
            .bind::<Timestamp, _>(end_time);

        query = query.sql(format!(" ORDER BY e.event_end {}", sort_direction));

        let results: Vec<R> = query
            .sql(format!(" LIMIT ${} OFFSET ${} ", bind_no + 1, bind_no + 2))
            .bind::<BigInt, _>(limit as i64)
            .bind::<BigInt, _>(page as i64 * limit as i64)
            .get_results(conn)
            .to_db_error(
                ErrorCode::QueryError,
                "Unable to load purchase event data for organization fan",
            )?;

        let total = results.get(0).map(|s| s.total).unwrap_or(0);
        let events = results.into_iter().map(|event| Event {
            id: event.id,
            name: event.name,
            organization_id: event.organization_id,
            venue_id: event.venue_id,
            created_at: event.created_at,
            event_start: event.event_start,
            door_time: event.door_time,
            status: event.status,
            publish_date: event.publish_date,
            redeem_date: event.redeem_date,
            promo_image_url: event.promo_image_url,
            additional_info: event.additional_info,
            age_limit: event.age_limit,
            top_line_info: event.top_line_info,
            cancelled_at: event.cancelled_at,
            updated_at: event.updated_at,
            video_url: event.video_url,
            is_external: event.is_external,
            external_url: event.external_url,
            override_status: event.override_status,
            client_fee_in_cents: event.client_fee_in_cents,
            company_fee_in_cents: event.company_fee_in_cents,
            settlement_amount_in_cents: event.settlement_amount_in_cents,
            event_end: event.event_end,
            sendgrid_list_id: event.sendgrid_list_id,
            event_type: event.event_type,
            cover_image_url: event.cover_image_url,
            private_access_code: event.private_access_code,
            facebook_pixel_key: event.facebook_pixel_key,
            deleted_at: event.deleted_at,
            extra_admin_data: event.extra_admin_data,
            settled_at: event.settled_at,
            slug_id: Some(event.slug_id),
            facebook_event_id: event.facebook_event_id,
            cloned_from_event_id: event.cloned_from_event_id,
        });

        let mut result: Vec<ActivitySummary> = Vec::new();
        for event in events {
            let summary = event.activity_summary(self.id, activity_type, conn)?;
            if summary.activity_items.len() > 0 {
                result.push(summary);
            }
        }

        let mut payload = Payload::new(result, Paging::new(page, limit));
        payload.paging.total = total as u64;
        payload.paging.dir = sort_direction;
        Ok(payload)
    }

    pub fn get_history_for_organization(
        &self,
        organization: &Organization,
        page: u32,
        limit: u32,
        sort_direction: SortingDir,
        conn: &PgConnection,
    ) -> Result<Payload<HistoryItem>, DatabaseError> {
        use schema::*;
        let query = order_items::table
            .inner_join(orders::table.on(order_items::order_id.eq(orders::id)))
            .inner_join(events::table.on(order_items::event_id.eq(events::id.nullable())))
            .filter(orders::status.eq(OrderStatus::Paid))
            .filter(
                orders::on_behalf_of_user_id.eq(Some(self.id))
                    .or(orders::on_behalf_of_user_id
                        .is_null()
                        .and(orders::user_id.eq(self.id))
                    )
            )
            .filter(events::organization_id.eq(organization.id))
            .group_by((orders::id, orders::order_date, events::name))
            .select((
                orders::id,
                orders::order_date,
                events::name,
                sql::<BigInt>(
                    "cast(COALESCE(sum(
                    CASE WHEN order_items.item_type = 'Tickets'
                    THEN (order_items.quantity - order_items.refunded_quantity)
                    ELSE 0 END
                    ), 0) as BigInt)",
                ),
                sql::<BigInt>(
                    "cast(sum(order_items.unit_price_in_cents * (order_items.quantity - order_items.refunded_quantity)) as bigint)",
                ),
                sql::<BigInt>("count(*) over()"),
            ))
            .order_by(sql::<()>(&format!("orders.order_date {}", sort_direction)))
            .limit(limit as i64)
            .offset((limit * page) as i64);

        #[derive(Queryable)]
        struct R {
            order_id: Uuid,
            order_date: NaiveDateTime,
            event_name: String,
            ticket_sales: i64,
            revenue_in_cents: i64,
            total_rows: i64,
        }
        let results: Vec<R> = query
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load history for organization fan")?;

        let paging = Paging::new(page, limit);
        let mut total: u64 = 0;
        if !results.is_empty() {
            total = results[0].total_rows as u64;
        }

        let history = results
            .into_iter()
            .map(|r| HistoryItem::Purchase {
                order_id: r.order_id,
                order_date: r.order_date,
                event_name: r.event_name,
                ticket_sales: r.ticket_sales as u32,
                revenue_in_cents: r.revenue_in_cents as u32,
            })
            .collect();

        let mut payload = Payload::new(history, paging);
        payload.paging.total = total;
        payload.paging.dir = sort_direction;
        Ok(payload)
    }

    pub fn get_profile_for_organization(
        &self,
        organization: &Organization,
        conn: &PgConnection,
    ) -> Result<FanProfile, DatabaseError> {
        let query = sql_query(
            "SELECT CAST(COALESCE((
                    SELECT SUM(oi.quantity - oi.refunded_quantity)
                    FROM orders o
                    JOIN order_items oi ON o.id = oi.order_id
                    JOIN events e on oi.event_id = e.id
                    WHERE COALESCE(o.on_behalf_of_user_id, o.user_id) = $1
                    AND e.organization_id = $2
                    AND o.status = 'Paid'
                    AND oi.item_type = 'Tickets'
                ), 0) as BigInt) as ticket_sales,
                    CAST(COALESCE((
                    SELECT COUNT(ti.id)
                    FROM ticket_instances ti
                    JOIN wallets w ON w.id = ti.wallet_id
                    JOIN assets a on ti.asset_id = a.id
                    JOIN ticket_types tt on tt.id = a.ticket_type_id
                    JOIN events e ON e.id = tt.event_id
                    WHERE w.user_id = $1
                    AND e.organization_id = $2
                    AND ti.status in ('Purchased', 'Redeemed')
                ), 0) as BigInt) as tickets_owned,
                 CAST(COALESCE((
                    SELECT SUM(oi.unit_price_in_cents * (oi.quantity - oi.refunded_quantity))
                    FROM order_items oi
                    JOIN orders o ON o.id = oi.order_id
                    JOIN events e ON e.id = oi.event_id
                    WHERE COALESCE(o.on_behalf_of_user_id, o.user_id) = $1
                    AND e.organization_id = $2
                    AND o.status = 'Paid'
                ), 0) as BigInt) as revenue_in_cents",
        )
        .bind::<dUuid, _>(self.id)
        .bind::<dUuid, _>(organization.id);

        #[derive(QueryableByName)]
        struct R {
            #[sql_type = "BigInt"]
            ticket_sales: i64,
            #[sql_type = "BigInt"]
            tickets_owned: i64,
            #[sql_type = "BigInt"]
            revenue_in_cents: i64,
        }
        let mut result: Vec<R> = query
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load profile for organization fan")?;

        let result = result.remove(0);
        if result.ticket_sales == 0 && result.tickets_owned == 0 && result.revenue_in_cents == 0 {
            return DatabaseError::no_results("Could not load profile for organization fan, NotFound");
        }
        Ok(FanProfile {
            first_name: self.first_name.clone(),
            last_name: self.last_name.clone(),
            email: self.email.clone(),
            facebook_linked: self.find_external_login(FACEBOOK_SITE, conn).optional()?.is_some(),
            revenue_in_cents: result.revenue_in_cents as u32,
            ticket_sales: result.ticket_sales as u32,
            tickets_owned: result.tickets_owned as u32,
            profile_pic_url: self.profile_pic_url.clone(),
            thumb_profile_pic_url: self.thumb_profile_pic_url.clone(),
            cover_photo_url: self.cover_photo_url.clone(),
            created_at: self.created_at,
            attendance_information: self.attendance_information(conn)?,
            deleted_at: self.deleted_at,
        })
    }

    pub fn attendance_information(&self, conn: &PgConnection) -> Result<Vec<AttendanceInformation>, DatabaseError> {
        use schema::*;
        ticket_instances::table
            .inner_join(assets::table.on(ticket_instances::asset_id.eq(assets::id)))
            .inner_join(wallets::table.on(ticket_instances::wallet_id.eq(wallets::id)))
            .inner_join(ticket_types::table.on(assets::ticket_type_id.eq(ticket_types::id)))
            .inner_join(events::table.on(ticket_types::event_id.eq(events::id)))
            .filter(ticket_instances::status.eq(TicketInstanceStatus::Redeemed))
            .filter(wallets::user_id.eq(self.id))
            .order_by(events::event_start)
            .select((
                ticket_types::event_id,
                sql::<Text>("events.name as event_name"),
                events::event_start,
            ))
            .distinct()
            .get_results(conn)
            .to_db_error(
                ErrorCode::QueryError,
                "Could not load attendance info for organization fan",
            )
    }

    pub fn find(id: Uuid, conn: &PgConnection) -> Result<User, DatabaseError> {
        DatabaseError::wrap(
            ErrorCode::QueryError,
            "Error loading user",
            users::table.find(id).first::<User>(conn),
        )
    }

    pub fn find_by_ids(user_ids: &Vec<Uuid>, conn: &PgConnection) -> Result<Vec<User>, DatabaseError> {
        users::table
            .filter(users::id.eq_any(user_ids))
            .select(users::all_columns)
            .distinct()
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load users")
    }

    pub fn find_by_email(email: &str, include_deleted: bool, conn: &PgConnection) -> Result<User, DatabaseError> {
        let lower_email = email.trim().to_lowercase();
        let mut query = users::table.filter(users::email.eq(lower_email)).into_boxed();
        if !include_deleted {
            query = query.filter(users::deleted_at.is_null())
        }

        DatabaseError::wrap(ErrorCode::QueryError, "Error loading user", query.first::<User>(conn))
    }

    pub fn find_by_phone(phone: &str, include_deleted: bool, conn: &PgConnection) -> Result<User, DatabaseError> {
        let mut query = users::table.filter(users::phone.eq(phone.trim())).into_boxed();
        if !include_deleted {
            query = query.filter(users::deleted_at.is_null());
        }
        DatabaseError::wrap(ErrorCode::QueryError, "Error loading user", query.first::<User>(conn))
    }

    pub fn create_magic_link_token(
        &self,
        token_issuer: &dyn TokenIssuer,
        expiry: Duration,
        fail_on_error: bool,
        conn: &PgConnection,
    ) -> Result<Option<String>, DatabaseError> {
        if self.role.contains(&Roles::Admin) || self.role.contains(&Roles::Super) {
            return if fail_on_error {
                DatabaseError::business_process_error(
                    "Cannot create a magic link for users who have admin or superadmin roles",
                )
            } else {
                Ok(None)
            };
        }

        if self.organizations(conn)?.len() > 0 {
            return if fail_on_error {
                DatabaseError::business_process_error(
                    "Cannot create a magic link for \
                     users who have organization access",
                )
            } else {
                Ok(None)
            };
        }

        Ok(Some(token_issuer.issue_with_limited_scopes(
            self.id,
            vec![Scopes::TokenRefresh],
            expiry,
        )?))
    }

    fn email_unique(
        id: Uuid,
        email: String,
        conn: &PgConnection,
    ) -> Result<Result<(), ValidationError>, DatabaseError> {
        let email_in_use = select(exists(
            users::table
                .filter(users::id.ne(id))
                .filter(users::email.eq(email.trim().to_lowercase())),
        ))
        .get_result(conn)
        .to_db_error(ErrorCode::QueryError, "Could not check if user email is unique")?;

        if email_in_use {
            let validation_error = create_validation_error("uniqueness", "Email is already in use");
            return Ok(Err(validation_error));
        }

        Ok(Ok(()))
    }

    fn validate_record(&self, update_attrs: &UserEditableAttributes, conn: &PgConnection) -> Result<(), DatabaseError> {
        let mut validation_errors = update_attrs.validate();

        if let Some(ref email) = update_attrs.email {
            validation_errors = validators::append_validation_error(
                validation_errors,
                "email",
                User::email_unique(self.id, email.to_string(), conn)?,
            );
        }

        Ok(validation_errors?)
    }

    pub fn update(
        &self,
        attributes: UserEditableAttributes,
        current_user_id: Option<Uuid>,
        conn: &PgConnection,
    ) -> Result<User, DatabaseError> {
        let mut lower_cased_attributes = attributes;
        lower_cased_attributes.email = lower_cased_attributes.email.map(|e| e.to_lowercase());
        self.validate_record(&lower_cased_attributes, conn)?;

        let query = diesel::update(self).set((&lower_cased_attributes, users::updated_at.eq(dsl::now)));

        let result = DatabaseError::wrap(ErrorCode::UpdateError, "Error updating user", query.get_result(conn))?;

        DomainEvent::create(
            DomainEventTypes::UserUpdated,
            "User was updated".to_string(),
            Tables::Users,
            Some(self.id),
            current_user_id,
            Some(json!(&lower_cased_attributes)),
        )
        .commit(conn)?;

        Ok(result)
    }

    pub fn check_password(&self, password: &str) -> bool {
        let hash = match PasswordHash::from_str(&self.hashed_pw) {
            Ok(h) => h,
            Err(_) => return false,
        };
        hash.verify(password)
    }

    pub fn add_role(&self, r: Roles, conn: &PgConnection) -> Result<User, DatabaseError> {
        let mut new_roles = self.role.clone();
        if !new_roles.contains(&r) {
            new_roles.push(r);
        }

        self.update_role(new_roles, conn)
    }

    pub fn remove_role(&self, r: Roles, conn: &PgConnection) -> Result<User, DatabaseError> {
        let mut current_roles = self.role.clone();

        current_roles.retain(|x| x != &r);

        self.update_role(current_roles, conn)
    }

    pub fn has_role(&self, role: Roles) -> bool {
        self.role.contains(&role)
    }

    pub fn is_admin(&self) -> bool {
        self.has_role(Roles::Admin) || self.has_role(Roles::Super)
    }

    pub fn get_global_scopes(&self) -> Vec<Scopes> {
        scopes::get_scopes(self.role.clone(), None)
    }

    pub fn event_users(&self, conn: &PgConnection) -> Result<Vec<EventUser>, DatabaseError> {
        event_users::table
            .filter(event_users::user_id.eq(self.id))
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Could not retrieve event users")
    }

    pub fn get_event_ids_for_organization(
        &self,
        organization_id: Uuid,
        conn: &PgConnection,
    ) -> Result<Vec<Uuid>, DatabaseError> {
        event_users::table
            .inner_join(events::table.on(events::id.eq(event_users::event_id)))
            .filter(event_users::user_id.eq(self.id))
            .filter(events::organization_id.eq(organization_id))
            .select(event_users::event_id)
            .load(conn)
            .to_db_error(
                ErrorCode::QueryError,
                "Could not retrieve event ids for organization user",
            )
    }

    pub fn get_event_ids_by_organization(
        &self,
        conn: &PgConnection,
    ) -> Result<(HashMap<Uuid, Vec<Uuid>>, HashMap<Uuid, Vec<Uuid>>), DatabaseError> {
        let mut events_by_organization = HashMap::new();
        let mut readonly_events_by_organization = HashMap::new();

        let organization_event_mapping = event_users::table
            .inner_join(events::table.on(event_users::event_id.eq(events::id)))
            .filter(event_users::user_id.eq(self.id))
            .group_by(events::organization_id)
            .select((
                events::organization_id,
                sql::<Array<dUuid>>("COALESCE(ARRAY_AGG(DISTINCT event_users.event_id) FILTER(WHERE event_users.role = 'Promoter'), '{}')"),
                sql::<Array<dUuid>>("COALESCE(ARRAY_AGG(DISTINCT event_users.event_id) FILTER(WHERE event_users.role = 'PromoterReadOnly'), '{}')"),
            ))
            .load::<(Uuid, Vec<Uuid>, Vec<Uuid>)>(conn)
            .to_db_error(
                ErrorCode::QueryError,
                "Could not retrieve organization info for user",
            )?;

        for (organization_id, event_ids, readonly_event_ids) in organization_event_mapping {
            events_by_organization.insert(organization_id, event_ids);
            readonly_events_by_organization.insert(organization_id, readonly_event_ids);
        }

        Ok((events_by_organization, readonly_events_by_organization))
    }

    pub fn get_roles_by_organization(
        &self,
        conn: &PgConnection,
    ) -> Result<HashMap<Uuid, (Vec<Roles>, Option<AdditionalOrgMemberScopes>)>, DatabaseError> {
        let mut roles_by_organization = HashMap::new();
        for organization in self.organizations(conn)? {
            roles_by_organization.insert(organization.id.clone(), organization.get_roles_for_user(self, conn)?);
        }
        Ok(roles_by_organization)
    }

    pub fn get_scopes_by_organization(&self, conn: &PgConnection) -> Result<HashMap<Uuid, Vec<Scopes>>, DatabaseError> {
        let mut scopes_by_organization = HashMap::new();
        for organization in self.organizations(conn)? {
            scopes_by_organization.insert(organization.id, organization.get_scopes_for_user(self, conn)?);
        }

        Ok(scopes_by_organization)
    }

    pub fn organizations(&self, conn: &PgConnection) -> Result<Vec<Organization>, DatabaseError> {
        if self.is_admin() {
            organizations::table
                .order_by(organizations::name.asc())
                .load::<Organization>(conn)
                .to_db_error(ErrorCode::QueryError, "Could not retrieve organizations for user")
        } else {
            organizations::table
                .left_join(organization_users::table)
                .filter(organization_users::user_id.eq(self.id))
                .select(organizations::all_columns)
                .order_by(organizations::name.asc())
                .load::<Organization>(conn)
                .to_db_error(ErrorCode::QueryError, "Could not retrieve organizations for user")
        }
    }

    pub fn payment_methods(&self, conn: &PgConnection) -> Result<Vec<PaymentMethod>, DatabaseError> {
        PaymentMethod::find_for_user(self.id, None, conn)
    }

    pub fn default_payment_method(&self, conn: &PgConnection) -> Result<PaymentMethod, DatabaseError> {
        PaymentMethod::find_default_for_user(self.id, conn)
    }

    pub fn payment_method(&self, name: PaymentProviders, conn: &PgConnection) -> Result<PaymentMethod, DatabaseError> {
        let mut payment_methods = PaymentMethod::find_for_user(self.id, Some(name), conn)?;
        if payment_methods.is_empty() {
            Err(DatabaseError::new(
                ErrorCode::NoResults,
                Some("No payment method found for user".to_string()),
            ))
        } else {
            Ok(payment_methods.remove(0))
        }
    }

    fn update_role(&self, new_roles: Vec<Roles>, conn: &PgConnection) -> Result<User, DatabaseError> {
        DatabaseError::wrap(
            ErrorCode::UpdateError,
            "Could not update role for user",
            diesel::update(self)
                .set((users::role.eq(new_roles), users::updated_at.eq(dsl::now)))
                .get_result(conn),
        )
    }

    pub fn find_events_with_access_to_scan(&self, conn: &PgConnection) -> Result<Vec<Event>, DatabaseError> {
        let one_day_ago = NaiveDateTime::from(Utc::now().naive_utc() - Duration::days(1));
        //Find all events that have their end_date that is >= 24 hours ago.
        let one_day_forward = NaiveDateTime::from(Utc::now().naive_utc() + Duration::days(1));
        //And we are at least one day away from the door_time
        let events_query = events::table
            .filter(events::status.eq(EventStatus::Published))
            .filter(events::is_external.eq(false))
            //Check that the event hasn't ended already (with some buffer)
            .filter(events::event_end.ge(one_day_ago))
            //Check that we are not before the start of the event (with some buffer)
            .filter(events::door_time.le(one_day_forward))
            .order_by(events::event_start.asc())
            .into_boxed();

        let result = if self.is_admin() {
            events_query.load(conn)
        } else {
            let user_organizations = self.get_scopes_by_organization(conn)?;
            let user_organization_ids: Vec<Uuid> = user_organizations
                .into_iter()
                .filter(|org| org.1.contains(&Scopes::EventScan))
                .map(|i| i.0)
                .collect();

            events_query
                .filter(events::organization_id.eq_any(user_organization_ids))
                .select(events::all_columns)
                .load(conn)
        };
        result.to_db_error(ErrorCode::QueryError, "Error loading scannable events")
    }

    pub fn full_name(&self) -> String {
        vec![
            self.first_name.clone().unwrap_or("".to_string()),
            self.last_name.clone().unwrap_or("".to_string()),
        ]
        .join(" ")
    }

    pub fn find_external_login(&self, site: &str, conn: &PgConnection) -> Result<ExternalLogin, DatabaseError> {
        ExternalLogin::find_for_site(self.id, site, conn)
    }

    pub fn external_logins(&self, conn: &PgConnection) -> Result<Vec<ExternalLogin>, DatabaseError> {
        ExternalLogin::find_all_for_user(self.id, conn)
    }

    pub fn add_external_login(
        &self,
        current_user_id: Option<Uuid>,
        external_user_id: String,
        site: String,
        access_token: String,
        scopes: Vec<String>,
        conn: &PgConnection,
    ) -> Result<ExternalLogin, DatabaseError> {
        ExternalLogin::create(external_user_id, site, self.id, access_token, scopes).commit(current_user_id, conn)
    }

    pub fn add_or_replace_external_login(
        &self,
        current_user_id: Option<Uuid>,
        external_user_id: String,
        site: String,
        access_token: String,
        scopes: Vec<String>,
        conn: &PgConnection,
    ) -> Result<ExternalLogin, DatabaseError> {
        let external_login = ExternalLogin::find_user(&external_user_id, &site, conn)?;
        if let Some(login) = external_login {
            login.delete(current_user_id, conn)?;
        };
        ExternalLogin::create(external_user_id, site, self.id, access_token, scopes).commit(current_user_id, conn)
    }

    pub fn wallets(&self, conn: &PgConnection) -> Result<Vec<Wallet>, DatabaseError> {
        Wallet::find_for_user(self.id, conn)
    }

    pub fn default_wallet(&self, conn: &PgConnection) -> Result<Wallet, DatabaseError> {
        Wallet::find_default_for_user(self.id, conn)
    }

    pub fn update_last_cart(&self, new_cart_id: Option<Uuid>, conn: &PgConnection) -> Result<(), DatabaseError> {
        // diesel does not have any easy way of handling "last_cart_id is null OR last_cart_id = 'x'"
        let query = if self.last_cart_id.is_none() {
            diesel::update(
                users::table
                    .filter(users::id.eq(self.id))
                    .filter(users::updated_at.eq(self.updated_at))
                    .filter(users::last_cart_id.is_null()),
            )
            .into_boxed()
        } else {
            diesel::update(
                users::table
                    .filter(users::id.eq(self.id))
                    .filter(users::updated_at.eq(self.updated_at))
                    .filter(users::last_cart_id.eq(self.last_cart_id)),
            )
            .into_boxed()
        };
        let rows_affected = query
            .set((users::last_cart_id.eq(new_cart_id), users::updated_at.eq(dsl::now)))
            .execute(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not update last cart on user")?;

        match rows_affected {
            1 => Ok(()),

            _ => DatabaseError::concurrency_error(
                "Could not update last cart on user because the row has been changed by another source",
            ),
        }
    }

    pub fn push_notification_tokens(&self, conn: &PgConnection) -> Result<Vec<PushNotificationToken>, DatabaseError> {
        PushNotificationToken::find_by_user_id(self.id, conn)
    }

    pub fn disable(self, current_user: Option<&User>, conn: &PgConnection) -> Result<Self, DatabaseError> {
        let result: User = diesel::update(&self)
            .set((users::deleted_at.eq(dsl::now), users::updated_at.eq(dsl::now)))
            .get_result(conn)
            .to_db_error(ErrorCode::DeleteError, "Could not delete user")?;

        DomainEvent::create(
            DomainEventTypes::UserDisabled,
            "User account deleted".to_string(),
            Tables::Users,
            Some(result.id),
            current_user.map(|u| u.id),
            None,
        )
        .commit(conn)?;

        for push_notification_token in self.push_notification_tokens(conn)? {
            PushNotificationToken::remove(self.id, push_notification_token.id, conn)?;
        }

        for external_login in self.external_logins(conn)? {
            external_login.delete(current_user.map(|u| u.id), conn)?
        }

        Ok(result)
    }
}

impl From<User> for DisplayUser {
    fn from(user: User) -> Self {
        DisplayUser {
            id: user.id,
            first_name: user.first_name,
            last_name: user.last_name,
            email: user.email,
            phone: user.phone,
            profile_pic_url: user.profile_pic_url,
            thumb_profile_pic_url: user.thumb_profile_pic_url,
            cover_photo_url: user.cover_photo_url,
            is_org_owner: false,
        }
    }
}

impl ForDisplay<DisplayUser> for User {
    fn for_display(self) -> Result<DisplayUser, DatabaseError> {
        Ok(self.into())
    }
}
