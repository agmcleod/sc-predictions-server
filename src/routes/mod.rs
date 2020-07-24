use actix_web::web;

pub mod games;
pub mod questions;

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api").service(
            web::scope("/questions")
                .route("", web::get().to(questions::get_all))
                .service(
                    web::scope("/games")
                        .route("", web::post().to(games::create))
                        .service(web::scope("/join").route("", web::post().to(games::join))),
                ),
        ),
    );
}
