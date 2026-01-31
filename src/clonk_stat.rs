use crate::sexpr::{Value, parse};
use std::collections::HashMap;

pub fn parse_stats(data: &str) -> Result<HashMap<String, Vec<Value>>, Box<dyn std::error::Error + Send + Sync>> {
    let parsed = parse(data, true).map_err(|e| format!("Error while parsing: {}", e.to_string()))?;
    let mut result = HashMap::new();
    if let Value::List(vec) = parsed {
        for line in vec {
            if let Value::List(entry) = line {
                if let Some(Value::Key(key)) = entry.get(0) {
                    result.insert(key.to_string(), entry);
                }
            }
        }
        Ok(result)
    } else {
        Err("Data is not a list of stats".into())
    }
}
/*
pub fn format_stat(key: &str, data: Vec<Value>) {
    match key {
        "color" => one(data),
        "name" => one(data),
        "resolution2025" => one(data),
        "copfish-ratio" => format!("Caught {} out of {} fish at twitch.tv/badcop_"),
        "news-edition" => one(data),
        "shindaggers-knives" => 
    }
}
*/
