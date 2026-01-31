use std::collections::HashMap;
use std::error::Error;
use std::time::Duration;
use reqwest::{
    Client,
    Url,
    header::{HeaderMap, HeaderValue, AUTHORIZATION}
};

pub struct Gateway {
    client: Client,
    base_url: Url,
    retry_count: u16,
}

impl Gateway {
    pub fn init(url: String, secret: String) -> Result<Self, Box<dyn Error>> {
        let mut headers = HeaderMap::new();
        let bearer = format!("Bearer {}", secret);
        headers.insert(AUTHORIZATION, HeaderValue::from_str(bearer.as_str())?);
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .connect_timeout(Duration::from_secs(10))
            .default_headers(headers)
            .build()?;
        Ok(Self {
            base_url: Url::parse(url.as_str())?,
            retry_count: 10,
            client
        })
    }

    pub async fn get_text(&self, path: &str, query_params: HashMap<&str, String>) -> Result<String, Box<dyn Error>> {
        log::debug!("Get: {} {:?}", path, query_params);
        let mut url = self.base_url.join(path)?;
        {
            let mut query = url.query_pairs_mut();
            for (k, v) in query_params {
                query.append_pair(k, v.as_str());
            }
        }
        let mut attempts = 0;
        let mut err = None;
        while attempts < self.retry_count  {
            attempts += 1;
            let resp = self.client.get(url.clone()).send().await;
            match resp {
                Ok(resp) => {
                    match resp.text().await {
                        Ok(text) => {
                            log::debug!("Got: {}", text);
                            return Ok(text)
                        }
                        Err(e) => {
                            log::error!("Failed to read body: {}", e);
                            err = Some(e)
                        }
                    }
                },
                Err(e) => {
                    log::error!("Get failed, boo womp");
                    err = Some(e)
                }
            }
            log::info!("Retry {} out of {}..", attempts, self.retry_count)
        }
        return Err(Box::new(err.unwrap()))
    }

    pub async fn get(&self, path: &str, query_params: HashMap<&str, String>) -> Result<json::JsonValue, Box<dyn Error>> {
        Ok(json::parse(self.get_text(path, query_params).await?.as_str())?)
    }

    pub async fn post(&self, path: &str, body: json::JsonValue) -> Result<Option<json::JsonValue>, Box<dyn Error>> {
        log::debug!("Post: {} {:?}", path, body);
        let url = self.base_url.join(path)?;
        let mut attempts = 0;
        let mut err = None;
        while attempts < self.retry_count {
            attempts += 1;
            match  self.client
                .post(url.clone())
                .body(json::stringify(body.clone()))
                .send().await
            {
            Ok(resp) => {
                match resp.text().await {
                    Ok(text) => {
                        if text.is_empty() {
                            return Ok(None)
                        } else {
                            return Ok(Some(json::parse(text.as_str())?))
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to read body: {}", e);
                        return Ok(None)
                    }
                }
                },
                Err(e) => {
                    log::error!("Post failed, boo womp");
                    err = Some(e)
                }
            }
            log::info!("Retry {} out of {}..", attempts, self.retry_count)
        }
        return Err(Box::new(err.unwrap()))
    }
}
