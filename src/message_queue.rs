use irc::client::prelude::{Client, Command, Message};
use tokio::{
    sync::{mpsc, Mutex},
    time::{Instant, Duration, sleep},
    task::JoinHandle
};
use std::sync::Arc;

pub struct MessageQueue {
    sender: mpsc::Sender<Message>,
    last_message: Arc<Mutex<Instant>>,
    send_loop: JoinHandle<()>,
}

pub async fn start(client: Arc<std::sync::Mutex<Client>>, delay_ms: u64) -> MessageQueue {
    let (tx, mut rx) = mpsc::channel::<Message>(100);
    let last_message = Arc::new(Mutex::new(Instant::now()));
    let last_message_ref = Arc::clone(&last_message);
    let handle = tokio::task::spawn(async move {
        let delay = Duration::from_millis(delay_ms);
        log::info!("Starting message queue");
        loop {
            let message = rx.recv().await.expect("Channel to message queue closed");

            let mut passed = Instant::now() - get_last_message(&last_message).await;
            while passed <= delay {
                sleep(delay).await;
                passed = Instant::now() - get_last_message(&last_message).await;
            }

            {
                let client = client.lock();
                if let Err(e) = &client {
                    log::error!("Error sending message due to obtaining mutex: {}", e);
                }
                let client = client.unwrap();
                if let Err(e) = client.send(message.clone()) {
                    match message.command {
                        Command::PRIVMSG(channel, chat_message) => 
                            log::error!("Error sending message \" {}\" to {}: {}",
                                chat_message, channel, e),
                        _ => log::error!("Error sending unknown command")
                    }
                }
            }
            *last_message.lock().await = Instant::now();
        }
    });

    MessageQueue {
        sender: tx,
        last_message: last_message_ref,
        send_loop: handle
    }
}

async fn get_last_message(last_message: &Arc<Mutex<Instant>>) -> Instant {
    *last_message.lock().await
}

impl MessageQueue {
    pub async fn send(&self, message: Message) {
        if let Err(e) = self.sender.send(message).await {
            log::error!("Error sending message: {}", e);
        }
    }

    pub async fn reset_delay(&self) {
        let mut last_message = self.last_message.lock().await;
        *last_message = Instant::now();
    }

    pub fn stop_loop(&self) {
        self.send_loop.abort();
    }
}
