use crate::config::Config;
use futures_util::future::BoxFuture;
use tgbot::{
    longpoll::LongPoll,
    types::{Update, UpdateKind},
    Api, Config as ApiConfig, UpdateHandler,
};
use tokio::sync::mpsc;

pub struct Bot;

impl Bot {
    pub fn init(config: &Config) -> Result<(Api, Api), String> {
        let mut api_config = ApiConfig::new(config.token.clone());

        if let Some(proxy) = config.proxy.clone() {
            api_config = api_config.proxy(proxy).or_else(|e| Err(e.to_string()))?;
        }

        let api_for_output = Api::new(api_config).or_else(|e| Err(e.to_string()))?;
        let api_for_input = api_for_output.clone();

        Ok((api_for_output, api_for_input))
    }

    pub async fn handle_output(api: Api, mut game_to_bot_receiver: mpsc::Receiver<String>) {
        while let Some(line) = game_to_bot_receiver.recv().await {
            print!("{}", line);
        }
    }

    pub async fn handle_input(api: Api, bot_to_game_sender: mpsc::Sender<String>) {
        LongPoll::new(api.clone(), Handler::new(api, bot_to_game_sender))
            .run()
            .await;
    }
}

struct Handler {
    api: Api,
    bot_to_game_sender: mpsc::Sender<String>,
}

impl Handler {
    fn new(api: Api, bot_to_game_sender: mpsc::Sender<String>) -> Self {
        Self {
            api,
            bot_to_game_sender,
        }
    }
}

impl UpdateHandler for Handler {
    type Future = BoxFuture<'static, ()>;

    fn handle(&self, update: Update) -> Self::Future {
        let bot_to_game_sender = self.bot_to_game_sender.clone();

        Box::pin(async move {
            if let UpdateKind::Message(message) = update.kind {
                if let Some(text) = message.get_text() {
                    bot_to_game_sender.send(text.data.clone()).await.unwrap();
                }
            }
        })
    }
}
