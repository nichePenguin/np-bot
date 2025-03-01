use futures::prelude::*;
use irc::client::prelude::{
    Config as IrcConfig,
    Capability, Command, Message, Client
};

use std::{
    error::Error,
    sync::{Arc, Mutex},
    path::PathBuf
};

use super::config::{self, Config, FeatureKey};
use super::message_queue;
use np_utils::{file_watch, log_line};

const SEPARATOR: &str = ",";

pub async fn connect(
    token: &str,
    config_path: PathBuf,
    tarot_history: PathBuf,
    tarot: np_tarot::Tarot) -> Result<tokio::task::JoinHandle<()>, Box<dyn Error>> 
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
            client.send_join(&channel.name);
        }
    }

    let main_config = Arc::new(Mutex::new(main_config));
    let config_ref= Arc::clone(&main_config);
    let client = Arc::new(Mutex::new(client));
    let client_ref= Arc::clone(&client);

    let queue = Arc::new(message_queue::start(client, 850));

    log::debug!("Starting config watcher...");
    np_utils::file_watch(config_path, 1000*3, Box::new(move |data| {
        log::info!("Config updated");
        if let Err(e) = update_config(Arc::clone(&client_ref), Arc::clone(&main_config), data) {
            log::error!("Error parsing updated config: {}", e);
        }
    }));

    log::info!("Starting IRC loop...");
    Ok(tokio::task::spawn( async move {
        log::info!("IRC loop started");
        while let Some(message) = stream.next().await {
            if let Err(e) = message {
                log::error!("Error receiving message: {}", e);
                log::error!("Exiting...");
                break;
            }
            let message = message.unwrap();

            if message.source_nickname().unwrap_or("unknown") == "nichepenguin" {
                queue.reset_delay();
            }

            if let Command::PRIVMSG(channel, text) = message.clone().command {
                if text.starts_with("hmmm") {
                    if is_enabled(FeatureKey::Hmm, &channel, &config_ref) {
                        reply_or_send("[💚] limesHmm", &channel, &queue, message).await;
                    }
                    continue;
                }
                if text.starts_with("!rice") {
                    if is_enabled(FeatureKey::Rice, &channel, &config_ref) {
                        reply_or_send("[💚] RICE BURNED TO CHARCOAL!!!", &channel, &queue, message).await;
                    }
                    continue;
                }
                if text.starts_with("!draw") {
                    if is_enabled(FeatureKey::Tarot, &channel, &config_ref) {
                        let card = tarot.draw();
                        if let Err(e) = card {
                            log::error!("Error drawing a card for {}: {}", message.source_nickname().unwrap_or("unknown"), e);
                            continue;
                        }
                        let (card, affinity) = card.unwrap();
                        let username = get_message_tag(&message, "display-name").unwrap_or("unknown".to_owned());
                        let color = get_message_tag(&message, "color").unwrap_or("#FFFFFF".to_owned());
                        let user_id = get_message_tag(&message, "user-id").unwrap_or("unknown".to_owned());
                        if let Err(e) = log_card(
                            &tarot_history,
                            &card, affinity, &channel, &username, &color, &user_id) {
                            log::error!("Error logging card draw by {} : {}", username, e);
                        }
                        log::info!("{}: {} drew {}", channel, username, card);
                        let sigil = if card.contains("Reversed") {"[💜]"} else {"[💚]"};
                        reply_or_send(&format!("{} {}", sigil, card), &channel, &queue, message).await;
                    }
                    continue;
                }
            }
        }
        log::info!("IRC loop exiting...");
    }))
}

fn log_card(
    history_file: &PathBuf,
    card: &str,
    affinity: i32,
    channel: &str,
    user: &str,
    color: &str,
    user_id: &str) -> Result<(), Box<dyn Error>> 
{
    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time traveled too much");
    let row = [
        time.as_secs().to_string(),
        channel.to_string(),
        user.to_string(),
        color.to_string(),
        card.to_string(),
        affinity.to_string(),
        user_id.to_string()
    ].join(SEPARATOR);
    np_utils::log_line(history_file, row, 10)
}

fn get_message_tag(message: &Message, tag: &str) -> Option<String> {
    if let Some(tags) = &message.tags {
        tags.iter().find(|t| t.0 == tag).map(|t| t.1.clone()).flatten()
    } else {
        None
    }
}

fn is_enabled(feature_key: FeatureKey, channel: &str, config: &Arc<Mutex<Config>>) -> bool {
    let config = config.lock().expect("Error obtaining lock for config");
    if let Some(channel_config) = config.channels.iter().find(|c| c.name == channel) {
        channel_config.features.contains(&FeatureKey::Full) 
        || channel_config.features.contains(&feature_key)
    } else {
        false
    }
}

async fn reply_or_send(
    reply: &str,
    channel: &str,
    queue: &message_queue::MessageQueue,
    reply_to: Message) 
{
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
            vec![channel, reply]).unwrap();
        queue.send(reply).await; 
    } else {
        queue.send(Command::PRIVMSG(channel.to_string(), reply.to_string()).into()).await;
    }
}

fn update_config(
    client: Arc<Mutex<Client>>,
    config: Arc<Mutex<Config>>,
    data: String) -> Result<(), Box<dyn Error>>
{
    let new_config = config::from_json_string(data.as_str())?;
    let mut config = config.lock().expect("Failed to obtain lock for config update");
    let (to_join, to_part) = config::channels_diff(&config, &new_config);
    if to_join.len() != 0 || to_part.len() != 0 {
        let client = client.lock().expect("Failed to obtain lock for client in config update");
        for part in to_part {
            log::debug!("PART {}", part);
            client.send_part(part.to_string());
        }
        for join in to_join {
            log::debug!("JOIN {}", join);
            client.send_join(join.to_string());
        }
    }
    *config = new_config;
    Ok(())
}
