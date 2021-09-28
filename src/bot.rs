use crate::{
    command::{GameCommand, GameCommandMap},
    config::Config,
};
use futures_util::future::BoxFuture;
use itertools::Itertools;
use std::{
    collections::{HashMap, HashSet},
    convert::TryFrom,
    process,
    sync::Arc,
};
use tgbot::{
    longpoll::LongPoll,
    methods::{GetMe, SendMessage, SetMyCommands},
    types::{BotCommand, Command, MessageKind, Update, UpdateKind},
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
            context: Context::init(config.user.clone()),
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
                println!("Running in longpoll mode\n");
                LongPoll::new(
                    self.api.clone(),
                    BotUpdateHandler::new(self.api, input_sender, self.context),
                )
                .run()
                .await;
            } else {
                println!("Running at port {} in webhook mode\n", self.webhook);
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
    pub user: String,
    pub bot_username: Arc<RwLock<Option<String>>>,
    pub commands: Arc<RwLock<Option<Arc<HashMap<String, GameCommand>>>>>,
    pub output_chat: Arc<Mutex<HashSet<i64>>>,
    pub bot_commands_sent: Arc<RwLock<bool>>,
}

impl Context {
    fn init(user_id: String) -> Self {
        Self {
            user: user_id,
            bot_username: Arc::new(RwLock::new(None)),
            commands: Arc::new(RwLock::new(None)),
            output_chat: Arc::new(Mutex::new(HashSet::new())),
            bot_commands_sent: Arc::new(RwLock::new(false)),
        }
    }
}

impl Clone for Context {
    fn clone(&self) -> Self {
        Self {
            user: self.user.clone(),
            bot_username: Arc::clone(&self.bot_username),
            commands: Arc::clone(&self.commands),
            output_chat: Arc::clone(&self.output_chat),
            bot_commands_sent: Arc::clone(&self.bot_commands_sent),
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
                    let bot_commands_sent = Arc::clone(&handler.context.bot_commands_sent);

                    let is_bot_commands_set = bot_commands_sent.read().await.clone();
                    if !is_bot_commands_set {
                        let commands = Arc::clone(
                            Arc::clone(&handler.context.commands)
                                .read()
                                .await
                                .as_ref()
                                .unwrap(),
                        );
                        let command_list = commands
                            .iter()
                            .sorted_unstable_by(|a, b| Ord::cmp(&a.0, &b.0))
                            .map(|(name, command)| {
                                BotCommand::new(name, command.description.clone())
                            })
                            .flat_map(|command| command);

                        let set_my_commands = SetMyCommands::new(command_list);
                        match handler.api.execute(set_my_commands).await {
                            Ok(_) => (),
                            Err(e) => {
                                eprintln!(
                                    "Failed sending the command list to Telegram: {}",
                                    e.to_string()
                                );
                                process::exit(1);
                            }
                        }

                        let mut is_bot_commands_set = bot_commands_sent.write().await;
                        *is_bot_commands_set = true;
                    }

                    let bot_username = Arc::clone(&handler.context.bot_username);
                    let is_bot_username_known = bot_username.read().await.is_some();

                    if !is_bot_username_known {
                        let bot = match handler.api.execute(GetMe).await {
                            Ok(b) => b,
                            Err(e) => {
                                eprintln!(
                                    "Failed to get bot info from Telegram: {}",
                                    e.to_string()
                                );
                                process::exit(1);
                            }
                        };

                        let mut bot_username = bot_username.write().await;
                        *bot_username = Some(bot.username);
                    }

                    let mut ignore_message = true;

                    if let Some(user) = command.get_message().get_user() {
                        if user.username.as_ref() == Some(&handler.context.user) {
                            if matches!(command.get_message().kind, MessageKind::Group { .. })
                                || matches!(
                                    command.get_message().kind,
                                    MessageKind::Supergroup { .. }
                                )
                            {
                                let bot_username = bot_username.read().await;

                                if let Some(text) = command.get_message().get_text() {
                                    if text
                                        .data
                                        .contains(&format!("@{}", bot_username.as_ref().unwrap()))
                                    {
                                        ignore_message = false;
                                    }
                                }
                            } else {
                                ignore_message = false;
                            }
                        }
                    }

                    if !ignore_message {
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
            }
        })
    }
}
