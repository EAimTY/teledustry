use crate::config::Config;
use carapax::{longpoll::LongPoll, webhook, Api, Config as ApiConfig, Dispatcher};
use tokio::sync::mpsc;

mod error;
mod handler;

pub struct Bot;

impl Bot {
    pub async fn run(
        config: Config,
        sender: mpsc::Sender<String>,
        receiver: mpsc::Receiver<String>,
    ) -> Result<(), String> {
        let mut api_config = ApiConfig::new(config.token);

        if let Some(proxy) = config.proxy {
            api_config = api_config.proxy(proxy).or_else(|e| Err(e.to_string()))?;
        }

        let api = Api::new(api_config).or_else(|e| Err(e.to_string()))?;

        let mut dispatcher = Dispatcher::new(Context {
            api: api.clone(),
            sender,
            receiver,
        });

        dispatcher.set_error_handler(error::BotErrorHandler);

        dispatcher.add_handler(handler::message_handler);

        if config.webhook_port == 0 {
            println!("Running in longpoll mode");
            LongPoll::new(api, dispatcher).run().await;
        } else {
            println!("Running at port {} in webhook mode", config.webhook_port);
            webhook::run_server(([127, 0, 0, 1], config.webhook_port), "/", dispatcher)
                .await
                .or_else(|e| Err(e.to_string()))?;
        }
        Ok(())
    }
}

pub struct Context {
    pub api: Api,
    pub sender: mpsc::Sender<String>,
    pub receiver: mpsc::Receiver<String>,
}
