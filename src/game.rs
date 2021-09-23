use std::{process::Stdio, sync::Arc};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, Command},
    sync::{
        mpsc::{Receiver, Sender},
        RwLock,
    },
};

pub struct Game;

impl Game {
    pub async fn start(
        game_to_bot_sender: Sender<String>,
        mut bot_to_game_receiver: Receiver<String>,
    ) -> Result<(), String> {
        let mut game = Command::new("java")
            .arg("-jar")
            .arg("server.jar")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .or_else(|e| Err(e.to_string()))?;

        let mut game_stdin = game.stdin.take().unwrap();

        let handle_input = tokio::spawn(async move {
            while let Some(cmd) = bot_to_game_receiver.recv().await {
                game_stdin
                    .write(format!("{}\n", cmd.trim()).as_bytes())
                    .await
                    .unwrap();
            }
        });

        let mut game_stdout = BufReader::new(game.stdout.take().unwrap());

        let handle_output = tokio::spawn(async move {
            let mut line = String::new();

            while let Ok(_) = game_stdout.read_line(&mut line).await {
                game_to_bot_sender.send(line.clone()).await.unwrap();
                line.clear();
            }
        });

        tokio::try_join!(handle_input, handle_output).unwrap();

        Ok(())
    }
}
