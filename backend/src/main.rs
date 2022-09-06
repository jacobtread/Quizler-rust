extern crate core;

pub mod socket;
pub mod game;
pub mod packets;
mod tools;
mod gamev2;

use actix::{Addr};
use actix_web::{
    App, HttpRequest, HttpResponse, HttpServer, Responder,
    web::{Data, get, Payload},
};
use actix_web_actors::ws::{start};
use crate::game::GameManager;
use crate::socket::Connection;
use simplelog::{ColorChoice, Config, LevelFilter, TerminalMode, TermLogger};

const APP_INDEX: &str = include_str!("../public/index.html");

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    const INTRO: &'static str =
        "   __         __       ___  __  \n\
              /  \\ |  | |  / |    |__  |__) \n\
                \\__X \\__/ | /_ |___ |___ |  \\   by Jacobtread\n\n";
    let port: u16 = option_env!("PORT").unwrap_or("8080").parse::<u16>().expect("expected port to be u16 number");
    println!(
        concat!(
        "{} Version ",
        env!("CARGO_PKG_VERSION"),
        "    Server started on http://localhost:{}\n"
        ), INTRO, port
    );

    #[cfg(debug_assertions)]
    {
        TermLogger::init(
            LevelFilter::Debug,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        );
    }

    #[cfg(not(debug_assertions))]
    {
        TermLogger::init(
            LevelFilter::Info,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        );
    }

    let manager = GameManager::new();
    HttpServer::new(move || {
        App::new()
            .app_data(manager.clone())
            .route("/ws", get().to(|req: HttpRequest, stream: Payload, manager: Data<Addr<GameManager>>| async move {
                start(Connection::new(manager.get_ref().clone()), &req, stream)
            }))
            .route("/{_:.*}", get().to(|| async {
                HttpResponse::Ok().content_type("text/html").body(APP_INDEX)
            }))
    })
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}