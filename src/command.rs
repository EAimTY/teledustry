use crate::bot::BotUpdateHandler;
use futures_util::{future::BoxFuture, StreamExt};
use itertools::Itertools;
use std::{collections::HashMap, path::PathBuf, process, sync::Arc};
use tgbot::{
    methods::{GetFile, SendMessage},
    types::{Command, MessageData},
    ExecuteError,
};
use tokio::{fs::File, io::AsyncWriteExt};

type GameCommandHandler = Box<
    dyn Fn(BotUpdateHandler, Command) -> BoxFuture<'static, Result<(), ExecuteError>> + Send + Sync,
>;

pub struct GameCommand {
    pub description: String,
    pub handler: GameCommandHandler,
}

pub struct GameCommandMap;

impl GameCommandMap {
    pub fn init(help_output: String) -> HashMap<String, GameCommand> {
        let mut commands = HashMap::new();

        fn about(
            handler: BotUpdateHandler,
            command: Command,
        ) -> BoxFuture<'static, Result<(), ExecuteError>> {
            Box::pin(async move {
                let chat_id = command.get_message().get_chat_id();

                let send_message = SendMessage::new(
                    chat_id,
                    r#"
teledustry

Manage your Mindustry server through a Telegram bot.

https://github.com/EAimTY/teledustry

Useful Commands: 
/output - Send the output to this chat
/stop_output - Stop sending the output to this chat
/help - Print the help menu
"#,
                );

                handler.api.execute(send_message).await?;

                Ok(())
            })
        }
        commands.insert(
            String::from("/start"),
            GameCommand {
                description: String::new(),
                handler: Box::new(about) as GameCommandHandler,
            },
        );
        commands.insert(
            String::from("/about"),
            GameCommand {
                description: String::from("About this bot"),
                handler: Box::new(about) as GameCommandHandler,
            },
        );

        fn help(
            handler: BotUpdateHandler,
            command: Command,
        ) -> BoxFuture<'static, Result<(), ExecuteError>> {
            Box::pin(async move {
                let chat_id = command.get_message().get_chat_id();

                let commands = Arc::clone(
                    Arc::clone(&handler.context.commands)
                        .read()
                        .await
                        .as_ref()
                        .unwrap(),
                );

                let mut help_message = String::from("Commands:");

                for (name, game_command) in commands
                    .iter()
                    .sorted_unstable_by(|a, b| Ord::cmp(&a.0, &b.0))
                    .filter(|(name, _command)| name != &"/start")
                {
                    help_message.push_str(&format!("\n{} {}", name, game_command.description));
                }

                handler
                    .api
                    .execute(SendMessage::new(chat_id, help_message))
                    .await?;

                Ok(())
            })
        }
        commands.insert(
            String::from("/help"),
            GameCommand {
                description: String::from("Print the help menu"),
                handler: Box::new(help) as GameCommandHandler,
            },
        );

        fn output(
            handler: BotUpdateHandler,
            command: Command,
        ) -> BoxFuture<'static, Result<(), ExecuteError>> {
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

                handler.api.execute(send_message).await?;

                Ok(())
            })
        }
        commands.insert(
            String::from("/output"),
            GameCommand {
                description: String::from("Send the output to this chat"),
                handler: Box::new(output) as GameCommandHandler,
            },
        );

        fn stop_output(
            handler: BotUpdateHandler,
            command: Command,
        ) -> BoxFuture<'static, Result<(), ExecuteError>> {
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

                handler.api.execute(send_message).await?;

                Ok(())
            })
        }
        commands.insert(
            String::from("/stop_output"),
            GameCommand {
                description: String::from("Stop sending the output to this chat"),
                handler: Box::new(stop_output) as GameCommandHandler,
            },
        );

        fn uploadmap(
            handler: BotUpdateHandler,
            command: Command,
        ) -> BoxFuture<'static, Result<(), ExecuteError>> {
            Box::pin(async move {
                let chat_id = command.get_message().get_chat_id();

                if let MessageData::Document { data, .. } = &command.get_message().data {
                    let file_id = data.file_id.clone();
                    let file_name = data.file_name.clone().unwrap_or(file_id.clone());

                    let get_file = GetFile::new(file_id);
                    let file = handler.api.execute(get_file).await?;

                    let mut is_map_saved = false;

                    if let Some(file_url) = file.file_path {
                        if let Ok(mut file_stream) = handler.api.download_file(file_url).await {
                            let file_path = {
                                let mut path = PathBuf::from(r"config/maps/");
                                path.push(file_name);
                                path
                            };

                            if let Ok(mut map) = File::create(file_path).await {
                                if let Some(file) = file_stream.next().await {
                                    if let Ok(file) = file {
                                        if let Ok(_) = map.write_all(&file).await {
                                            is_map_saved = true;

                                            let send_message = SendMessage::new(
                                                chat_id,
                                                "Map saved. Use /reloadmaps to reload all maps from disk",
                                            );
                                            handler.api.execute(send_message).await?;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if !is_map_saved {
                        let send_message = SendMessage::new(chat_id, "Failed to save the map");
                        handler.api.execute(send_message).await?;
                    }
                } else {
                    let send_message = SendMessage::new(
                        chat_id,
                        "Please send the map file as an attachment of the command",
                    );
                    handler.api.execute(send_message).await?;
                }

                Ok(())
            })
        }
        commands.insert(
            String::from("/uploadmap"),
            GameCommand {
                description: String::from("Upload a map to config/maps/"),
                handler: Box::new(uploadmap) as GameCommandHandler,
            },
        );

        fn generic_handler(
            handler: BotUpdateHandler,
            command: Command,
        ) -> BoxFuture<'static, Result<(), ExecuteError>> {
            Box::pin(async move {
                let name = &command.get_name()[1..].replace('_', "-");
                let args = command
                    .get_args()
                    .into_iter()
                    .map(|arg| format!(" {}", arg))
                    .collect::<String>();

                match handler.input_sender.send(format!("{}{}", name, args)).await {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        eprintln!(
                            "Failed to communicate with the game proccess: {}",
                            e.to_string()
                        );
                        process::exit(1);
                    }
                }
            })
        }

        for command in help_output.split('\n') {
            if let Some((name, description)) = command.trim_start().split_once(' ') {
                commands
                    .entry(format!("/{}", name.replace('-', "_")))
                    .or_insert(GameCommand {
                        description: description
                            .trim_start_matches("- ")
                            .trim_end_matches('.')
                            .to_string(),
                        handler: Box::new(generic_handler) as GameCommandHandler,
                    });
            }
        }

        commands.remove("/exit");

        commands
    }
}
