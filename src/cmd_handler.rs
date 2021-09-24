use futures_util::future::BoxFuture;
use std::{collections::HashSet, convert::TryFrom, sync::Arc};
use tgbot::{
    methods::SendMessage,
    types::{Command, Update, UpdateKind},
    Api, UpdateHandler,
};
use tokio::sync::{mpsc, Mutex};

pub struct CmdHandler {
    api: Api,
    bot_to_game_sender: mpsc::Sender<String>,
    output_chat: Arc<Mutex<HashSet<i64>>>,
}

impl CmdHandler {
    pub fn new(
        api: Api,
        bot_to_game_sender: mpsc::Sender<String>,
        output_chat: Arc<Mutex<HashSet<i64>>>,
    ) -> Self {
        Self {
            api,
            bot_to_game_sender,
            output_chat,
        }
    }
}

impl Clone for CmdHandler {
    fn clone(&self) -> Self {
        Self {
            api: self.api.clone(),
            bot_to_game_sender: self.bot_to_game_sender.clone(),
            output_chat: Arc::clone(&self.output_chat),
        }
    }
}

impl UpdateHandler for CmdHandler {
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

                        "/help" => {
                            handler
                                .bot_to_game_sender
                                .send(String::from("help"))
                                .await
                                .unwrap();
                        }

                        _ => {}
                    }
                }
            }
        })
    }
}
