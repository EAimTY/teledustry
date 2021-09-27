use crate::bot::{BotUpdateHandler, CommandHandler};
use futures_util::future::BoxFuture;
use std::collections::HashMap;
use tgbot::{methods::SendMessage, types::Command};

pub struct CommandList;

impl CommandList {
    pub fn init() -> HashMap<&'static str, CommandHandler<()>> {
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

        commands
    }
}
