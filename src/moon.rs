use std::error::Error;
use chrono::{Local, Datelike};
use reqwest::{Url, Client};
use tokio::sync::RwLock;
use scraper::{
    Html, Selector,
    html::Select
};

pub struct Moon {
    client: Client,
    url: Url,
    last_day: RwLock<u32>,
    last_moon: RwLock<Option<MoonInfo>>,
    last_sun: RwLock<Option<SunInfo>>
}

#[derive(Clone)]
pub struct MoonInfo {
    pub day: String,
    pub month: String,
    pub phase: String,
    pub emoji: char,
    pub illumination: String,
    pub age: String,
    pub angle: String,
    pub distance:String 
}

#[derive(Clone)]
pub struct SunInfo {
    angle: f64,
    distance: f64
}

pub fn init(url: String) -> Result<Moon, Box<dyn Error>>{
    let url = Url::parse(url.as_str())?;
    let client = Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .connect_timeout(std::time::Duration::from_secs(10))
        .build()?;
    Ok(Moon {
        client,
        url,
        last_day: RwLock::new(1000),
        last_moon: RwLock::new(None),
        last_sun: RwLock::new(None),
    })
}

fn read_next(name: &str, select: &mut Select) -> Result<String, Box<dyn Error>> {
    select
        .next()
        .map(|e| e.inner_html())
        .ok_or(format!("Failed to read field {}", name).into())
}
impl Moon {
    pub async fn info(&self) -> Result<MoonInfo, Box<dyn Error>> {
        let now = Local::now();
        let day = now.day();
        if *self.last_day.read().await != day || self.last_moon.read().await.is_none() {
            let result = self.parse(self.fetch().await?)?;
            *self.last_moon.write().await = Some(result);
            *self.last_day.write().await = day;
        }
        Ok(self.last_moon.read().await.clone().unwrap())
    }

    async fn fetch(&self) -> Result<String, Box<dyn Error>> {
        Ok(self.client.get(self.url.clone()).send().await?.text().await?)
    }

    fn parse(&self, data: String) -> Result<MoonInfo, Box<dyn Error>> {
        let document = Html::parse_document(data.as_str());
        let selector = Selector::parse(r#"div[id="moonDetails"]"#)?;
        if let Some(info) = document.select(&selector).next() {
            let data = Html::parse_fragment(info.inner_html().as_str());
            let selector = Selector::parse("span")?;
            self.parse_entries(&mut data.select(&selector))

        } else {
            Err("Error parsing - no moonDetails div!".into())
        }
    }

    fn parse_entries(&self, select: &mut Select) -> Result<MoonInfo, Box<dyn Error>> {
        let now = Local::now();
        let day = now.day();
        let month = self.to_month(now.month());
        let phase = read_next("phase", select)?;
        Ok(MoonInfo {
            day: day.to_string(),
            month: month.to_owned(),
            phase: phase.clone(),
            emoji: self.to_emoji(phase),
            illumination: read_next("illumination", select)?,
            age: read_next("age", select)?,
            angle: read_next("angle", select)?,
            distance: read_next("distance", select)?,
        })
    }


    fn to_month(&self, month: u32) -> &'static str {
        match month {
            1 => "Jan",
            2 => "Feb",
            3 => "Mar",
            4 => "Apr",
            5 => "May",
            6 => "Jun",
            7 => "Jul",
            8 => "Aug",
            9 => "Sep",
            10 => "Oct",
            11 => "Nov",
            12 => "Dec",
            _ => "???"
        }
    }

    fn to_emoji(&self, phase: String) -> char {
        match phase.as_str() {
            "New" => '🌑',
            "Waxing Crescent" => '🌒',
            "First Quarter" => '🌓',
            "Waxing Gibbous" => '🌔',
            "Full" => '🌕',
            "Waning Gibbous" => '🌖',
            "Last Quarter" => '🌗',
            "Waning Crescent" => '🌘',
            _ => '?'
        }
    }
}
