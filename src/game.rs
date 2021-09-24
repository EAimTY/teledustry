use std::process::Stdio;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Command,
    sync::mpsc::{Receiver, Sender},
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
            .kill_on_drop(true)
            .spawn()
            .or_else(|e| Err(e.to_string()))?;

        let mut game_stdin = game.stdin.take().unwrap();

        let handle_input = tokio::spawn(async move {
            while let Some(cmd) = bot_to_game_receiver.recv().await {
                game_stdin
                    .write(format!("{}\nCMD_IND\nCMD_IND\n", cmd.trim()).as_bytes())
                    .await
                    .unwrap();
            }
        });

        let mut game_stdout = BufReader::new(game.stdout.take().unwrap());

        let handle_output = tokio::spawn(async move {
            let mut output = String::new();

            let mut buf = String::new();
            let mut last_line = String::new();

            while let Ok(_) = game_stdout.read_line(&mut buf).await {
                if buf.contains("[I] Server loaded. Type 'help' for help.") {
                    continue;
                }

                if buf.contains("[E] Invalid command. Type 'help' for help.")
                    && last_line.contains("[E] Invalid command. Type 'help' for help.")
                {
                    game_to_bot_sender.send(output.clone()).await.unwrap();

                    output.clear();
                    buf.clear();
                    last_line.clear();
                } else {
                    output.push_str(&last_line);

                    last_line.clone_from(&buf);
                    buf.clear();
                }
            }
        });

        tokio::try_join!(handle_input, handle_output).unwrap();

        Ok(())
    }
}
