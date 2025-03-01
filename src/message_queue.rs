use irc::client::prelude::{Client, Command, Message};
use tokio::{
    sync::mpsc,
    time::{Instant, Duration, sleep},
};
use std::sync::{Arc, Mutex};

pub struct MessageQueue {
    sender: mpsc::Sender<Message>,
    last_message: Arc<Mutex<Instant>>,
}

pub fn start(client: Arc<Mutex<Client>>, delay_ms: u64) -> MessageQueue {
    let (tx, mut rx) = mpsc::channel::<Message>(100);
    let last_message = Arc::new(Mutex::new(Instant::now()));
    let last_message_ref = Arc::clone(&last_message);
    tokio::task::spawn(async move {
        let delay = Duration::from_millis(delay_ms);
        log::info!("Starting message queue");
        loop {
            let message = rx.recv().await.expect("Channel to messageq queue closed");

            let mut passed = Instant::now() - get_last_message(&last_message);
            while passed <= delay {
                sleep(delay).await;
                passed = Instant::now() - get_last_message(&last_message);
            }

            {
                let client = client.lock().expect("Failed to obtain lock for client");
                if let Err(e) = client.send(message.clone()) {
                    match message.command {
                        Command::PRIVMSG(channel, chat_message) => 
                            log::error!("Error sending message \" {}\" to {}: {}",
                                chat_message, channel, e),
                        _ => log::error!("Error sending unknown command")
                    }
                }
            }
            *last_message.lock().expect("Failed to obtain lock for message_queue") = Instant::now();
        }
    });

    MessageQueue {
        sender: tx,
        last_message: last_message_ref
    }
}

fn get_last_message(last_message: &Arc<Mutex<Instant>>) -> Instant {
    *last_message.lock().expect("Failed to obtain lock on last_message")
}

impl MessageQueue {
    pub async fn send(&self, message: Message) {
        self.sender.send(message).await;
    }

    pub fn reset_delay(&self) {
        let mut last_message = self.last_message.lock().expect("Failed to obtain lock on last_message");
        *last_message = Instant::now();
    }
}
