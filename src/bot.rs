use crate::{
    command::{GameCommand, GameCommandMap},
    config::Config,
};
use futures_util::future::BoxFuture;
use std::{
    collections::{HashMap, HashSet},
    convert::TryFrom,
    process,
    sync::Arc,
};
use tgbot::{
    longpoll::LongPoll,
    methods::SendMessage,
    types::{Command, Update, UpdateKind},
    webhook, Api, Config as ApiConfig, UpdateHandler,
};
use tokio::{
    sync::{mpsc, Mutex, RwLock},
    task::JoinHandle,
};

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
            webhook: config.webhook,
            context: Context::init(),
        })
    }

    pub async fn handle_output(
        self,
        mut output_receiver: mpsc::Receiver<String>,
    ) -> JoinHandle<()> {
        let output_handler = tokio::spawn(async move {
            while let Some(output) = output_receiver.recv().await {
                if output.starts_with("Commands:\n") {
                    *self.context.commands.write().await =
                        Some(Arc::new(GameCommandMap::init(output)));
                } else {
                    let output_chat = (*self.context.output_chat.lock().await).clone();

                    for chat_id in output_chat {
                        match self
                            .api
                            .execute(SendMessage::new(chat_id, output.clone()))
                            .await
                        {
                            Ok(_) => (),
                            Err(e) => eprintln!("{}", e.to_string()),
                        }
                    }
                }
            }
        });

        output_handler
    }

    pub async fn handle_input(self, input_sender: mpsc::Sender<String>) -> JoinHandle<()> {
        let input_handler = tokio::spawn(async move {
            if self.webhook == 0 {
                println!("Running in longpoll mode");
                LongPoll::new(
                    self.api.clone(),
                    BotUpdateHandler::new(self.api, input_sender, self.context),
                )
                .run()
                .await;
            } else {
                println!("Running at port {} in webhook mode", self.webhook);
                match webhook::run_server(
                    ([127, 0, 0, 1], self.webhook),
                    "/",
                    BotUpdateHandler::new(self.api, input_sender, self.context),
                )
                .await
                {
                    Ok(_) => (),
                    Err(e) => {
                        eprintln!("Failed running the webhook server: {}", e.to_string());
                        process::exit(1);
                    }
                }
            }
        });
        input_handler
    }
}

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
                        match (game_command.handler)(handler, command).await {
                            Ok(_) => (),
                            Err(e) => eprintln!("{}", e.to_string()),
                        }
                    }
                }
            }
        })
    }
}
