use chrono::NaiveDateTime;
use diesel;
use diesel::expression::dsl;
use diesel::prelude::*;
use diesel::sql_types::{Array, Bool, Uuid as dUuid};
use models::*;
use schema::{artist_genres, artists, event_artists, events, genres, organization_users, organizations};
use utils::errors::*;
use utils::pagination::*;
use utils::text;
use uuid::Uuid;
use validator::Validate;
use validators;

#[derive(Associations, Deserialize, Identifiable, Queryable, Serialize, Debug, PartialEq, Clone)]
pub struct Artist {
    pub id: Uuid,
    pub organization_id: Option<Uuid>,
    pub is_private: bool,
    pub name: String,
    pub bio: String,
    pub image_url: Option<String>,
    pub thumb_image_url: Option<String>,
    pub website_url: Option<String>,
    pub youtube_video_urls: Vec<String>,
    pub facebook_username: Option<String>,
    pub instagram_username: Option<String>,
    pub snapchat_username: Option<String>,
    pub soundcloud_username: Option<String>,
    pub bandcamp_username: Option<String>,
    pub spotify_id: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub other_image_urls: Option<Vec<String>>,
    pub main_genre_id: Option<Uuid>,
}

#[derive(Insertable, Default, Deserialize, Validate)]
#[table_name = "artists"]
pub struct NewArtist {
    pub organization_id: Option<Uuid>,
    pub name: String,
    pub bio: String,
    #[validate(url(message = "Image URL is invalid"))]
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub image_url: Option<String>,
    #[validate(url(message = "Thumb image URL is invalid"))]
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub thumb_image_url: Option<String>,
    #[validate(url(message = "Website URL is invalid"))]
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub website_url: Option<String>,
    #[validate(custom = "validators::validate_urls")]
    pub youtube_video_urls: Option<Vec<String>>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub facebook_username: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub instagram_username: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub snapchat_username: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub soundcloud_username: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub bandcamp_username: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub spotify_id: Option<String>,
    pub other_image_urls: Option<Vec<String>>,
}

impl NewArtist {
    pub fn commit(&self, conn: &PgConnection) -> Result<Artist, DatabaseError> {
        self.validate()?;
        DatabaseError::wrap(
            ErrorCode::InsertError,
            "Could not create new artist",
            diesel::insert_into(artists::table).values(self).get_result(conn),
        )
    }

    pub fn merge(&mut self, new_artist: NewArtist) {
        if self.bio == "" {
            self.bio = new_artist.bio;
        }
        if let None = self.image_url {
            self.image_url = new_artist.image_url;
        }
        if let None = self.thumb_image_url {
            self.thumb_image_url = new_artist.thumb_image_url;
        }
        if let None = self.website_url {
            self.website_url = new_artist.website_url;
        }
        if let None = self.youtube_video_urls {
            self.youtube_video_urls = new_artist.youtube_video_urls;
        }
        if let None = self.facebook_username {
            self.facebook_username = new_artist.facebook_username;
        }
        if let None = self.instagram_username {
            self.instagram_username = new_artist.instagram_username;
        }
        if let None = self.snapchat_username {
            self.snapchat_username = new_artist.snapchat_username;
        }
        if let None = self.soundcloud_username {
            self.soundcloud_username = new_artist.soundcloud_username;
        }
        if let None = self.bandcamp_username {
            self.bandcamp_username = new_artist.bandcamp_username;
        }
        if let None = self.spotify_id {
            self.spotify_id = new_artist.spotify_id;
        }
    }
}

impl Artist {
    pub fn find_spotify_linked_artists(conn: &PgConnection) -> Result<Vec<Artist>, DatabaseError> {
        artists::table
            .filter(artists::spotify_id.is_not_null())
            .order_by(artists::name)
            .select(artists::all_columns)
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Unable to load artists with linked spotify ids")
    }

