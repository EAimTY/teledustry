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
            eprintln!("{}", e);
            return;
        }
    };

    let (output_sender, output_receiver) = mpsc::channel(2);
    let (input_sender, input_receiver) = mpsc::channel(2);

    let bot_output_handler = match BotInstance::init(&config) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("{}", e);
            return;
        }
    };

    let bot_input_handler = bot_output_handler.clone();

    let handle_bot_output = bot_output_handler.handle_output(output_receiver).await;
    let handle_bot_input = bot_input_handler.handle_input(input_sender).await;

    match Game::spawn(output_sender, input_receiver).await {
        Ok(_) => (),
        Err(e) => {
            eprintln!("{}", e);
            return;
        }
    }

    match tokio::try_join!(handle_bot_output, handle_bot_input) {
        Ok(_) => (),
        Err(e) => {
            eprintln!("{}", e);
            return;
        }
    }
}
