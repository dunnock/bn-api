use chrono::{Datelike, Duration, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;
use diesel;
use diesel::dsl::{exists, select};
use diesel::expression::dsl;
use diesel::expression::sql_literal::sql;
use diesel::prelude::*;
use diesel::sql_types::{Array, BigInt, Nullable, Text, Timestamp, Uuid as dUuid};
use models::scopes;
use models::*;
use schema::{
    assets, event_users, events, fee_schedules, order_items, orders, organization_users, organizations, ticket_types,
    users,
};
use std::cmp;
use std::collections::HashMap;
use utils::encryption::*;
use utils::errors::*;
use utils::pagination::Paginate;
use utils::text;
use uuid::Uuid;

const DEFAULT_SETTLEMENT_TIMEZONE: &str = "America/Los_Angeles";

#[derive(
    Identifiable, Associations, Queryable, QueryableByName, AsChangeset, Clone, Serialize, Deserialize, PartialEq, Debug,
)]
#[table_name = "organizations"]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
    pub address: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
    pub postal_code: Option<String>,
    pub phone: Option<String>,
    pub event_fee_in_cents: i64,
    pub sendgrid_api_key: Option<String>,
    pub google_ga_key: Option<String>,
    pub facebook_pixel_key: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub fee_schedule_id: Uuid,
    pub client_event_fee_in_cents: i64,
    pub company_event_fee_in_cents: i64,
    pub allowed_payment_providers: Vec<PaymentProviders>,
    pub timezone: Option<String>,
    pub cc_fee_percent: f32,
    pub globee_api_key: Option<String>,
    pub max_instances_per_ticket_type: i64,
    pub max_additional_fee_in_cents: i64,
    pub settlement_type: SettlementTypes,
    pub slug_id: Option<Uuid>,
    pub google_ads_conversion_id: Option<String>,
    pub google_ads_conversion_labels: Vec<String>,
}

#[derive(Serialize)]
pub struct DisplayOrganizationLink {
    pub id: Uuid,
    pub name: String,
    pub role: Vec<Roles>,
}

#[derive(Default, Insertable, Serialize, Deserialize, PartialEq, Debug, Clone)]
#[table_name = "organizations"]
pub struct NewOrganization {
    pub name: String,
    pub fee_schedule_id: Uuid,
    pub address: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
    pub postal_code: Option<String>,
    pub phone: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub sendgrid_api_key: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub google_ga_key: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub facebook_pixel_key: Option<String>,
    pub client_event_fee_in_cents: Option<i64>,
    pub company_event_fee_in_cents: Option<i64>,
    pub allowed_payment_providers: Option<Vec<PaymentProviders>>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub timezone: Option<String>,
    pub cc_fee_percent: f32,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub globee_api_key: Option<String>,
    pub max_instances_per_ticket_type: Option<i64>,
    pub settlement_type: Option<SettlementTypes>,
}

#[derive(Default, Serialize, Clone, Deserialize, Debug, PartialEq)]
pub struct TrackingKeys {
    pub google_ga_key: Option<String>,
    pub facebook_pixel_key: Option<String>,
    pub google_ads_conversion_id: Option<String>,
    pub google_ads_conversion_labels: Vec<String>,
}

