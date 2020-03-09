use crate::auth::claims;
use crate::errors::*;
use crate::jwt::{decode, Validation};
use crate::server::GetAppState;
use actix_web::error::{self, ErrorUnauthorized};
use actix_web::HttpMessage;

pub(crate) struct Uuid;
impl Uuid {
	pub(crate) fn from_request<R>(req: &R) -> error::Result<uuid::Uuid> 
		where R: HttpMessage + GetAppState
	{
        if let Some(auth_header) = req.headers().get("Authorization") {
			let mut parts = auth_header
				.to_str()?
				.split_whitespace();
			if str::ne(parts.next().unwrap_or("None"), "Bearer") {
				return Err(ErrorUnauthorized("Authorization scheme not supported"));
			}

			match parts.next() {
				Some(access_token) => {
					let token = decode::<claims::AccessToken>(
						&access_token,
						req.state().config.token_secret.as_bytes(),
						&Validation::default(),
					)
					.map_err(|e| BigNeonError::from(e))?;
					token.claims.get_id()
                },
				None => Err(ErrorUnauthorized("No access token provided"))
			}
        } else {
            Err(ErrorUnauthorized("Missing auth token"))
        }
    }
}
