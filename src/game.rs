use std::process::Stdio;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Command,
    sync::mpsc::{Receiver, Sender},
};

pub struct Game;

impl Game {
    pub async fn new(sender: Sender<String>, mut receiver: Receiver<String>) -> Result<(), String> {
        let mut game = Command::new("java")
            .arg("-jar")
            .arg("server.jar")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .or_else(|e| Err(e.to_string()))?;

        let mut stdin = game.stdin.take().unwrap();
        let mut stdout = BufReader::new(game.stdout.take().unwrap());

        tokio::spawn(async move {
            while let Some(cmd) = receiver.recv().await {
                stdin
                    .write(format!("{}\n", cmd.trim()).as_bytes())
                    .await
                    .unwrap();
            }
        });

        let mut s = String::new();

        while let Ok(bytes) = stdout.read_line(&mut s).await {
            if bytes != 0 {
                //print!("{}", s);
                s.clear();
            } else {
                println!("here3");
                break;
            }
        }

        Ok(())
    }
}
