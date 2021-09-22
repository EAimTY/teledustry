use std::{process::Stdio, sync::Arc};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, Command},
    sync::{
        mpsc::{Receiver, Sender},
        RwLock,
    },
};

pub struct Game {
    game: Child,
}

impl Game {
    pub fn new() -> Result<Self, String> {
        let game = Command::new("java")
            .arg("-jar")
            .arg("server.jar")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .or_else(|e| Err(e.to_string()))?;

        Ok(Self { game })
    }

    pub async fn listen(
        mut self,
        sender: Sender<String>,
        mut receiver: Receiver<String>,
    ) -> Result<(), String> {
        let output = Arc::new(RwLock::new(String::new()));

        let output_clone = Arc::clone(&output);

        let mut stdin = self.game.stdin.take().unwrap();

        tokio::spawn(async move {
            while let Some(cmd) = receiver.recv().await {
                stdin
                    .write(format!("{}\n", cmd.trim()).as_bytes())
                    .await
                    .unwrap();

                let mut output = output_clone.write().await;
                print!("{}", *output);
                (*output).clear();
            }
        });

        let mut stdout = BufReader::new(self.game.stdout.take().unwrap());

        let mut line = String::new();

        while let Ok(_) = stdout.read_line(&mut line).await {
            let mut output = output.write().await;
            (*output).push_str(&line);
            line.clear();
        }

        Ok(())
    }
}
