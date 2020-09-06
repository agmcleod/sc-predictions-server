use actix_identity::Identity;
use actix_web::{
    web::{block, Data, HttpResponse, Json},
    Result,
};

use auth::{get_claim_from_identity, Role};
use db::{get_conn, models::Round, PgPool};
use errors::Error;

pub async fn lock_round(id: Identity, pool: Data<PgPool>) -> Result<HttpResponse, Error> {
    let (claim, _) = get_claim_from_identity(id)?;
    if claim.role != Role::Owner {
        return Err(Error::Forbidden);
    }

    block(move || {
        let conn = get_conn(&pool)?;
        let round = Round::get_active_round_by_game_id(&conn, claim.game_id)?;
        Round::lock(&conn, round.id)?;
        Ok(())
    })
    .await?;

    Ok(HttpResponse::Ok().json(()))
}

#[cfg(test)]
mod tests {
    use diesel::{self, ExpressionMethods, QueryDsl, RunQueryDsl};

    use auth::{create_jwt, PrivateClaim, Role};
    use db::{
        get_conn,
        models::{Game, Round},
        new_pool,
        schema::{games, rounds},
    };

    use crate::tests::helpers::tests::test_post;

    #[derive(Insertable)]
    #[table_name = "games"]
    struct NewGame {
        locked: bool,
        slug: Option<String>,
    }

    #[derive(Insertable)]
    #[table_name = "rounds"]
    pub struct NewRound {
        player_one: String,
        player_two: String,
        game_id: i32,
        locked: bool,
    }

    #[actix_rt::test]
    async fn test_lock_current_round() {
        let pool = new_pool();
        let conn = get_conn(&pool).unwrap();

        let game: Game = diesel::insert_into(games::table)
            .values(NewGame {
                locked: true,
                slug: Some("abc123".to_string()),
            })
            .get_result(&conn)
            .unwrap();

        diesel::insert_into(rounds::table)
            .values(vec![
                NewRound {
                    player_one: "maru".to_string(),
                    player_two: "zest".to_string(),
                    game_id: game.id,
                    locked: false,
                },
                NewRound {
                    player_one: "serral".to_string(),
                    player_two: "ty".to_string(),
                    game_id: game.id,
                    locked: true,
                },
            ])
            .execute(&conn)
            .unwrap();

        let claim = PrivateClaim::new(game.id, game.slug.unwrap(), game.id, Role::Owner);
        let token = create_jwt(claim).unwrap();

        let res: (u16, ()) = test_post("/api/rounds/lock", (), Some(token)).await;

        assert_eq!(res.0, 200);
        let results: Vec<Round> = rounds::dsl::rounds
            .filter(rounds::dsl::game_id.eq(game.id))
            .get_results(&conn)
            .unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].locked, true);
        assert_eq!(results[1].locked, true);
    }
}
