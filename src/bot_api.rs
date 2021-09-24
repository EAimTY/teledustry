use crate::{cmd_handler::CmdHandler, config::Config};
use std::{collections::HashSet, sync::Arc};
use tgbot::{longpoll::LongPoll, methods::SendMessage, webhook, Api, Config as ApiConfig};
use tokio::sync::{mpsc, Mutex};

pub struct Bot {
    api: Api,
    webhook: u16,
    output_chat: Arc<Mutex<HashSet<i64>>>,
}

impl Bot {
    pub fn init(config: &Config) -> Result<Self, String> {
        let mut api_config = ApiConfig::new(config.token.clone());

        if let Some(proxy) = config.proxy.clone() {
            api_config = api_config.proxy(proxy).or_else(|e| Err(e.to_string()))?;
        }

        let api = Api::new(api_config).or_else(|e| Err(e.to_string()))?;

        Ok(Self {
            api,
            webhook: 0,
            output_chat: Arc::new(Mutex::new(HashSet::new())),
        })
    }

    pub async fn handle_output(self, mut game_to_bot_receiver: mpsc::Receiver<String>) {
        while let Some(output) = game_to_bot_receiver.recv().await {
            let output_chat = (*self.output_chat.lock().await).clone();

            for chat_id in output_chat {
                self.api
                    .execute(SendMessage::new(chat_id, output.clone()))
                    .await
                    .unwrap();
            }
        }
    }

    pub async fn handle_input(self, bot_to_game_sender: mpsc::Sender<String>) {
        LongPoll::new(
            self.api.clone(),
            CmdHandler::new(self.api, bot_to_game_sender, self.output_chat),
        )
        .run()
        .await;
    }
}

impl Clone for Bot {
    fn clone(&self) -> Self {
        Self {
            api: self.api.clone(),
            webhook: self.webhook,
            output_chat: Arc::clone(&self.output_chat),
        }
    }
}
