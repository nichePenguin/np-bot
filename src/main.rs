mod irc;
mod config;
mod message_queue;

use std::{
    error::Error,
    path::PathBuf,
    env::var,
};

use np_utils::get_env_var;

const HISTORY_FILE: &str = "history.csv";
const AFFINITY_FILE: &str = "affinity.csv";
const CONFIG_FILE: &str = "ircconfig.json";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>>{
    simple_logger::init_with_env().expect("failed to setup logging");
    log::debug!("Reading token");
    let token = var("NPBOT_TOKEN")?;

    let affinity_file = get_env_var("NPBOT_AFFINITY", AFFINITY_FILE);
    let tarot_provider = np_tarot::Tarot::new(PathBuf::from(affinity_file))?;

    let history_file = get_env_var("NPBOT_HISTORY", HISTORY_FILE);
    let config_file = get_env_var("NPBOT_CONFIG", CONFIG_FILE);

    let handle = irc::connect(
        &token,
        PathBuf::from(config_file),
        PathBuf::from(history_file),
        tarot_provider).await?;

    handle.await?;
    Ok(())
}