impl NewOrganization {
    pub fn commit(
        self,
        settlement_period_in_days: Option<u32>,
        encryption_key: &str,
        current_user_id: Option<Uuid>,
        conn: &PgConnection,
    ) -> Result<Organization, DatabaseError> {
        let mut updated_organisation = self;
        if encryption_key.len() > 0 {
            if let Some(key) = updated_organisation.sendgrid_api_key.clone() {
                updated_organisation.sendgrid_api_key = Some(encrypt(&key, encryption_key)?);
            }
            if let Some(key) = updated_organisation.google_ga_key.clone() {
                updated_organisation.google_ga_key = Some(encrypt(&key, encryption_key)?);
            }
            if let Some(key) = updated_organisation.facebook_pixel_key.clone() {
                updated_organisation.facebook_pixel_key = Some(encrypt(&key, encryption_key)?);
            }
            if let Some(key) = updated_organisation.globee_api_key.clone() {
                updated_organisation.globee_api_key = Some(encrypt(&key, encryption_key)?);
            }
        }

        let org: Organization = diesel::insert_into(organizations::table)
            .values((
                &updated_organisation,
                organizations::event_fee_in_cents.eq(updated_organisation.client_event_fee_in_cents.unwrap_or(0)
                    + updated_organisation.company_event_fee_in_cents.unwrap_or(0)),
            ))
            .get_result(conn)
            .to_db_error(ErrorCode::InsertError, "Could not create new organization")?;

        let slug = Slug::generate_slug(
            &SlugContext::Organization {
                id: org.id,
                name: org.name.clone(),
            },
            SlugTypes::Organization,
            conn,
        )?;
        let org = diesel::update(&org)
            .set((
                organizations::updated_at.eq(dsl::now),
                organizations::slug_id.eq(slug.id),
            ))
            .get_result::<Organization>(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not update organization slug")?;

        diesel::update(fee_schedules::table.filter(fee_schedules::id.eq(org.fee_schedule_id)))
            .set((
                fee_schedules::organization_id.eq(org.id),
                fee_schedules::updated_at.eq(dsl::now),
            ))
            .execute(conn)
            .to_db_error(
                ErrorCode::UpdateError,
                "Could not set the fee schedule for this organization",
            )?;
        org.schedule_domain_actions(settlement_period_in_days, conn)?;

        DomainEvent::create(
            DomainEventTypes::OrganizationCreated,
            "Organization created".to_string(),
            Tables::Organizations,
            Some(org.id),
            current_user_id,
            Some(json!(updated_organisation)),
        )
        .commit(conn)?;

        Ok(org)
    }
}

#[derive(AsChangeset, Default, Deserialize)]
#[table_name = "organizations"]
pub struct OrganizationEditableAttributes {
    pub name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub address: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub city: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub state: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub country: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub postal_code: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub phone: Option<String>,
    #[serde(default, deserialize_with = "double_option_deserialize_unless_blank")]
    pub sendgrid_api_key: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option_deserialize_unless_blank")]
    pub google_ga_key: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option_deserialize_unless_blank")]
    pub facebook_pixel_key: Option<Option<String>>,
    pub client_event_fee_in_cents: Option<i64>,
    pub company_event_fee_in_cents: Option<i64>,
    pub allowed_payment_providers: Option<Vec<PaymentProviders>>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub timezone: Option<String>,
    pub cc_fee_percent: Option<f32>,
    #[serde(default, deserialize_with = "double_option_deserialize_unless_blank")]
    pub globee_api_key: Option<Option<String>>,
    #[serde(default)]
    pub max_instances_per_ticket_type: Option<i64>,
    #[serde(default)]
    pub max_additional_fee_in_cents: Option<i64>,
    pub settlement_type: Option<SettlementTypes>,
    #[serde(default, deserialize_with = "double_option_deserialize_unless_blank")]
    pub google_ads_conversion_id: Option<Option<String>>,
    #[serde(default)]
    pub google_ads_conversion_labels: Option<Vec<String>>,
}

impl Organization {
    pub fn create_next_settlement_processing_domain_action(
        &self,
        settlement_period_in_days: Option<u32>,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        if let Some(upcoming_domain_action) = DomainAction::upcoming_domain_action(
            Some(Tables::Organizations),
            Some(self.id),
            DomainActionTypes::ProcessSettlementReport,
            conn,
        )? {
            if upcoming_domain_action.scheduled_at > Utc::now().naive_utc() {
                return DatabaseError::already_scheduled_error(
                    "Settlement processing domain action is already pending",
                );
            }
        }

        let mut action = DomainAction::create(
            None,
            DomainActionTypes::ProcessSettlementReport,
            None,
            json!({}),
            Some(Tables::Organizations),
            Some(self.id),
        );
        action.schedule_at(self.next_settlement_date(settlement_period_in_days)?);
        action.commit(conn)?;

        Ok(())
    }

    pub fn can_process_settlements(&self, conn: &PgConnection) -> Result<bool, DatabaseError> {
        Ok(self.first_order_date(conn).optional()?.is_some())
    }

    pub fn schedule_domain_actions(
        &self,
        settlement_period_in_days: Option<u32>,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        // Settlements weekly domain event
        if DomainAction::upcoming_domain_action(
            Some(Tables::Organizations),
            Some(self.id),
            DomainActionTypes::ProcessSettlementReport,
            conn,
        )?
        .is_none()
        {
            self.create_next_settlement_processing_domain_action(settlement_period_in_days, conn)?
        }

        Ok(())
    }

    pub fn slug(&self, conn: &PgConnection) -> Result<Slug, DatabaseError> {
        match self.slug_id {
            Some(s) => Ok(Slug::find(s, conn)?),
            None => DatabaseError::no_results("Organization does not have a slug"),
        }
    }

    pub fn timezone(&self) -> Result<Tz, DatabaseError> {
        self.timezone
            .clone()
            .unwrap_or(DEFAULT_SETTLEMENT_TIMEZONE.to_string())
            .parse::<Tz>()
            .map_err(|e| DatabaseError::business_process_error::<Tz>(&e).unwrap_err())
    }

    pub fn first_order_date(&self, conn: &PgConnection) -> Result<NaiveDateTime, DatabaseError> {
        organizations::table
            .inner_join(events::table.on(events::organization_id.eq(organizations::id)))
            .inner_join(order_items::table.on(order_items::event_id.eq(events::id.nullable())))
            .inner_join(orders::table.on(orders::id.eq(order_items::order_id)))
            .filter(organizations::id.eq(self.id))
            .select(orders::created_at)
            .order_by(orders::created_at.asc())
            .limit(1)
            .get_result(conn)
            .to_db_error(ErrorCode::QueryError, "Error loading first order date")
    }

    pub fn next_settlement_date(&self, settlement_period_in_days: Option<u32>) -> Result<NaiveDateTime, DatabaseError> {
        let timezone = if self.settlement_type == SettlementTypes::Rolling {
            "America/Los_Angeles"
                .to_string()
                .parse::<Tz>()
                .map_err(|e| DatabaseError::business_process_error::<Tz>(&e).unwrap_err())?
        } else {
            self.timezone()?
        };
        let now = timezone.from_utc_datetime(&Utc::now().naive_utc());
        let today = timezone.ymd(now.year(), now.month(), now.day()).and_hms(0, 0, 0);

        let start_hour = if self.settlement_type == SettlementTypes::PostEvent {
            3
        } else {
            0
        };

        // If this is a normal week long settlement period, set it to start on the following Monday
        // Else set as number of days from today
        if let Some(settlement_period) = settlement_period_in_days {
            let next_period = today.naive_local() + Duration::days(settlement_period as i64);
            Ok(timezone
                .ymd(next_period.year(), next_period.month(), next_period.day())
                .and_hms(start_hour, 0, 0)
                .naive_utc())
        } else {
            let next_period_date = now
                + Duration::days(
                    DEFAULT_SETTLEMENT_PERIOD_IN_DAYS - today.naive_local().weekday().num_days_from_monday() as i64,
                );
            Ok(timezone
                .ymd(
                    next_period_date.year(),
                    next_period_date.month(),
                    next_period_date.day(),
                )
                .and_hms(start_hour, 0, 0)
                .naive_utc())
        }
    }

