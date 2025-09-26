use std::path::PathBuf;
use std::error::Error;

use irc::client::prelude::{Message, Command};
use crate::config::FeatureKey;
use crate::irc::Context;
use rand::prelude::*;

const HISTORY_SEPARATOR: &str = ",";

enum ParsedMessage {
    Rice,
    Tarot,
    Armory,
    Hmmm,
    VoidStranger,
    Needle,
    Ping(String),
    Np(Vec<String>),
    Ignore,
    Exit
}

fn parse(input: &Message, ctx: &Context) -> (ParsedMessage, Option<String>, Option<FeatureKey>) {
    if let Command::PRIVMSG(channel, text) = &input.command {
        let (parsed, key) = if text.starts_with("!rice") {
            (ParsedMessage::Rice, Some(FeatureKey::Rice))
        } else if text == "!needle" || text == "!haystack"{
            (ParsedMessage::Needle, Some(FeatureKey::Needle))
        } else if text.starts_with("!ping") {
            (ParsedMessage::Ping(text.clone()), Some(FeatureKey::Ping))
        } else if text.starts_with("!armory") {
            (ParsedMessage::Armory, Some(FeatureKey::Tarot))
        } else if text.starts_with("!draw") {
            (ParsedMessage::Tarot, Some(FeatureKey::Tarot))
        } else if text.starts_with("!voidstranger") {
            (ParsedMessage::VoidStranger, Some(FeatureKey::VoidStranger))
        } else if text.starts_with("hmmm") {
            (ParsedMessage::Hmmm, Some(FeatureKey::Hmmm))
        } else if text.starts_with("!np") {
            (ParsedMessage::Np(text
                .split_whitespace()
                .map(|s| s.to_owned())
                .collect()), Some(FeatureKey::Np))
        } else if text.starts_with(ctx.safe_word.as_str()) {
            log::info!("Secret word red");
            (ParsedMessage::Exit, Some(FeatureKey::Any))
        } else {
            (ParsedMessage::Ignore, None)
        };
        (parsed, Some(channel.clone()), key)
    } else {
        (ParsedMessage::Ignore, None, None)
    }
}

fn get_message_tag(message: &Message, tag: &str) -> Option<String> {
    if let Some(tags) = &message.tags {
        tags.iter().find(|t| t.0 == tag).map(|t| t.1.clone()).flatten()
    } else {
        None
    }
}

pub async fn handle(input: Message, ctx: &Context) -> Result<bool, Box<dyn std::error::Error>> {
    let (parsed, channel, key) = parse(&input, ctx);
    if let ParsedMessage::Ignore = parsed  {
        return Ok(false);
    }
    if channel.is_none() || key.is_none() || !ctx.is_enabled(key.unwrap(), &channel.as_ref().unwrap()[..])
    {
        return Ok(false);
    }
    let channel = channel.unwrap();
    match parsed {
        ParsedMessage::Exit => {
            let username = get_message_tag(&input, "display-name").unwrap_or("unknown".to_owned());
            if username != "nichePenguin" {
                log::info!("Good one, {}!", username);
                ctx.reply_or_send(input, "Good one!").await?
            } else {
                ctx.reply_or_send(input, "Bye, boss!").await?
            }
            return Ok(true);
        },
        ParsedMessage::Ignore => {},
        ParsedMessage::Needle => {
            if rand::rng().random::<u8>() > 250 {
                let username = get_message_tag(&input, "display-name").unwrap_or("unknown".to_owned());
                let needle = ctx.swords.draw(&username, true).await.map_err(|e| e.to_string())?;
                ctx.reply_or_send(input, format!("[💚] You rummage around in a haystack... finding {}!", needle).as_str()).await?;
                log::info!("{}: {} found {}", channel, username, &needle);
                ctx.swords.log(needle).await.map_err(|e| e.to_string())?;
            } else {
                ctx.reply_or_send(input, "[💚] You rummage around in a haystack... not finding any needles...").await?
            }
        },
        ParsedMessage::Ping(text) => {
            let reply = format!("[💚] pong{}", &text[5..]);
            ctx.reply_or_send(input, reply.as_str()).await?
        },
        ParsedMessage::VoidStranger => ctx.reply_or_send(input, "[💚] store.steampowered.com/app/2121980").await?,
        ParsedMessage::Rice => ctx.reply_or_send(input, "[💚] RICE BURNED TO CHARCOAL!!!").await?,
        ParsedMessage::Hmmm => ctx.reply_or_send(input, "[💚] lcolonThinking").await?,
        ParsedMessage::Armory => {
            let username = get_message_tag(&input, "display-name").unwrap_or("unknown".to_owned());
            let (count, example) = ctx.swords.check(&username).await.map_err(|e| e.to_string())?;
            let message = if example.is_some() {
                if count == 1 {
                    format!("[💚] A single blade is kept safe in your armory, thus you gaze upon {}.", example.unwrap())
                } else if count < 100 {
                    format!("[💚] Your armory boasts {} swords, you set your eyes upon {}.", count, example.unwrap())
                } else {
                    format!("[💚] Your armory groans beneath the weight of {} blades, yet you regard just one this time: {}.", count, example.unwrap())
                }
            } else {
                format!("[💚] Your hand has not yet taken to your sword...")
            };
            log::info!("{}: {}", channel, message);
            ctx.reply_or_send(input, message.as_str()).await?;
            return Ok(false);
        },
        ParsedMessage::Tarot => {
            let username = get_message_tag(&input, "display-name").unwrap_or("unknown".to_owned());
            if rand::rng().random::<u8>() >= (255 - 32) {
                let sword = ctx.swords.draw(&username, false).await.map_err(|e| e.to_string())?;
                let message = format!("[💚] {} drew a sword, en garde! It's {}.", username, sword);
                log::info!("{}: {}", channel, message);
                ctx.reply_or_send(input, message.as_str()).await?;
                ctx.swords.log(sword).await.map_err(|e| e.to_string())?;
                return Ok(false);
            }
            let card = ctx.tarot.draw();
            if let Err(e) = card {
                log::error!("Error drawing a card for {}: {}", input.source_nickname().unwrap_or("unknown"), e);
                return Err(e);
            }
            let (card, affinity) = card.map_err(|e| format!("Error drawing card: {}", e))?;
            let color = get_message_tag(&input, "color").unwrap_or("#FFFFFF".to_owned());
            let user_id = get_message_tag(&input, "user-id").unwrap_or("unknown".to_owned());
            if let Err(e) = log_card(
                &ctx.tarot_history,
                &card, affinity, &channel, &username, &color, &user_id) {
                log::error!("Error logging card draw by {} : {}", username, e);
            }
            log::info!("{}: {} drew {}", channel, username, card);
            let sigil = if card.contains("Reversed") {"[💜]"} else {"[💚]"};
            let reply = format!("{} {}", sigil, card);
            ctx.reply_or_send(input, reply.as_str()).await?
        },
        ParsedMessage::Np(tokens) => {
            let username = get_message_tag(&input, "display-name").unwrap_or("unknown".to_owned());
            log::info!("Noted user: {}", username);
            log::debug!("User sent: {:?}", tokens);
            if !std::fs::read_to_string(&ctx.noted_users)?.split_whitespace().any(|s| s == username){
                np_utils::log_line(&ctx.noted_users, username, 1000)?;
            }
            ctx.reply_or_send(input, "Your curiosity will be rewarded").await?
        }
    }
    return Ok(false);
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
    ].join(HISTORY_SEPARATOR);
    np_utils::log_line(history_file, row, 10)
}

