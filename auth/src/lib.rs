use std::env;

use actix_identity::{Identity, IdentityPolicy, IdentityService};
use actix_web::{
    dev::{ServiceRequest, ServiceResponse},
    error,
};
use chrono::{Duration, Utc};
use futures_util::future::{ok, Ready};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use errors::Error;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub enum Role {
    Player,
    Owner,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PrivateClaim {
    pub id: i32,
    pub user_name: String,
    pub game_id: i32,
    pub role: Role,
    exp: i64,
}

impl PrivateClaim {
    pub fn new(id: i32, user_name: String, game_id: i32, role: Role) -> Self {
        PrivateClaim {
            id,
            user_name,
            game_id,
            role,
            exp: (Utc::now() + Duration::hours(3)).timestamp(),
        }
    }

    pub fn set_exp(&mut self, exp: i64) {
        self.exp = exp;
    }
}

pub struct AuthHeaderIdentityPolicy;

impl AuthHeaderIdentityPolicy {
    fn new() -> Self {
        AuthHeaderIdentityPolicy {}
    }
}

impl IdentityPolicy for AuthHeaderIdentityPolicy {
    type Future = Ready<Result<Option<String>, error::Error>>;
    type ResponseFuture = Ready<Result<(), error::Error>>;

    fn from_request(&self, request: &mut ServiceRequest) -> Self::Future {
        let mut token: Option<String> = None;
        let auth_token = request.headers().get("Authorization");

        if let Some(auth_token) = auth_token {
            let token_string = auth_token.to_str();
            if token_string.is_ok() {
                token = Some(String::from(token_string.unwrap()).replace("Bearer ", ""));
            }
        }

        ok(token)
    }

    fn to_response<B>(
        &self,
        _identity: Option<String>,
        _changed: bool,
        _response: &mut ServiceResponse<B>,
    ) -> Self::ResponseFuture {
        ok(())
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

pub fn get_identity_service() -> IdentityService<AuthHeaderIdentityPolicy> {
    IdentityService::new(AuthHeaderIdentityPolicy::new())
}

pub fn get_claim_from_identity(id: Identity) -> Result<(PrivateClaim, String), Error> {
    if let Some(token) = id.identity() {
        let claim = decode_jwt(&token)?;
        return Ok((claim, token));
    }
    Err(Error::Unauthorized)
}

pub fn identity_matches_game_id(id: Identity, game_id: i32) -> Result<(), Error> {
    let (claim, _) = get_claim_from_identity(id)?;
    if game_id != claim.game_id {
        return Err(Error::Forbidden);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{create_jwt, decode_jwt, PrivateClaim, Role};

    #[test]
    fn test_creates_jwt() {
        let private_claim = PrivateClaim::new(1, "agmcleod".to_string(), 1, Role::Player);
        let jwt = create_jwt(private_claim);
        assert!(jwt.is_ok());
    }

    #[test]
    fn test_decodes_jwt() {
        let private_claim = PrivateClaim::new(1, "agmcleod".to_string(), 2, Role::Owner);
        let jwt = create_jwt(private_claim.clone()).unwrap();
        let decoded = decode_jwt(&jwt).unwrap();
        assert_eq!(private_claim, decoded);
    }
}
