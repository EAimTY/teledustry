use crate::bot::BotUpdateHandler;
use futures_util::future::BoxFuture;
use itertools::Itertools;
use std::{collections::HashMap, process, sync::Arc};
use tgbot::{methods::SendMessage, types::Command, ExecuteError};

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
