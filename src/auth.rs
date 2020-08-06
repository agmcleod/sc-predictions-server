use std::env;

use actix_identity::{CookieIdentityPolicy, Identity, IdentityService};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::errors::Error;

pub const SESSION_NAME: &str = "auth";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PrivateClaim {
    pub id: i32,
    pub user_name: String,
    pub game_id: i32,
    exp: i64,
}

impl PrivateClaim {
    pub fn new(id: i32, user_name: String, game_id: i32) -> Self {
        PrivateClaim {
            id,
            user_name,
            game_id,
            exp: (Utc::now() + Duration::hours(3)).timestamp(),
        }
    }
}

pub fn create_jwt(private_claim: PrivateClaim) -> Result<String, Error> {
    let encoding_key = EncodingKey::from_secret(&env::var("JWT_KEY").unwrap().as_ref());
    encode(&Header::default(), &private_claim, &encoding_key)
        .map_err(|e| Error::CannotEncodeJwtToken(e.to_string()))
}

pub fn decode_jwt(token: &str) -> Result<PrivateClaim, Error> {
    let jwt_key = env::var("JWT_KEY").unwrap();
    let decoding_key = DecodingKey::from_secret(&jwt_key.as_ref());
    decode::<PrivateClaim>(token, &decoding_key, &Validation::default())
        .map(|data| data.claims)
        .map_err(|e| Error::CannotDecodeJwtToken(e.to_string()))
}

pub fn identity_matches_game_id(id: Identity, game_id: i32) -> Result<(), Error> {
    let token = id.identity().unwrap();
    let claim = decode_jwt(&token)?;
    if game_id != claim.game_id {
        return Err(Error::Forbidden);
    }

    Ok(())
}

pub fn get_identity_service() -> IdentityService<CookieIdentityPolicy> {
    IdentityService::new(
        CookieIdentityPolicy::new(&env::var("SESSION_KEY").unwrap().as_ref())
            .name(SESSION_NAME)
            .max_age_time(Duration::minutes(20))
            // allow to transmit over http
            .secure(false),
    )
}

#[cfg(test)]
mod tests {
    use crate::auth::{create_jwt, decode_jwt, PrivateClaim};

    #[test]
    fn test_creates_jwt() {
        let private_claim = PrivateClaim::new(1, "agmcleod".to_string(), 1);
        let jwt = create_jwt(private_claim);
        assert!(jwt.is_ok());
    }

    #[test]
    fn test_decodes_jwt() {
        let private_claim = PrivateClaim::new(1, "agmcleod".to_string(), 2);
        let jwt = create_jwt(private_claim.clone()).unwrap();
        let decoded = decode_jwt(&jwt).unwrap();
        assert_eq!(private_claim, decoded);
    }
}
