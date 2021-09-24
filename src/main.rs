use crate::{bot::Bot, config::Config, game::Game};
use std::env;
use tokio::sync::mpsc;

mod bot;
mod config;
mod game;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    let config = match Config::parse(args) {
        Ok(config) => config,
        Err(e) => {
            println!("{}", e);
            return;
        }
    };

    let (bot_to_game_sender, bot_to_game_receiver) = mpsc::channel(2);
    let (game_to_bot_sender, game_to_bot_receiver) = mpsc::channel(2);

    let bot = Bot::init(&config).unwrap();
    let bot_output = bot.clone();
    let bot_input = bot.clone();

    tokio::spawn(async move { bot_output.handle_output(game_to_bot_receiver).await });
    tokio::spawn(async move { bot_input.handle_input(bot_to_game_sender).await });

    match Game::start(game_to_bot_sender, bot_to_game_receiver).await {
        Ok(()) => (),
        Err(e) => {
            println!("{}", e);
            return;
        }
    }
}
