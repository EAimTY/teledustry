use crate::config::Config;
use futures_util::future::BoxFuture;
use std::{
    collections::{HashMap, HashSet},
    convert::TryFrom,
    sync::Arc,
};
use tgbot::{
    longpoll::LongPoll,
    methods::SendMessage,
    types::{Command, Update, UpdateKind},
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

type CommandHandler<T> =
    Box<dyn Fn(BotUpdateHandler, Command) -> BoxFuture<'static, T> + Send + Sync>;

struct Context {
    commands: Arc<HashMap<&'static str, CommandHandler<()>>>,
    output_chat: Arc<Mutex<HashSet<i64>>>,
}

impl Context {
    fn init() -> Self {
        let mut commands = HashMap::new();

        fn output(handler: BotUpdateHandler, command: Command) -> BoxFuture<'static, ()> {
            Box::pin(async move {
                let chat_id = command.get_message().get_chat_id();

                let mut output_chat = handler.context.output_chat.lock().await;

                let send_message;

                if output_chat.insert(chat_id) {
                    send_message = SendMessage::new(chat_id, "OKay, I will send the output here");
                } else {
                    send_message =
                        SendMessage::new(chat_id, "This chat is already in the output chat list");
                }

                handler.api.execute(send_message).await.unwrap();
            })
        }
        commands.insert("/output", Box::new(output) as CommandHandler<()>);

        fn stop_output(handler: BotUpdateHandler, command: Command) -> BoxFuture<'static, ()> {
            Box::pin(async move {
                let chat_id = command.get_message().get_chat_id();

                let mut output_chat = handler.context.output_chat.lock().await;

                let send_message;

                if output_chat.remove(&chat_id) {
                    send_message =
                        SendMessage::new(chat_id, "Okay, I will not send the output here anymore");
                } else {
                    send_message =
                        SendMessage::new(chat_id, "This chat is not in the output chat list");
                }

                handler.api.execute(send_message).await.unwrap();
            })
        }
        commands.insert("/stop_output", Box::new(stop_output) as CommandHandler<()>);

        fn help(handler: BotUpdateHandler, _command: Command) -> BoxFuture<'static, ()> {
            Box::pin(async move {
                handler
                    .bot_to_game_sender
                    .send(String::from("help"))
                    .await
                    .unwrap();
            })
        }
        commands.insert("/help", Box::new(help) as CommandHandler<()>);

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

impl Clone for BotUpdateHandler {
    fn clone(&self) -> Self {
        Self {
            api: self.api.clone(),
            bot_to_game_sender: self.bot_to_game_sender.clone(),
            context: self.context.clone(),
        }
    }
}

impl UpdateHandler for BotUpdateHandler {
    type Future = BoxFuture<'static, ()>;

    fn handle(&self, update: Update) -> Self::Future {
        let handler = self.clone();

        Box::pin(async move {
            if let UpdateKind::Message(message) = update.kind {
                if let Ok(command) = Command::try_from(message) {
                    let commands = Arc::clone(&handler.context.commands);

                    if let Some(command_handler) = commands.get(command.get_name()) {
                        command_handler(handler, command).await;
                    }
                }
            }
        })
    }
}