    pub fn create(name: &str, fee_schedule_id: Uuid) -> NewOrganization {
        NewOrganization {
            name: name.into(),
            fee_schedule_id,
            ..Default::default()
        }
    }

    pub fn update(
        &self,
        mut attributes: OrganizationEditableAttributes,
        settlement_period_in_days: Option<u32>,
        encryption_key: &String,
        conn: &PgConnection,
    ) -> Result<Organization, DatabaseError> {
        if encryption_key.len() > 0 {
            if let Some(Some(key)) = attributes.sendgrid_api_key {
                attributes.sendgrid_api_key = Some(Some(encrypt(&key, encryption_key)?));
            }
            if let Some(Some(key)) = attributes.google_ga_key {
                attributes.google_ga_key = Some(Some(encrypt(&key, encryption_key)?));
            }
            if let Some(Some(key)) = attributes.facebook_pixel_key {
                attributes.facebook_pixel_key = Some(Some(encrypt(&key, encryption_key)?));
            }
            if let Some(Some(key)) = attributes.globee_api_key {
                attributes.globee_api_key = Some(Some(encrypt(&key, encryption_key)?));
            }

            if let Some(Some(key)) = attributes.google_ads_conversion_id {
                attributes.google_ads_conversion_id = Some(Some(encrypt(&key, encryption_key)?));
            }
        }

        if attributes.timezone.is_some() && attributes.timezone != self.timezone {
            if let Some(settlement_job) = DomainAction::upcoming_domain_action(
                Some(Tables::Organizations),
                Some(self.id),
                DomainActionTypes::ProcessSettlementReport,
                conn,
            )? {
                settlement_job.set_cancelled(conn)?;
            }
        }

        let event_fee = attributes
            .client_event_fee_in_cents
            .clone()
            .unwrap_or(self.client_event_fee_in_cents)
            + attributes
                .company_event_fee_in_cents
                .clone()
                .unwrap_or(self.company_event_fee_in_cents);

        let organization = diesel::update(&*self)
            .set((
                attributes,
                organizations::updated_at.eq(dsl::now),
                organizations::event_fee_in_cents.eq(event_fee),
            ))
            .get_result::<Organization>(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not update organization")?;
        organization.schedule_domain_actions(settlement_period_in_days, conn)?;

        Ok(organization)
    }

    pub fn find_by_asset_id(asset_id: Uuid, conn: &PgConnection) -> Result<Organization, DatabaseError> {
        organizations::table
            .inner_join(events::table.on(events::organization_id.eq(organizations::id)))
            .inner_join(ticket_types::table.on(ticket_types::event_id.eq(events::id)))
            .inner_join(assets::table.on(assets::ticket_type_id.eq(ticket_types::id)))
            .filter(assets::id.eq(asset_id))
            .select(organizations::all_columns)
            .get_result(conn)
            .to_db_error(ErrorCode::QueryError, "Could not retrieve organization by asset id")
    }

    pub fn find_by_ticket_type_ids(
        ticket_type_ids: Vec<Uuid>,
        conn: &PgConnection,
    ) -> Result<Vec<Organization>, DatabaseError> {
        organizations::table
            .inner_join(events::table.on(events::organization_id.eq(organizations::id)))
            .inner_join(ticket_types::table.on(ticket_types::event_id.eq(events::id)))
            .filter(ticket_types::id.eq_any(ticket_type_ids))
            .select(organizations::all_columns)
            .distinct()
            .get_results(conn)
            .to_db_error(
                ErrorCode::QueryError,
                "Could not retrieve organizations by ticket type ids",
            )
    }

    pub fn find_by_order_item_ids(
        order_item_ids: &Vec<Uuid>,
        conn: &PgConnection,
    ) -> Result<Vec<Organization>, DatabaseError> {
        organizations::table
            .inner_join(events::table.on(events::organization_id.eq(organizations::id)))
            .inner_join(order_items::table.on(order_items::event_id.eq(events::id.nullable())))
            .filter(order_items::id.eq_any(order_item_ids))
            .select(organizations::all_columns)
            .order_by(organizations::name.asc())
            .distinct()
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Error loading organizations")
    }

    pub fn users(
        &self,
        event_id: Option<Uuid>,
        conn: &PgConnection,
    ) -> Result<Vec<(OrganizationUser, User)>, DatabaseError> {
        let mut query = organization_users::table
            .inner_join(users::table.on(users::id.eq(organization_users::user_id)))
            .left_join(
                event_users::table.on(event_users::user_id
                    .eq(users::id)
                    .and(event_users::event_id.nullable().eq(event_id))),
            )
            .into_boxed();

        if event_id.is_some() {
            query = query.filter(event_users::id.is_not_null());
        }

        let mut users = query
            .filter(organization_users::organization_id.eq(self.id))
            .select(organization_users::all_columns)
            .order_by(users::last_name.asc())
            .then_order_by(users::first_name.asc())
            .load::<OrganizationUser>(conn)
            .to_db_error(ErrorCode::QueryError, "Could not retrieve organization users")?;

        let mut results = vec![];

        // Search scoped to event, limit roles to event based roles
        if let Some(event_id) = event_id {
            let event_users = EventUser::find_all_by_event_id(event_id, conn)?;
            let mut user_map: HashMap<Uuid, Roles> = HashMap::new();
            for event_user in event_users {
                user_map.insert(event_user.user_id, event_user.role);
            }

            for mut u in &mut users {
                if let Some(role) = user_map.get(&u.user_id) {
                    u.role = vec![*role];
                }
            }
        }

        for u in users {
            let user = User::find(u.user_id, conn)?;
            results.push((u, user));
        }

        Ok(results)
    }

    pub fn pending_invites(
        &self,
        event_id: Option<Uuid>,
        conn: &PgConnection,
    ) -> Result<Vec<OrganizationInvite>, DatabaseError> {
        OrganizationInvite::find_pending_by_organization(self.id, event_id, conn)
    }

    pub fn events(&self, conn: &PgConnection) -> Result<Vec<Event>, DatabaseError> {
        events::table
            .filter(events::deleted_at.is_null())
            .filter(events::organization_id.eq(self.id))
            .order_by(events::created_at)
            .get_results::<EventData>(conn)
            .to_db_error(ErrorCode::QueryError, "Could not retrieve events")
            .map(EventData::vec_into_events)
    }

    pub fn upcoming_events(&self, conn: &PgConnection) -> Result<Vec<Event>, DatabaseError> {
        events::table
            .filter(events::deleted_at.is_null())
            .filter(events::organization_id.eq(self.id))
            .filter(events::status.eq(EventStatus::Published))
            .filter(events::event_start.ge(Utc::now().naive_utc()))
            .order_by(events::created_at)
            .get_results::<EventData>(conn)
            .to_db_error(ErrorCode::QueryError, "Could not retrieve upcoming events")
            .map(EventData::vec_into_events)
    }

    pub fn venues(&self, conn: &PgConnection) -> Result<Vec<Venue>, DatabaseError> {
        OrganizationVenue::find_venues_by_organization(self.id, conn)
    }

    pub fn has_fan(&self, user: &User, conn: &PgConnection) -> Result<bool, DatabaseError> {
        let results = self.search_fans(
            Some(user.id.to_string()),
            0,
            1,
            FanSortField::Email,
            SortingDir::Desc,
            conn,
        )?;
        Ok(!results.data.is_empty())
    }

    pub fn get_scopes_for_user(&self, user: &User, conn: &PgConnection) -> Result<Vec<Scopes>, DatabaseError> {
        let (roles, additional_scopes) = self.get_roles_for_user(user, conn)?;
        let user_scopes = scopes::get_scopes(roles, additional_scopes);

        Ok(user_scopes)
    }

    pub fn get_roles_for_user(
        &self,
        user: &User,
        conn: &PgConnection,
    ) -> Result<(Vec<Roles>, Option<AdditionalOrgMemberScopes>), DatabaseError> {
        if user.is_admin() {
            let mut roles = Vec::new();

            roles.push(Roles::OrgOwner);

            Ok((roles, None))
        } else {
            let org_member = OrganizationUser::find_by_user_id(user.id, self.id, conn).optional()?;

            match org_member {
                Some(member) => {
                    let additional_scopes: Option<AdditionalOrgMemberScopes> =
                        member.additional_scopes.map(|a| a.into());
                    Ok((member.role, additional_scopes))
                }
                None => Ok((vec![], None)),
            }
        }
    }

    pub fn find(id: Uuid, conn: &PgConnection) -> Result<Organization, DatabaseError> {
        organizations::table
            .find(id)
            .first::<Organization>(conn)
            .to_db_error(ErrorCode::QueryError, "Error loading organization")
    }

    pub fn find_for_event(event_id: Uuid, conn: &PgConnection) -> Result<Organization, DatabaseError> {
        events::table
            .inner_join(organizations::table)
            .filter(events::deleted_at.is_null())
            .filter(events::id.eq(event_id))
            .select(organizations::all_columns)
            .first(conn)
            .to_db_error(ErrorCode::QueryError, "Could not find organization for this event")
    }

    pub fn tracking_keys_for_ids(
        org_ids: Vec<Uuid>,
        encryption_key: &String,
        conn: &PgConnection,
    ) -> Result<HashMap<Uuid, TrackingKeys>, DatabaseError> {
        #[derive(Queryable)]
        struct TrackingKeyFields {
            pub id: Uuid,
            pub google_ga_key: Option<String>,
            pub facebook_pixel_key: Option<String>,
            pub google_ads_conversion_id: Option<String>,
            pub google_ads_conversion_labels: Vec<String>,
        };

        let orgs: Vec<TrackingKeyFields> = organizations::table
            .filter(organizations::id.eq_any(org_ids))
            .select((
                organizations::id,
                organizations::google_ga_key,
                organizations::facebook_pixel_key,
                organizations::google_ads_conversion_id,
                organizations::google_ads_conversion_labels,
            ))
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Unable to load all organization tracking keys")?;

        let mut result: HashMap<Uuid, TrackingKeys> = HashMap::new();

        for org in orgs {
            let mut google_ga_key = org.google_ga_key;
            if let Some(key) = google_ga_key.clone() {
                google_ga_key = Some(decrypt(&key, &encryption_key)?);
            }
            let mut facebook_pixel_key = org.facebook_pixel_key;
            if let Some(key) = facebook_pixel_key.clone() {
                facebook_pixel_key = Some(decrypt(&key, &encryption_key)?);
            }
            let mut google_ads_conversion_id = org.google_ads_conversion_id;
            let mut google_ads_conversion_labels = vec![];
            if let Some(key) = google_ads_conversion_id.clone() {
                google_ads_conversion_id = Some(decrypt(&key, &encryption_key)?);
                google_ads_conversion_labels = org.google_ads_conversion_labels.clone();
            }

            result.insert(
                org.id,
                TrackingKeys {
                    google_ga_key,
                    facebook_pixel_key,
                    google_ads_conversion_id,
                    google_ads_conversion_labels,
                },
            );
        }

        Ok(result)
    }

    pub fn all(conn: &PgConnection) -> Result<Vec<Organization>, DatabaseError> {
        DatabaseError::wrap(
            ErrorCode::QueryError,
            "Unable to load all organizations",
            organizations::table.order_by(organizations::name).load(conn),
        )
    }

    pub fn all_linked_to_user(user_id: Uuid, conn: &PgConnection) -> Result<Vec<Organization>, DatabaseError> {
        let orgs = organization_users::table
            .filter(organization_users::user_id.eq(user_id))
            .inner_join(organizations::table)
            .select(organizations::all_columns)
            .order_by(organizations::name.asc())
            .load::<Organization>(conn)
            .to_db_error(ErrorCode::QueryError, "Unable to load all organizations")?;

        Ok(orgs)
    }

    pub fn all_org_names_linked_to_user(
        user_id: Uuid,
        conn: &PgConnection,
    ) -> Result<Vec<DisplayOrganizationLink>, DatabaseError> {
        //Compile list of organisations where the user is a member of that organisation
        let org_member_list: Vec<OrganizationUser> = organization_users::table
            .filter(organization_users::user_id.eq(user_id))
            .inner_join(organizations::table)
            .select(organization_users::all_columns)
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Unable to load all organizations")?;
        //Compile complete list with only display information
        let mut result_list: Vec<DisplayOrganizationLink> = Vec::new();

        for curr_org_member in &org_member_list {
            let curr_entry = DisplayOrganizationLink {
                id: curr_org_member.organization_id,
                name: Organization::find(curr_org_member.organization_id, conn)?.name,
                role: curr_org_member.role.clone(),
            };
            result_list.push(curr_entry);
        }
        result_list.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(result_list)
    }

    pub fn remove_user(&self, user_id: Uuid, conn: &PgConnection) -> Result<usize, DatabaseError> {
        let event_ids: Vec<Uuid> = self.events(conn)?.iter().map(|e| e.id).collect();
        diesel::delete(
            event_users::table
                .filter(event_users::user_id.eq(user_id))
                .filter(event_users::event_id.eq_any(event_ids)),
        )
        .execute(conn)
        .to_db_error(ErrorCode::DeleteError, "Error removing event promoters")?;

        diesel::delete(
            organization_users::table
                .filter(organization_users::user_id.eq(user_id))
                .filter(organization_users::organization_id.eq(self.id)),
        )
        .execute(conn)
        .to_db_error(ErrorCode::DeleteError, "Error removing user")
    }

    pub fn add_user(
        &self,
        user_id: Uuid,
        role: Vec<Roles>,
        event_ids: Vec<Uuid>,
        conn: &PgConnection,
    ) -> Result<OrganizationUser, DatabaseError> {
        let org_user = OrganizationUser::create(self.id, user_id, role.clone()).commit(conn)?;
        if event_ids.len() > 0 {
            if role.iter().position(|&r| r == Roles::Promoter).is_some() {
                EventUser::update_or_create(user_id, &event_ids, Roles::Promoter, conn)?;
            } else if role.iter().position(|&r| r == Roles::PromoterReadOnly).is_some() {
                EventUser::update_or_create(user_id, &event_ids, Roles::PromoterReadOnly, conn)?;
            }
        }

        Ok(org_user)
    }

    pub fn is_member(&self, user: &User, conn: &PgConnection) -> Result<bool, DatabaseError> {
        select(exists(
            organization_users::table
                .filter(
                    organization_users::user_id
                        .eq(user.id)
                        .and(organization_users::organization_id.eq(self.id)),
                )
                .select(organization_users::organization_id),
        ))
        .get_result(conn)
        .to_db_error(ErrorCode::QueryError, "Could not check user member status")
    }

    pub fn add_fee_schedule(
        &self,
        fee_schedule: &FeeSchedule,
        conn: &PgConnection,
    ) -> Result<Organization, DatabaseError> {
        diesel::update(fee_schedule)
            .set((
                fee_schedules::organization_id.eq(self.id),
                fee_schedules::updated_at.eq(dsl::now),
            ))
            .execute(conn)
            .to_db_error(
                ErrorCode::UpdateError,
                "Could not set the fee schedule for this organization",
            )?;

        diesel::update(self)
            .set((
                organizations::fee_schedule_id.eq(fee_schedule.id),
                organizations::updated_at.eq(dsl::now),
            ))
            .get_result(conn)
            .to_db_error(
                ErrorCode::UpdateError,
                "Could not set the fee schedule for this organization",
            )
    }

    pub fn search_fans(
        &self,
        //        event_id: Option<Uuid>,
        search_query: Option<String>,
        page: u32,
        limit: u32,
        sort_field: FanSortField,
        sort_direction: SortingDir,
        conn: &PgConnection,
    ) -> Result<Payload<DisplayFan>, DatabaseError> {
        use schema::*;
        let sort_column = match sort_field {
            FanSortField::FirstName => "2",
            FanSortField::LastName => "3",
            FanSortField::Email => "4",
            FanSortField::Phone => "5",
            FanSortField::OrganizationId => "7",
            FanSortField::Orders => "8",
            FanSortField::UserCreated => "9",
            FanSortField::FirstOrder => "10",
            FanSortField::LastOrder => "11",
            FanSortField::Revenue => "12",
            FanSortField::FirstInteracted => "13",
            FanSortField::LastInteracted => "14",
        };

        let mut query = organization_interactions::table
            .inner_join(users::table)
            .filter(organization_interactions::organization_id.eq(self.id))
            .into_boxed();

        // Parse UUID if passed into query to search for a specific user
        if let Some(query_text) = search_query {
            if query_text.trim().len() > 0 {
                if let Ok(uuid) = Uuid::parse_str(&query_text) {
                    query = query.filter(users::id.eq(uuid));
                } else {
                    let query_string = text::escape_control_chars(&query_text);
                    let fuzzy_query_string: String = str::replace(&query_string.trim(), ",", "");
                    let fuzzy_query_string = fuzzy_query_string
                        .split_whitespace()
                        .map(|w| w.split("").collect::<Vec<&str>>().join("%"))
                        .collect::<Vec<String>>()
                        .join("%");

                    query = query.filter(
                        sql("users.email ILIKE ")
                            .bind::<Text, _>(fuzzy_query_string.clone())
                            .or(sql("users.phone ILIKE ").bind::<Text, _>(fuzzy_query_string.clone()))
                            .or(sql("CONCAT(users.first_name, ' ', users.last_name) ILIKE ")
                                .bind::<Text, _>(fuzzy_query_string.clone()))
                            .or(sql("CONCAT(users.last_name, ' ', users.first_name) ILIKE ")
                                .bind::<Text, _>(fuzzy_query_string.clone())),
                    );
                }
            }
        }

        //        if let Some(event_id) = event_id {
        //            query = query.filter(events::id.eq(event_id));
        //        }

        //First fetch all of the fans
        let (fans, record_count): (Vec<DisplayFan>, i64) = query
            .group_by((
                organization_interactions::organization_id,
                users::first_name,
                users::last_name,
                users::email,
                users::phone,
                users::id,
                organization_interactions::first_interaction,
                organization_interactions::last_interaction,
            ))
            .select((
                users::id,
                users::first_name,
                users::last_name,
                users::email,
                users::phone,
                users::thumb_profile_pic_url,
                organization_interactions::organization_id,
                sql::<Nullable<BigInt>>("CAST (0 AS BIGINT)"), //order_count
                users::created_at,
                sql::<Nullable<Timestamp>>("NULL"),            //first_order_time
                sql::<Nullable<Timestamp>>("NULL"),            //last_order_time
                sql::<Nullable<BigInt>>("CAST (0 AS BIGINT)"), //revenue_in_cents - This will be replaced
                organization_interactions::first_interaction.nullable(),
                organization_interactions::last_interaction.nullable(),
            ))
            .order_by(sql::<()>(&format!("{} {}", sort_column, sort_direction)))
            .paginate(page as i64)
            .per_page(limit as i64)
            .load_and_count_pages(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load fans for organization")?;

        //Now extract all of the user_ids that were found
        let user_ids: Vec<Uuid> = fans.iter().map(|x| x.user_id).collect();
        let user_value_map: HashMap<Uuid, FanRevenue> = self.user_revenue_totals(user_ids, conn)?;

        //Map fans with their new values if they were found
        let fans = fans
            .into_iter()
            .map(|x| {
                let fan_revenue = user_value_map
                    .get(&x.user_id)
                    .map(|x| x.clone())
                    .unwrap_or(FanRevenue { ..Default::default() }.clone());
                let fan = DisplayFan {
                    revenue_in_cents: Some(fan_revenue.revenue_in_cents.unwrap_or(0)),
                    first_order_time: fan_revenue.first_order_time,
                    last_order_time: fan_revenue.last_order_time,
                    order_count: Some(fan_revenue.order_count.unwrap_or(0)),
                    ..x
                };
                fan
            })
            .collect();

        let payload = Payload::from_data(fans, page, limit, Some(record_count as u64));
        Ok(payload)
    }

    pub fn log_interaction(
        &self,
        user_id: Uuid,
        interaction_date: NaiveDateTime,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        if let Some(interaction_data) = self.interaction_data(user_id, conn).optional()? {
            interaction_data.update(
                &OrganizationInteractionEditableAttributes {
                    first_interaction: Some(cmp::min(interaction_data.first_interaction, interaction_date)),
                    last_interaction: Some(cmp::max(interaction_data.last_interaction, interaction_date)),
                    interaction_count: Some(interaction_data.interaction_count + 1),
                },
                conn,
            )?;
        } else {
            OrganizationInteraction::create(self.id, user_id, interaction_date, interaction_date, 1).commit(conn)?;
        }

        Ok(())
    }

    pub fn regenerate_interaction_data(&self, user_id: Uuid, conn: &PgConnection) -> Result<(), DatabaseError> {
        use schema::*;
        #[derive(Debug, Queryable, QueryableByName)]
        struct R {
            #[sql_type = "Nullable<Timestamp>"]
            first_interaction: Option<NaiveDateTime>,
            #[sql_type = "Nullable<Timestamp>"]
            last_interaction: Option<NaiveDateTime>,
            #[sql_type = "BigInt"]
            interaction_count: i64,
        }

        // Load order data
        let order_data: R = orders::table
            .inner_join(order_items::table.on(order_items::order_id.eq(orders::id)))
            .inner_join(events::table.on(order_items::event_id.eq(events::id.nullable())))
            .filter(orders::status.eq(OrderStatus::Paid))
            .filter(events::organization_id.eq(self.id))
            .filter(
                orders::on_behalf_of_user_id
                    .eq(user_id)
                    .or(orders::on_behalf_of_user_id.is_null().and(orders::user_id.eq(user_id))),
            )
            .select((
                sql::<Nullable<Timestamp>>("MIN(orders.order_date)"),
                sql::<Nullable<Timestamp>>("MAX(orders.order_date)"),
                sql::<BigInt>("COUNT(DISTINCT orders.id)"),
            ))
            .get_result(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load fan order data")?;

        let mut first_interaction = order_data.first_interaction;
        let mut last_interaction = order_data.last_interaction;
        let mut interaction_count = order_data.interaction_count;

        // Load transfer data
        let transfer_data: R = transfers::table
            .inner_join(transfer_tickets::table.on(transfer_tickets::transfer_id.eq(transfers::id)))
            .inner_join(
                ticket_instances::table
                    .on(ticket_instances::id.eq(transfer_tickets::ticket_instance_id)),
            )
            .inner_join(assets::table.on(assets::id.eq(ticket_instances::asset_id)))
            .inner_join(ticket_types::table.on(ticket_types::id.eq(assets::ticket_type_id)))
            .inner_join(events::table.on(events::id.eq(ticket_types::event_id)))
            .filter(events::organization_id.eq(self.id))
            .filter(
                transfers::destination_user_id
                    .eq(user_id)
                    .or(transfers::source_user_id.eq(user_id)),
            )
            .select((
                sql::<Nullable<Timestamp>>("MIN(transfers.created_at)"),
                sql::<Nullable<Timestamp>>("MAX(transfers.updated_at)"),
                sql::<BigInt>(&format!("
                    COUNT(DISTINCT transfers.id) FILTER(WHERE transfers.source_user_id = '{}') +
                    COUNT(DISTINCT transfers.id) FILTER(WHERE transfers.status = 'Completed' or transfers.status = 'Cancelled')
                ", user_id)),
            ))
            .get_result(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load fan transfer data")?;
        first_interaction = first_interaction.or(transfer_data.first_interaction);
        if let (Some(old_first_interaction), Some(new_first_interaction)) =
            (first_interaction, transfer_data.first_interaction)
        {
            first_interaction = Some(cmp::min(old_first_interaction, new_first_interaction));
        }
        last_interaction = last_interaction.or(transfer_data.last_interaction);
        if let (Some(old_last_interaction), Some(new_last_interaction)) =
            (last_interaction, transfer_data.last_interaction)
        {
            last_interaction = Some(cmp::max(old_last_interaction, new_last_interaction));
        }
        interaction_count += transfer_data.interaction_count;

        // Load ticket data
        let ticket_data: R = ticket_instances::table
            .inner_join(wallets::table.on(ticket_instances::wallet_id.eq(wallets::id)))
            .inner_join(assets::table.on(assets::id.eq(ticket_instances::asset_id)))
            .inner_join(ticket_types::table.on(ticket_types::id.eq(assets::ticket_type_id)))
            .inner_join(events::table.on(events::id.eq(ticket_types::event_id)))
            .filter(events::organization_id.eq(self.id))
            .filter(wallets::user_id.eq(user_id))
            .filter(ticket_instances::redeemed_at.is_not_null())
            .select((
                sql::<Nullable<Timestamp>>("MIN(ticket_instances.redeemed_at)"),
                sql::<Nullable<Timestamp>>("MAX(ticket_instances.redeemed_at)"),
                sql::<BigInt>("COUNT(DISTINCT ticket_instances.id)"),
            ))
            .get_result(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load fan ticket data")?;
        first_interaction = first_interaction.or(ticket_data.first_interaction);
        if let (Some(old_first_interaction), Some(new_first_interaction)) =
            (first_interaction, ticket_data.first_interaction)
        {
            first_interaction = Some(cmp::min(old_first_interaction, new_first_interaction));
        }
        last_interaction = last_interaction.or(ticket_data.last_interaction);
        if let (Some(old_last_interaction), Some(new_last_interaction)) =
            (last_interaction, ticket_data.last_interaction)
        {
            last_interaction = Some(cmp::max(old_last_interaction, new_last_interaction));
        }
        interaction_count += ticket_data.interaction_count;

        // Load event interest data
        let event_interest_data: R = event_interest::table
            .inner_join(events::table.on(events::id.eq(event_interest::event_id)))
            .filter(events::organization_id.eq(self.id))
            .filter(event_interest::user_id.eq(user_id))
            .select((
                sql::<Nullable<Timestamp>>("MIN(event_interest.created_at)"),
                sql::<Nullable<Timestamp>>("MAX(event_interest.created_at)"),
                sql::<BigInt>("COUNT(DISTINCT event_interest.id)"),
            ))
            .get_result(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load fan event interest data")?;
        first_interaction = first_interaction.or(event_interest_data.first_interaction);
        if let (Some(old_first_interaction), Some(new_first_interaction)) =
            (first_interaction, event_interest_data.first_interaction)
        {
            first_interaction = Some(cmp::min(old_first_interaction, new_first_interaction));
        }
        last_interaction = last_interaction.or(event_interest_data.last_interaction);
        if let (Some(old_last_interaction), Some(new_last_interaction)) =
            (last_interaction, event_interest_data.last_interaction)
        {
            last_interaction = Some(cmp::max(old_last_interaction, new_last_interaction));
        }
        interaction_count += event_interest_data.interaction_count;

        // Load event interest data
        let refund_data: R = refunds::table
            .inner_join(orders::table.on(orders::id.eq(refunds::order_id)))
            .inner_join(order_items::table.on(order_items::order_id.eq(orders::id)))
            .inner_join(events::table.on(order_items::event_id.eq(events::id.nullable())))
            .filter(events::organization_id.eq(self.id))
            .filter(
                orders::on_behalf_of_user_id
                    .eq(user_id)
                    .or(orders::on_behalf_of_user_id.is_null().and(orders::user_id.eq(user_id))),
            )
            .select((
                sql::<Nullable<Timestamp>>("MIN(refunds.created_at)"),
                sql::<Nullable<Timestamp>>("MAX(refunds.created_at)"),
                sql::<BigInt>("COUNT(DISTINCT refunds.id)"),
            ))
            .get_result(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load fan event interest data")?;
        first_interaction = first_interaction.or(refund_data.first_interaction);
        if let (Some(old_first_interaction), Some(new_first_interaction)) =
            (first_interaction, refund_data.first_interaction)
        {
            first_interaction = Some(cmp::min(old_first_interaction, new_first_interaction));
        }
        last_interaction = last_interaction.or(refund_data.last_interaction);
        if let (Some(old_last_interaction), Some(new_last_interaction)) =
            (last_interaction, refund_data.last_interaction)
        {
            last_interaction = Some(cmp::max(old_last_interaction, new_last_interaction));
        }
        interaction_count += refund_data.interaction_count;

        if let Some(interaction_data) = self.interaction_data(user_id, conn).optional()? {
            interaction_data.update(
                &OrganizationInteractionEditableAttributes {
                    first_interaction,
                    last_interaction,
                    interaction_count: Some(interaction_count),
                },
                conn,
            )?;
        } else {
            if let (Some(first_interaction), Some(last_interaction)) = (first_interaction, last_interaction) {
                OrganizationInteraction::create(
                    self.id,
                    user_id,
                    first_interaction,
                    last_interaction,
                    interaction_count,
                )
                .commit(conn)?;
            }
        }

        Ok(())
    }

    pub fn interaction_data(
        &self,
        user_id: Uuid,
        conn: &PgConnection,
    ) -> Result<OrganizationInteraction, DatabaseError> {
        OrganizationInteraction::find_by_organization_user(self.id, user_id, conn)
    }

    pub fn user_revenue_totals(
        &self,
        user_ids: Vec<Uuid>,
        conn: &PgConnection,
    ) -> Result<HashMap<Uuid, FanRevenue>, DatabaseError> {
        let query_revenue = include_str!("../queries/total_revenue_per_user.sql");
        let user_values: Vec<FanRevenue> = diesel::sql_query(query_revenue)
            .bind::<dUuid, _>(self.id)
            .bind::<Text, _>(OrderStatus::Paid)
            .bind::<Array<dUuid>, _>(user_ids)
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load revenue per fan")?;

        //Store the user_id:value in a hashmap for easy collection
        let mut user_value_map: HashMap<Uuid, FanRevenue> = HashMap::new();
        for i in user_values.iter() {
            user_value_map.insert(i.user_id, i.clone());
        }
        Ok(user_value_map)
    }

    pub fn decrypt(&mut self, encryption_key: &str) -> Result<(), DatabaseError> {
        if encryption_key.len() > 0 {
            if let Some(key) = self.sendgrid_api_key.clone() {
                self.sendgrid_api_key = Some(decrypt(&key, &encryption_key)?);
            }
            if let Some(key) = self.google_ga_key.clone() {
                self.google_ga_key = Some(decrypt(&key, &encryption_key)?);
            }
            if let Some(key) = self.facebook_pixel_key.clone() {
                self.facebook_pixel_key = Some(decrypt(&key, &encryption_key)?);
            }
            if let Some(key) = self.google_ads_conversion_id.clone() {
                self.google_ads_conversion_id = Some(decrypt(&key, &encryption_key)?);
            }
            if let Some(key) = self.globee_api_key.clone() {
                self.globee_api_key = Some(decrypt(&key, &encryption_key)?);
            }
        }

        Ok(())
    }

    pub fn for_display(&self, conn: &PgConnection) -> Result<DisplayOrganization, DatabaseError> {
        Ok(DisplayOrganization {
            id: self.id,
            name: self.name.clone(),
            address: self.address.clone(),
            city: self.city.clone(),
            state: self.state.clone(),
            country: self.country.clone(),
            postal_code: self.postal_code.clone(),
            phone: self.phone.clone(),
            timezone: self.timezone.clone(),
            slug: Slug::primary_slug(self.id, Tables::Organizations, conn)?.slug,
        })
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct DisplayOrganization {
    pub id: Uuid,
    pub name: String,
    pub address: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
    pub postal_code: Option<String>,
    pub phone: Option<String>,
    pub timezone: Option<String>,
    pub slug: String,
}
