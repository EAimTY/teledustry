use std::{process::Stdio, str};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Command,
    sync::mpsc::{Receiver, Sender},
};

pub struct Game;

impl Game {
    pub async fn start(
        output_sender: Sender<String>,
        mut input_receiver: Receiver<String>,
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
            game_stdin
                .write("help\nEND_CMD\nEND_CMD\n".as_bytes())
                .await
                .unwrap();

            while let Some(cmd) = input_receiver.recv().await {
                game_stdin
                    .write(format!("{}\nEND_CMD\nEND_CMD\n", cmd).as_bytes())
                    .await
                    .unwrap();
            }
        });

        let mut game_stdout = BufReader::new(game.stdout.take().unwrap());

        let handle_output = tokio::spawn(async move {
            let mut output = String::new();

            let ignore = b"[00-00-0000 00:00:00] [0] "
                .iter()
                .cloned()
                .collect::<Vec<u8>>();
            let mut last_line = Vec::new();

            let mut buf = Vec::new();
            last_line.clone_from(&ignore);

            while let Ok(_) = game_stdout.read_until(10, &mut buf).await {
                buf = strip_ansi_escapes::strip(&buf).unwrap();

                if buf.ends_with(b"[I] Server loaded. Type 'help' for help.\n") {
                    buf.clear();
                    continue;
                }

                if buf.ends_with(b"[E] Invalid command. Type 'help' for help.\n")
                    && last_line.ends_with(b"[E] Invalid command. Type 'help' for help.\n")
                {
                    output_sender.send(output.clone()).await.unwrap();
                    output.clear();
                    buf.clear();
                    last_line.clone_from(&ignore);
                } else {
                    if output.len() + last_line.len() > 4096 + 22 {
                        output_sender.send(output.clone()).await.unwrap();
                        output.clear();
                    }

                    output.push_str(str::from_utf8(&last_line[26..]).unwrap());
                    last_line.clone_from(&buf);
                    buf.clear();
                }
            }
        });

        tokio::try_join!(handle_input, handle_output).unwrap();

        Ok(())
    }
}
