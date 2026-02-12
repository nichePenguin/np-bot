mod irc;
mod config;
mod message_handler;
mod message_queue;
mod clonk_stat;
mod armory;
mod sexpr;
mod gateway;

use std::{
    error::Error,
    path::PathBuf,
    env::var,
    sync::Arc,
    time::SystemTime
};

use np_utils::get_env_var;
use log::LevelFilter;

const RETRY_DELAY_MS: u64 = 1000;
const HISTORY_FILE: &str = "history.csv";
const USERS_FILE: &str = "noted_users.txt";
const ELVEN_FILE: &str = "language_elven.txt";
const AFFINITY_FILE: &str = "affinity.csv";
const CONFIG_FILE: &str = "ircconfig.json";

fn setup_logger() -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {} {}] {}",
                humantime::format_rfc3339_seconds(SystemTime::now()),
                record.level(),
                record.target(),
                message
            ))
        })
        .level(log::LevelFilter::Debug)
        .level_for("reqwest", LevelFilter::Off)
        .level_for("hyper", LevelFilter::Off)
        .chain(std::io::stdout())
        .chain(fern::log_file("log.txt")?)
        .apply()?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>>{
    setup_logger()?;
    let mut handle = connect().await;
    let mut attempts = 0;
    let max_attempts = 5;
    loop {
        let restart = match handle {
            Ok(result) => {
                log::info!("Connected");
                attempts = 0;
                let result = result.await;
                match result {
                    Ok(Ok(_)) => {
                        log::info!("Exiting main loop, bye!");
                        false
                    },
                    Ok(Err(e)) => {
                        log::error!("Unrecoverable error in IRC loop: {}", e);
                        true
                    },
                    Err(join_error) => {
                        log::error!("IRC loop panicked: {}", join_error);
                        true
                    }
                }
            },
            Err(e) => {
                log::error!("Error connecting: {}", e);
                true
            }
        };

        if restart {
            if attempts >= max_attempts {
                log::error!("Failed after {} attempts, exiting...", max_attempts);
                return Err(format!("Failed to connect after {} attempts", max_attempts).into());
            } else {
                attempts += 1;
                log::info!("Retrying in {}ms ... [{}/{}]", RETRY_DELAY_MS, attempts, max_attempts);
            }
        } else {
            break;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(RETRY_DELAY_MS)).await;
        handle = connect().await;
    }
    Ok(())
}

async fn connect() -> Result<tokio::task::JoinHandle<Result<(), Box<dyn Error + Send + Sync>>>, Box<dyn Error>> {
    log::debug!("Reading token");
    let token = var("NPBOT_TOKEN")?;
    log::debug!("Reading safeword");
    let safe_word = var("NPBOT_SAFEWORD")?;
    log::debug!("Reading gateway url");
    let gateway = var("NPBOT_GATEWAY")?;
    log::debug!("Reading gateway secret");
    let gateway_secret = var("NPBOT_GATEWAY_KEY")?;
    log::debug!("All secrets are red and kept safe");

    let affinity_file = get_env_var("NPBOT_AFFINITY", AFFINITY_FILE);
    let tarot_provider = np_tarot::Tarot::new(PathBuf::from(affinity_file))?;

    let gateway = Arc::new(gateway::Gateway::init(gateway, gateway_secret)?);

    let elven = get_env_var("NPBOT_ELVEN", ELVEN_FILE);
    let sword_provider = armory::Swords::new(
        PathBuf::from(elven),
        Arc::clone(&gateway),
    ).await.map_err(|e| e.to_string())?;

    let history_file = get_env_var("NPBOT_HISTORY", HISTORY_FILE);
    let noted_users = get_env_var("NPBOT_USERS", USERS_FILE);
    let config_file = get_env_var("NPBOT_CONFIG", CONFIG_FILE);

    irc::connect(
        &token,
        safe_word,
        PathBuf::from(config_file),
        PathBuf::from(history_file),
        PathBuf::from(noted_users),
        sword_provider,
        tarot_provider,
        gateway,
    ).await
}
