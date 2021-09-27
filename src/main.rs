use crate::{bot::BotInstance, config::Config, game::Game};
use std::env;
use tokio::sync::mpsc;

mod bot;
mod command;
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

    let (output_sender, output_receiver) = mpsc::channel(2);
    let (input_sender, input_receiver) = mpsc::channel(2);

    let bot_output_handler = BotInstance::init(&config).unwrap();
    let bot_input_handler = bot_output_handler.clone();

    let handle_bot_output = bot_output_handler.handle_output(output_receiver).await;
    let handle_bot_input = bot_input_handler.handle_input(input_sender).await;

    Game::spawn(output_sender, input_receiver).await.unwrap();

    tokio::try_join!(handle_bot_output, handle_bot_input).unwrap();
}
