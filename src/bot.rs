use crate::config::Config;
use futures_util::future::BoxFuture;
use std::{collections::HashSet, convert::TryFrom, sync::Arc};
use tgbot::{
    longpoll::LongPoll,
    methods::SendMessage,
    types::{Command, Update, UpdateKind},
    webhook, Api, Config as ApiConfig, UpdateHandler,
};
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
        while let Some(s) = game_to_bot_receiver.recv().await {
            print!("{}", s);
        }
    }

    pub async fn handle_input(self, bot_to_game_sender: mpsc::Sender<String>) {
        LongPoll::new(
            self.api.clone(),
            Handler {
                api: self.api,
                bot_to_game_sender,
                output_chat: self.output_chat,
            },
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

struct Handler {
    api: Api,
    bot_to_game_sender: mpsc::Sender<String>,
    output_chat: Arc<Mutex<HashSet<i64>>>,
}

impl UpdateHandler for Handler {
    type Future = BoxFuture<'static, ()>;

    fn handle(&self, update: Update) -> Self::Future {
        let handler = self.clone();

        Box::pin(async move {
            if let UpdateKind::Message(message) = update.kind {
                let chat_id = message.get_chat_id();

                if let Ok(command) = Command::try_from(message) {
                    match command.get_name() {
                        "/output_here" => {
                            let mut output_chat = handler.output_chat.lock().await;
                            let send_message;

                            if output_chat.insert(chat_id) {
                                send_message =
                                    SendMessage::new(chat_id, "OKay, I will send the output here");
                            } else {
                                send_message = SendMessage::new(
                                    chat_id,
                                    "This chat is already in the output chat list",
                                );
                            }

                            handler.api.execute(send_message).await.unwrap();
                        }

                        "/stop_output" => {
                            let mut output_chat = handler.output_chat.lock().await;
                            let send_message;

                            if output_chat.remove(&chat_id) {
                                send_message = SendMessage::new(
                                    chat_id,
                                    "Okay, I will not send the output here anymore",
                                );
                            } else {
                                send_message = SendMessage::new(
                                    chat_id,
                                    "This chat is not in the output chat list",
                                );
                            }

                            handler.api.execute(send_message).await.unwrap();
                        }
                        _ => {}
                    }
                    // bot_to_game_sender.send(text.data.clone()).await.unwrap();
                }
            }
        })
    }
}

impl Clone for Handler {
    fn clone(&self) -> Self {
        Self {
            api: self.api.clone(),
            bot_to_game_sender: self.bot_to_game_sender.clone(),
            output_chat: Arc::clone(&self.output_chat),
        }
    }
}
