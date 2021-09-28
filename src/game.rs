use crate::config::Config;
use std::{
    process::{self, Stdio},
    str,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Command,
    sync::mpsc::{Receiver, Sender},
};

pub struct Game;

impl Game {
    pub async fn spawn(
        config: &Config,
        output_sender: Sender<String>,
        mut input_receiver: Receiver<String>,
    ) -> Result<(), String> {
        let mut game = Command::new("java")
            .arg("-jar")
            .arg(config.file.as_str())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .or_else(|e| Err(e.to_string()))?;

        let mut game_stdin = game
            .stdin
            .take()
            .ok_or_else(|| String::from("Failed to create STDIN pipe"))?;

        let input_handler = tokio::spawn(async move {
            match game_stdin
                .write("help\nEND_CMD\nEND_CMD\n".as_bytes())
                .await
            {
                Ok(_) => (),
                Err(e) => {
                    eprintln!(
                        "Failed to communicate with the game process: {}",
                        e.to_string()
                    );
                    process::exit(1);
                }
            }

            while let Some(cmd) = input_receiver.recv().await {
                match game_stdin
                    .write(format!("{}\nEND_CMD\nEND_CMD\n", cmd).as_bytes())
                    .await
                {
                    Ok(_) => (),
                    Err(e) => {
                        eprintln!(
                            "Failed to communicate with the game process: {}",
                            e.to_string()
                        );
                        process::exit(1);
                    }
                }
            }
        });

        let mut game_stdout = BufReader::new(
            game.stdout
                .take()
                .ok_or_else(|| String::from("Failed to create STDOUT pipe"))?,
        );

        let output_handler = tokio::spawn(async move {
            let mut output = String::new();

            let ignore = b"[00-00-0000 00:00:00] [0] "
                .iter()
                .cloned()
                .collect::<Vec<u8>>();

            let mut last_line = Vec::new();
            last_line.clone_from(&ignore);

            let mut buf = Vec::new();

            while let Ok(_) = game_stdout.read_until(10, &mut buf).await {
                buf = strip_ansi_escapes::strip(&buf).unwrap_or(ignore.clone());

                if buf.ends_with(b"[I] Server loaded. Type 'help' for help.\n") {
                    buf.clear();
                    continue;
                }

                let end_indicator = b"[E] Invalid command. Type 'help' for help.\n";

                if buf.ends_with(end_indicator) && last_line.ends_with(end_indicator) {
                    print!("{}", output);

                    match output_sender.send(output.clone()).await {
                        Ok(_) => (),
                        Err(e) => {
                            eprintln!(
                                "Failed to communicate with the bot instance: {}",
                                e.to_string()
                            );
                            process::exit(1);
                        }
                    }

                    output.clear();
                    buf.clear();
                    last_line.clone_from(&ignore);
                } else {
                    if output.len() + last_line.len() > 4096 + 26 {
                        print!("{}", output);

                        match output_sender.send(output.clone()).await {
                            Ok(_) => (),
                            Err(e) => {
                                eprintln!(
                                    "Failed to communicate with the bot instance: {}",
                                    e.to_string()
                                );
                                process::exit(1);
                            }
                        }
                        output.clear();
                    }

                    output.push_str(str::from_utf8(&last_line[26..]).unwrap_or(""));

                    last_line.clone_from(&buf);
                    buf.clear();
                }
            }
        });

        tokio::try_join!(input_handler, output_handler).or_else(|e| Err(e.to_string()))?;

        Ok(())
    }
}
