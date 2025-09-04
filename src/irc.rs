use futures::prelude::*;
use futures::stream::FusedStream;
use irc::client::prelude::{
    Config as IrcConfig,
    Capability, Command, Message, Client
};

use std::{
    error::Error,
    sync::{Arc, Mutex},
    path::PathBuf
};


use crate::config::{self, Config, FeatureKey};
use crate::message_handler::handle;
use crate::message_queue;

pub struct Context {
    queue: Arc<message_queue::MessageQueue>,
    pub tarot: np_tarot::Tarot,
    pub tarot_history: PathBuf,
    pub noted_users: PathBuf,
    pub safe_word: String,
    config: Arc<Mutex<Config>>
}

impl Context {
    pub async fn reply_or_send(&self, reply_to: Message, text: &str) -> Result<(), Box<dyn Error>> {
        let channel = if let Command::PRIVMSG(channel, _) = reply_to.command {
            channel
        } else {
            return Err("No channel in to_reply message".into());
        };

        if let Some(Some(message_id)) = reply_to.tags
            .into_iter().flatten()
            .find(|t| t.0 == "id").map(|t| t.1)
        {
            let reply = Message::with_tags(
                Some(vec![
                    irc::proto::message::Tag("reply-parent-msg-id".to_owned(), Some(message_id))
                ]),
                None,
                "PRIVMSG",
                vec![&channel[..], text])?;
            self.queue.send(reply).await;
        } else {
            self.queue.send(Command::PRIVMSG(channel.to_string(), text.to_string()).into()).await;
        }
        Ok(())
    }

    pub fn is_enabled(&self, key: FeatureKey, channel: &str) -> bool{
        if let FeatureKey::Any = key {
            return true;
        }

        let config = self.config.lock();
        if let Err(e) = &config {
            log::error!("Failed to get config lock, assuming feature disabled: {}", e);
        }
        let config = config.unwrap();

        fn contains_negative(key: &FeatureKey, channel_config: &config::ChannelConfig) -> bool {
            channel_config.features.iter().any(|config_key| 
                if let FeatureKey::Not(not_key) = config_key {
                    *key == **not_key
                } else {
                    false
                })
        }

        if let Some(channel_config) = config.channels.iter().find(|c| c.name == channel) {
            !contains_negative(&key, channel_config) && (
                channel_config.features.contains(&FeatureKey::Full)
                || channel_config.features.contains(&key))
        } else {
            false
        }
    }
}

pub async fn connect(
    token: &str,
    safe_word: String,
    config_path: PathBuf,
    tarot_history: PathBuf,
    noted_users: PathBuf,
    tarot: np_tarot::Tarot) -> Result<tokio::task::JoinHandle<Result<(), Box<dyn Error + Send + Sync>>>, Box<dyn Error>>
{
    let config = IrcConfig {
        nickname: Some("nichePenguin".to_owned()),
        server: Some("irc.twitch.tv".to_owned()),
        port: Some(6667),
        password: Some(token.to_owned()),
        use_tls: Some(false),
        .. IrcConfig::default()
    };

    let mut client = Client::from_config(config).await?;
    let mut stream = client.stream()?;

    client.identify()?;
    client.send_cap_req(&[
        Capability::Custom("twitch.tv/membership"),
        Capability::Custom("twitch.tv/tags")])?;

    let main_config = config::from_json(&config_path)?;

    for channel in &main_config.channels {
        if channel.active {
            if let Err(e) = client.send_join(&channel.name) {
                log::error!("Error joining channel {}: {}", &channel.name, e)
            }
        }
    }

    let main_config = Arc::new(Mutex::new(main_config));
    let config_ref= Arc::clone(&main_config);
    let client = Arc::new(Mutex::new(client));
    let client_ref= Arc::clone(&client);

    let queue = Arc::new(message_queue::start(client, 850).await);
    let ctx = Context {
        queue,
        tarot,
        tarot_history,
        noted_users,
        safe_word,
        config: config_ref
    };

    log::debug!("Starting config watcher...");
    np_utils::file_watch(config_path, 1000*3, Box::new(move |data| {
        log::info!("Config updated");
        if let Err(e) = update_config(Arc::clone(&client_ref), Arc::clone(&main_config), data) {
            log::error!("Error parsing updated config: {}", e);
        }
    }));

    Ok(tokio::task::spawn( async move {
        log::info!("IRC loop started");
        while let Some(message) = stream.next().await {
            if let Err(e) = message {
                log::error!("Error receiving message: {}", e);
                log::error!("Exiting...");
                break;
            }

            let message = message?;

            if message.source_nickname().unwrap_or("unknown") == "nichepenguin" {
                ctx.queue.reset_delay().await;
            }
            let exit = handle(message, &ctx).await.map_err(|e| format!("Error handling message: {}", e))?;
            if exit {
                log::info!("Received exit request on handle");
                break;
            }
        }
        log::info!("IRC loop exiting...");
        ctx.queue.stop_loop();
        if stream.is_terminated() {
            Err("Client stream terminated without a command, will retry".into())
        } else {
            Ok(())
        }
    }))
}

fn update_config(
    client: Arc<std::sync::Mutex<Client>>,
    config: Arc<std::sync::Mutex<Config>>,
    data: String) -> Result<(), Box<dyn Error>>
{
    let new_config = config::from_json_string(data.as_str())?;
    let mut config = config.lock().map_err(|e| format!("Failed to obtain config lock: {}", e))?;
    let (to_join, to_part) = config::channels_diff(&config, &new_config);
    if to_join.len() != 0 || to_part.len() != 0 {
        let client = client.lock().map_err(|e| format!("Failed to obtain client lock: {}", e))?;
        for part in to_part {
            let part = part.to_string();
            log::debug!("PART {}", part);
            if let Err(e) = client.send_part(&part) {
                log::error!("Error parting from {}: {}", part, e)
            }
        }
        for join in to_join {
            let join = join.to_string();
            log::debug!("JOIN {}", join);
            if let Err(e) = client.send_join(&join) {
                log::error!("Error joining to {}: {}", join, e)
            }
        }
    }
    *config = new_config;
    Ok(())
}
