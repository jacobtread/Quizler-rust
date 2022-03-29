use actix::Actor;
use actix_web_actors::ws;

use crate::game::Game;
use crate::player::Player;

pub struct Connection<'c, 'p> {
    player: Option<&'p Player<'p>>,
    host: Option<&'a Game<'a>>,
    game: Option<&'a Game<'a>>,
}

impl Actor for Connection {
    type Context = ws::WebsocketContext<Self>;
}
