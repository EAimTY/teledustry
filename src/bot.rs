use crate::config::Config;
use futures_util::{future::BoxFuture, Future};
use std::{
    collections::{HashMap, HashSet},
    convert::TryFrom,
    sync::Arc,
};
use tgbot::{
    longpoll::LongPoll,
    methods::SendMessage,
    types::{Command as BotCommand, Update, UpdateKind},
    Api, Config as ApiConfig, UpdateHandler,
};
use tokio::sync::{mpsc, Mutex};

#[derive(Clone)]
pub struct BotInstance {
    api: Api,
    webhook: u16,
    context: Context,
}

impl BotInstance {
    pub fn init(config: &Config) -> Result<Self, String> {
        let mut api_config = ApiConfig::new(config.token.clone());

        if let Some(proxy) = config.proxy.clone() {
            api_config = api_config.proxy(proxy).or_else(|e| Err(e.to_string()))?;
        }

        let api = Api::new(api_config).or_else(|e| Err(e.to_string()))?;

        Ok(Self {
            api,
            webhook: 0,
            context: Context::init(),
        })
    }

    pub async fn handle_output(self, mut game_to_bot_receiver: mpsc::Receiver<String>) {
        while let Some(output) = game_to_bot_receiver.recv().await {
            let output_chat = (*self.context.output_chat.lock().await).clone();

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
            BotUpdateHandler::new(self.api, bot_to_game_sender, self.context),
        )
        .run()
        .await;
    }
}

type CommandOutput<T> = Box<dyn Future<Output = T>>;
type Command<T> = Box<dyn Fn(BotUpdateHandler) -> CommandOutput<T> + Send + Sync>;

struct Context {
    commands: Arc<HashMap<&'static str, Command<()>>>,
    output_chat: Arc<Mutex<HashSet<i64>>>,
}

impl Context {
    fn init() -> Self {
        let mut commands = HashMap::new();

        fn func(handler: BotUpdateHandler) -> CommandOutput<()> {
            Box::new(async move { todo!() })
        }
        commands.insert("key", Box::new(func) as Command<()>);

        Self {
            commands: Arc::new(commands),
            output_chat: Arc::new(Mutex::new(HashSet::new())),
        }
    }
}

impl Clone for Context {
    fn clone(&self) -> Self {
        Self {
            commands: Arc::clone(&self.commands),
            output_chat: Arc::clone(&self.output_chat),
        }
    }
}

struct BotUpdateHandler {
    api: Api,
    bot_to_game_sender: mpsc::Sender<String>,
    context: Context,
}

impl BotUpdateHandler {
    fn new(api: Api, bot_to_game_sender: mpsc::Sender<String>, context: Context) -> Self {
        Self {
            api,
            bot_to_game_sender,
            context,
        }
    }
}

impl UpdateHandler for BotUpdateHandler {
    type Future = BoxFuture<'static, ()>;

    fn handle(&self, update: Update) -> Self::Future {
        let handler = self.clone();

        Box::pin(async move {
            if let UpdateKind::Message(message) = update.kind {
                let chat_id = message.get_chat_id();

                if let Ok(command) = BotCommand::try_from(message) { /*
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
                     }*/
                }
            }
        })
    }
}
