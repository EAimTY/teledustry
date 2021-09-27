use crate::{
    command::{CommandList, GameCommand},
    config::Config,
};
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
use tokio::sync::{mpsc, Mutex, RwLock};

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

    pub async fn handle_output(self, mut output_receiver: mpsc::Receiver<String>) {
        while let Some(output) = output_receiver.recv().await {
            if output.starts_with("Commands:\n") {
                *self.context.commands.write().await = Some(Arc::new(CommandList::init(output)));
            } else {
                let output_chat = (*self.context.output_chat.lock().await).clone();

                for chat_id in output_chat {
                    self.api
                        .execute(SendMessage::new(chat_id, output.clone()))
                        .await
                        .unwrap();
                }
            }
        }
    }

    pub async fn handle_input(self, input_sender: mpsc::Sender<String>) {
        LongPoll::new(
            self.api.clone(),
            BotUpdateHandler::new(self.api, input_sender, self.context),
        )
        .run()
        .await;
    }
}

pub type CommandHandler<T> =
    Box<dyn Fn(BotUpdateHandler, Command) -> BoxFuture<'static, T> + Send + Sync>;

pub struct Context {
    pub commands: Arc<RwLock<Option<Arc<HashMap<String, GameCommand>>>>>,
    pub output_chat: Arc<Mutex<HashSet<i64>>>,
}

impl Context {
    fn init() -> Self {
        Self {
            commands: Arc::new(RwLock::new(None)),
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

pub struct BotUpdateHandler {
    pub api: Api,
    pub input_sender: mpsc::Sender<String>,
    pub context: Context,
}

impl BotUpdateHandler {
    fn new(api: Api, input_sender: mpsc::Sender<String>, context: Context) -> Self {
        Self {
            api,
            input_sender,
            context,
        }
    }
}

impl Clone for BotUpdateHandler {
    fn clone(&self) -> Self {
        Self {
            api: self.api.clone(),
            input_sender: self.input_sender.clone(),
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
                    let commands = Arc::clone(
                        Arc::clone(&handler.context.commands)
                            .read()
                            .await
                            .as_ref()
                            .unwrap(),
                    );

                    if let Some(game_command) = commands.get(command.get_name()) {
                        (game_command.handler)(handler, command).await;
                    }
                }
            }
        })
    }
}
