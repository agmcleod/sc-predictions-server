use actix_web::web;

use crate::middleware::Auth;

pub mod games;
pub mod questions;
pub mod rounds;

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("").service(
            web::scope("/api")
                .service(web::scope("/questions").route("", web::get().to(questions::get_all)))
                .service(
                    web::scope("/games")
                        .route("", web::post().to(games::create))
                        .service(web::scope("/join").route("", web::post().to(games::join)))
                        .service(
                            web::scope("/{id}")
                                .wrap(Auth)
                                .route("", web::get().to(games::status))
                                .route("/players", web::get().to(games::get_players)),
                        ),
                )
                .service(
                    web::scope("/rounds")
                        .wrap(Auth)
                        .route("", web::post().to(rounds::create))
                        .route("/set-picks", web::post().to(rounds::save_picks))
                        .route("/lock", web::post().to(rounds::lock_round)),
                )
                .service(
                    web::scope("/current-round")
                        .wrap(Auth)
                        .route("", web::get().to(rounds::status)),
                ),
        ),
    );
}