    pub fn events(&self, conn: &PgConnection) -> Result<Vec<Event>, DatabaseError> {
        events::table
            .inner_join(event_artists::table.on(event_artists::event_id.eq(events::id)))
            .filter(event_artists::artist_id.eq(self.id))
            .filter(events::deleted_at.is_null())
            .order_by(events::name)
            .select(events::all_columns)
            .load::<EventData>(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load events for artist")
            .map(EventData::vec_into_events)
    }

    pub fn genres(&self, conn: &PgConnection) -> Result<Vec<String>, DatabaseError> {
        genres::table
            .inner_join(artist_genres::table.on(artist_genres::genre_id.eq(genres::id)))
            .filter(artist_genres::artist_id.eq(self.id))
            .select(genres::name)
            .order_by(genres::name)
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not get genres for artist")
    }

    pub fn set_genres(
        &self,
        genres: &Vec<String>,
        user_id: Option<Uuid>,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        let genre_ids = Genre::find_or_create(genres, conn)?;

        let query = r#"
            INSERT INTO artist_genres (artist_id, genre_id)
            SELECT DISTINCT $1 as artist_id, g.id as genre_id
            FROM genres g
            WHERE g.id = ANY($2)
            AND g.id not in (select genre_id from artist_genres where artist_id = $1);
        "#;
        diesel::sql_query(query)
            .bind::<dUuid, _>(self.id)
            .bind::<Array<dUuid>, _>(genre_ids.clone())
            .execute(conn)
            .to_db_error(ErrorCode::QueryError, "Could not set genres")?;

        let query = r#"
            DELETE FROM artist_genres ag
            WHERE ag.artist_id = $1
            AND NOT (ag.genre_id = ANY($2));
        "#;
        diesel::sql_query(query)
            .bind::<dUuid, _>(self.id)
            .bind::<Array<dUuid>, _>(genre_ids)
            .execute(conn)
            .to_db_error(ErrorCode::QueryError, "Could not clear old genres")?;

        DomainEvent::create(
            DomainEventTypes::GenresUpdated,
            "Artist genres updated".to_string(),
            Tables::Artists,
            Some(self.id),
            user_id,
            Some(json!({ "genres": genres })),
        )
        .commit(conn)?;

        Ok(())
    }

    pub fn create(name: &str, organization_id: Option<Uuid>, bio: &str, website_url: &str) -> NewArtist {
        NewArtist {
            organization_id,
            name: String::from(name),
            bio: String::from(bio),
            website_url: Some(String::from(website_url)),
            ..Default::default()
        }
    }

    pub fn search(
        user: &Option<User>,
        query_filter: Option<String>,
        paging: &Paging,
        conn: &PgConnection,
    ) -> Result<(Vec<DisplayArtist>, i64), DatabaseError> {
        let query_like = match query_filter {
            Some(n) => format!("%{}%", text::escape_control_chars(&n)),
            None => "%".to_string(),
        };
        //
        let (artists, count): (Vec<Artist>, i64) = match user {
            Some(u) => artists::table
                .left_join(
                    organization_users::table.on(artists::organization_id
                        .eq(organization_users::organization_id.nullable())
                        .and(organization_users::user_id.eq(u.id))),
                )
                .left_join(organizations::table.on(artists::organization_id.eq(organizations::id.nullable())))
                .filter(
                    organization_users::user_id
                        .eq(u.id)
                        .or(artists::is_private.eq(false))
                        .or(dsl::sql("TRUE = ").bind::<Bool, _>(u.is_admin())),
                )
                .filter(artists::name.ilike(query_like.clone()))
                .order_by(artists::name)
                .select(artists::all_columns)
                .paginate(paging.page as i64)
                .per_page(paging.limit as i64)
                .load_and_count_pages(conn),

            None => artists::table
                .filter(artists::is_private.eq(false))
                .filter(artists::name.ilike(query_like.clone()))
                .order_by(artists::name)
                .select(artists::all_columns)
                .paginate(paging.page as i64)
                .per_page(paging.limit as i64)
                .load_and_count_pages(conn),
        }
        .to_db_error(ErrorCode::QueryError, "Unable to search artists")?;

        let artist_ids: Vec<Uuid> = artists.iter().map(|a| a.id).collect();
        let genre_mapping = Genre::find_by_artist_ids(&artist_ids, conn)?;
        let mut result = vec![];
        for artist in artists {
            let genres = genre_mapping
                .get(&artist.id)
                .map(|m| m.into_iter().map(|g| g.name.clone()).collect())
                .unwrap_or(Vec::new());
            result.push(DisplayArtist::from(artist, genres, conn)?);
        }

        Ok((result, count))
    }

    pub fn all(user: Option<&User>, conn: &PgConnection) -> Result<Vec<DisplayArtist>, DatabaseError> {
        let artists: Vec<Artist> = match user {
            Some(u) => artists::table
                .left_join(
                    organization_users::table.on(artists::organization_id
                        .eq(organization_users::organization_id.nullable())
                        .and(organization_users::user_id.eq(u.id))),
                )
                .left_join(organizations::table.on(artists::organization_id.eq(organizations::id.nullable())))
                .filter(
                    organization_users::user_id
                        .eq(u.id)
                        .or(artists::is_private.eq(false))
                        .or(dsl::sql("TRUE = ").bind::<Bool, _>(u.is_admin())),
                )
                .order_by(artists::name)
                .select(artists::all_columns)
                .load(conn),
            None => artists::table
                .filter(artists::is_private.eq(false))
                .order_by(artists::name)
                .select(artists::all_columns)
                .load(conn),
        }
        .to_db_error(ErrorCode::QueryError, "Unable to load all artists")?;

        let artist_ids: Vec<Uuid> = artists.iter().map(|a| a.id).collect();
        let genre_mapping = Genre::find_by_artist_ids(&artist_ids, conn)?;

        let mut result = vec![];
        for artist in artists {
            let genres = genre_mapping
                .get(&artist.id)
                .map(|m| m.into_iter().map(|g| g.name.clone()).collect())
                .unwrap_or(Vec::new());
            result.push(DisplayArtist::from(artist, genres, conn)?);
        }

        Ok(result)
    }

    pub fn for_display(self, conn: &PgConnection) -> Result<DisplayArtist, DatabaseError> {
        let genres = self.genres(conn)?;
        DisplayArtist::from(self, genres, conn)
    }

    pub fn find(id: &Uuid, conn: &PgConnection) -> Result<Artist, DatabaseError> {
        artists::table
            .find(id)
            .first::<Artist>(conn)
            .to_db_error(ErrorCode::QueryError, "Error loading artist")
    }

    pub fn find_for_organization(
        user: Option<&User>,
        organization_id: Uuid,
        conn: &PgConnection,
    ) -> Result<Vec<DisplayArtist>, DatabaseError> {
        let artists: Vec<Artist> = match user {
            Some(u) => artists::table
                .left_join(
                    organization_users::table.on(artists::organization_id
                        .eq(organization_users::organization_id.nullable())
                        .and(organization_users::user_id.eq(u.id))),
                )
                .left_join(organizations::table.on(artists::organization_id.eq(organizations::id.nullable())))
                .filter(
                    organization_users::user_id
                        .eq(u.id)
                        .or(artists::is_private.eq(false))
                        .or(dsl::sql("TRUE = ").bind::<Bool, _>(u.is_admin())),
                )
                .filter(artists::organization_id.eq(organization_id))
                .order_by(artists::name)
                .select(artists::all_columns)
                .load(conn),
            None => artists::table
                .filter(artists::is_private.eq(false))
                .filter(artists::organization_id.eq(organization_id))
                .order_by(artists::name)
                .select(artists::all_columns)
                .load(conn),
        }
        .to_db_error(ErrorCode::QueryError, "Unable to load all artists")?;

        let artist_ids: Vec<Uuid> = artists.iter().map(|a| a.id).collect();
        let genre_mapping = Genre::find_by_artist_ids(&artist_ids, conn)?;
        let mut result = vec![];
        for artist in artists {
            let genres = genre_mapping
                .get(&artist.id)
                .map(|m| m.into_iter().map(|g| g.name.clone()).collect())
                .unwrap_or(Vec::new());
            result.push(DisplayArtist::from(artist, genres, conn)?);
        }

        Ok(result)
    }

    pub fn update(&self, attributes: &ArtistEditableAttributes, conn: &PgConnection) -> Result<Artist, DatabaseError> {
        attributes.validate()?;
        let query = diesel::update(self).set((attributes, artists::updated_at.eq(dsl::now)));

        DatabaseError::wrap(ErrorCode::UpdateError, "Error updating artist", query.get_result(conn))
    }

    pub fn destroy(&self, conn: &PgConnection) -> Result<usize, DatabaseError> {
        DatabaseError::wrap(
            ErrorCode::DeleteError,
            "Failed to destroy artist record",
            diesel::delete(self).execute(conn),
        )
    }

    pub fn organization(&self, conn: &PgConnection) -> Result<Option<Organization>, DatabaseError> {
        match self.organization_id {
            Some(organization_id) => Ok(Some(Organization::find(organization_id, conn)?)),
            None => Ok(None),
        }
    }

    pub fn set_privacy(&self, is_private: bool, conn: &PgConnection) -> Result<Artist, DatabaseError> {
        DatabaseError::wrap(
            ErrorCode::UpdateError,
            "Could not update is_private for artist",
            diesel::update(self)
                .set((artists::is_private.eq(is_private), artists::updated_at.eq(dsl::now)))
                .get_result(conn),
        )
    }
}

#[derive(AsChangeset, Default, Deserialize, Validate)]
#[table_name = "artists"]
pub struct ArtistEditableAttributes {
    pub name: Option<String>,
    pub bio: Option<String>,
    #[validate(url(message = "Image URL is invalid"))]
    #[serde(default, deserialize_with = "double_option_deserialize_unless_blank")]
    pub image_url: Option<Option<String>>,
    #[validate(url(message = "Thumb image URL is invalid"))]
    #[serde(default, deserialize_with = "double_option_deserialize_unless_blank")]
    pub thumb_image_url: Option<Option<String>>,
    #[validate(url(message = "Website URL is invalid"))]
    #[serde(default, deserialize_with = "double_option_deserialize_unless_blank")]
    pub website_url: Option<Option<String>>,
    #[validate(custom = "validators::validate_urls")]
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
    pub main_genre_id: Option<Option<Uuid>>,
}

impl ArtistEditableAttributes {
    pub fn new() -> ArtistEditableAttributes {
        Default::default()
    }
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
pub struct DisplayArtist {
    pub id: Uuid,
    pub organization_id: Option<Uuid>,
    pub is_private: bool,
    pub name: String,
    pub bio: String,
    pub image_url: Option<String>,
    pub thumb_image_url: Option<String>,
    pub website_url: Option<String>,
    pub youtube_video_urls: Vec<String>,
    pub facebook_username: Option<String>,
    pub instagram_username: Option<String>,
    pub snapchat_username: Option<String>,
    pub soundcloud_username: Option<String>,
    pub bandcamp_username: Option<String>,
    pub spotify_id: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub other_image_urls: Option<Vec<String>>,
    pub main_genre_id: Option<Uuid>,
    pub main_genre: Option<String>,
    pub genres: Vec<String>,
}

impl DisplayArtist {
    fn from(artist: Artist, genres: Vec<String>, conn: &PgConnection) -> Result<Self, DatabaseError> {
        let main_genre = if let Some(main_genre_id) = artist.main_genre_id {
            Some(Genre::find(main_genre_id, conn)?.name)
        } else {
            None
        };

        Ok(DisplayArtist {
            id: artist.id,
            organization_id: artist.organization_id,
            is_private: artist.is_private,
            name: artist.name,
            bio: artist.bio,
            image_url: artist.image_url,
            thumb_image_url: artist.thumb_image_url,
            website_url: artist.website_url,
            youtube_video_urls: artist.youtube_video_urls,
            facebook_username: artist.facebook_username,
            instagram_username: artist.instagram_username,
            snapchat_username: artist.snapchat_username,
            soundcloud_username: artist.soundcloud_username,
            bandcamp_username: artist.bandcamp_username,
            spotify_id: artist.spotify_id,
            created_at: artist.created_at,
            updated_at: artist.updated_at,
            other_image_urls: artist.other_image_urls,
            main_genre_id: artist.main_genre_id,
            main_genre,
            genres,
        })
    }
}
