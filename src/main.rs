use crate::{bot::BotInstance, config::Config, mindustry::Game};
use std::env;
use tokio::sync::mpsc;

mod bot;
mod command;
mod config;
mod mindustry;

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

    let (input_sender, input_receiver) = mpsc::channel(2);
    let (output_sender, output_receiver) = mpsc::channel(2);

    let bot_output = BotInstance::init(&config).unwrap();
    let bot_input = bot_output.clone();

    tokio::spawn(async move { bot_output.handle_output(output_receiver).await });
    tokio::spawn(async move { bot_input.handle_input(input_sender).await });

    match Game::start(output_sender, input_receiver).await {
        Ok(()) => (),
        Err(e) => {
            println!("{}", e);
            return;
        }
    }
}
