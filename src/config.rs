use std::error::Error;
use std::collections::HashSet;

pub struct Config {
    pub channels: Vec<ChannelConfig>
}

/// Calculate channels to disconnect or connect after a config update
pub fn channels_diff(before: &Config, after: &Config) -> (Vec<String>, Vec<String>) {
    let before_set = before.channels.iter().map(|c| c.name.clone()).collect::<HashSet<_>>();
    let after_set = after.channels.iter().map(|c| c.name.clone()).collect::<HashSet<_>>();

    let mut to_remove = before_set.difference(&after_set)
        .filter(|name| channel(name, before).active) // were active, now removed
        .map(|name| name.clone())
        .collect::<Vec<String>>();
    let mut to_add = after_set.difference(&before_set)
        .filter(|name| channel(name, after).active) // were added, now active
        .map(|name| name.clone())
        .collect::<Vec<String>>();

    before_set.intersection(&after_set).for_each( |name| {
        let before = channel(name, before);
        let after = channel(name, after);
        if before.active && !after.active {
            to_remove.push(name.clone()); // were active before, now not
        } else if !before.active && after.active{
            to_add.push(name.clone()); // were not active, now are
        }
    });
    (to_add, to_remove)
}

fn channel<'a, 'b>(name: &'b String, config: &'a Config) -> &'a ChannelConfig {
    config.channels.iter().find(|c| c.name == *name).expect("Original set source always contains it")
}

#[derive(Debug)]
pub struct ChannelConfig {
    pub active: bool,
    pub name: String,
    pub features: Vec<FeatureKey>
}

#[derive(PartialEq, Eq, Debug)]
pub enum FeatureKey {
    Any,
    Full,
    BugAd,
    Tarot,
    Hmmm,
    Mmmm,
    Rice,
    VoidStranger,
    Ping,
    Needle,
    Np,
    Not(Box<FeatureKey>),
    Unknown(String),
}

fn parse_feature(string: &str) -> FeatureKey {
    if string.starts_with("!") {
        let parsed = parse_feature(&string[1..]);
        // Double negation supported :D
        return if let FeatureKey::Not(parsed) = parsed {
            *parsed
        } else {
            FeatureKey::Not(Box::new(parse_feature(&string[1..])))
        }
    }
    match string {
        "full" => FeatureKey::Full,
        "tarot" => FeatureKey::Tarot,
        "rice" => FeatureKey::Rice,
        "hmm" => FeatureKey::Hmmm,
        "mmm" => FeatureKey::Mmmm,
        "bug_ad" => FeatureKey::BugAd,
        "needle" => FeatureKey::Needle,
        "ping" => FeatureKey::Ping,
        "np" => FeatureKey::Np,
        "voidstranger" => FeatureKey::VoidStranger,
        _ => {
            log::warn!("Parsing unknown feature: {}", string);
            FeatureKey::Unknown(string.to_owned())
        }
    }
}

pub fn from_json_string(data: &str) -> Result<Config, Box<dyn Error>> {
    let raw_json = json::parse(data)?;
    if !raw_json.has_key("channels") || !raw_json["channels"].is_array() {
        return Err("Error parsing config: \"channels\" not found or not an array".into());
    }
    let mut channels = Vec::<ChannelConfig>::new();
    for (index, channel) in raw_json["channels"].members().enumerate() {
        channels
            .push(parse_channel(channel)
                .map_err(|e| format!{"Error parsing channel at {} : {}", index, e})?);
    }
    Ok(Config { channels })
}

pub fn from_json(path: &std::path::PathBuf) -> Result<Config, Box<dyn Error>> {
    from_json_string(std::fs::read_to_string(path)?.as_str())
}

fn parse_channel(json: &json::JsonValue) -> Result<ChannelConfig, Box<dyn Error>> {
    Ok(ChannelConfig {
        active: json["active"].as_bool().ok_or("Failed to parse \"active\"")?,
        name: json["name"].as_str().ok_or("Failed to parse \"name\"")?.to_owned(),
        features: parse_features(&json["features"])?
    })
}

fn parse_features(json: &json::JsonValue) -> Result<Vec<FeatureKey>, Box<dyn Error>> {
    let mut result = Vec::<FeatureKey>::new();
    for entry in json.members() {
        result.push(parse_feature(entry.as_str().ok_or("Failed to parse \"features\"")?));
    }
    Ok(result)
}
