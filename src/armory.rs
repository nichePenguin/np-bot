use std::fmt;
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use std::io::{BufReader, BufRead};
use std::fs::File;
use std::collections::HashMap;

use cruet::to_title_case;
use tokio::sync::RwLock;
use json::object;
use rand::{
    distr::{Distribution, StandardUniform},
    seq::IndexedRandom,
    Rng
};
use crate::gateway;

const LANG_SIZE: usize = 2222;

pub struct Swords {
    cache: Arc<RwLock<Vec<Sword>>>,
    cache_synced: bool,
    elven: PathBuf
}

impl Swords {
    pub async fn new(elven: PathBuf, gateway: Arc<gateway::Gateway>) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let mut cache_synced = false;
        let cache = match Self::init_cache(gateway).await {
            Ok(cache) => {
                cache_synced = true;
                cache
            },
            Err(e) => {
                log::error!("Failed to initialize cache: {}", e);
                log::warn!("Continuing without local cache...");
                Vec::new()
            }
        };
        Ok(Self {
            elven,
            cache: Arc::new(RwLock::new(cache)),
            cache_synced,
        })
    }

    async fn init_cache(gateway: Arc<gateway::Gateway>) -> Result<Vec<Sword>, Box<dyn Error + Send + Sync>> {
        let mut total = Vec::new();
        let mut page = 1;
        loop {
            page += 1;
            let (mut swords, has_next) = Self::get_swords(page, Arc::clone(&gateway)).await?;
            total.append(&mut swords);
            if !has_next {
                break
            }
        }
        Ok(total)
    }

    fn roll_sword(&self, owner: &String, guarantee_artifact: bool, needle: bool) -> Sword {
        let quality = if guarantee_artifact {
            Quality::Artifact
        } else {
            rand::random()
        };
        let material = rand::random::<Material>();
        let (sword_type, handle) = if needle {
            (SwordType::Needle, None)
        } else {
            let handle = match quality {
                Quality::Common => None,
                Quality::Artifact => Some(rand::random()),
                _ => if rand::random::<u8>() > 128 {
                    Some(rand::random())
                } else {
                    None
                }
            };
            (rand::random(), handle)
        };

        Sword {
            id: None,
            material,
            sword_type,
            name: None,
            real_name: None,
            handle, quality, owner: owner.clone()
        }
    }

    async fn is_unique(&self, sword: &Sword) -> Result<bool, Box<dyn Error + Send + Sync>> {
        for cached in self.cache.read().await.iter() {
            if *cached == *sword {
                return Ok(false)
            }
        }
        Ok(true)
    }

    pub async fn log(&self, sword: Sword, gateway: Arc<gateway::Gateway>) {
        self.post_sword(&sword, gateway);
    }

    fn post_sword(&self, sword: &Sword, gateway: Arc<gateway::Gateway>) {
        let gateway = Arc::clone(&gateway);
        let cache = Arc::clone(&self.cache);
        let mut sword = sword.clone();
        tokio::spawn(
            async move {
                if let Ok(Some(Some(id))) = gateway.post("/armory", sword.serialize()).await.map(|v| v.map(|v| v["id"].as_i64())) {
                    sword.set_id(id)
                } else {
                    log::warn!("Failed to bestow an id onto a sword {}", sword)
                }
                cache.write().await.push(sword);
            }
        );
    }

    async fn get_swords(
        page: u32,
        gateway: Arc<gateway::Gateway>
    ) -> Result<(Vec<Sword>, bool), Box<dyn Error + Send + Sync>> {
        let params = HashMap::from([
            ("page", page.to_string()),
            ("per_page", 1000.to_string())
        ]);
        gateway.get("/armory", params).await
            .map(|json| {
                if !json.is_object() ||
                    !json.has_key("data") ||
                    !json["data"].is_array() ||
                    !json.has_key("meta") ||
                    !json["meta"].is_object() ||
                    !json["meta"].has_key("has_next") ||
                    !json["meta"]["has_next"].is_boolean()
                {
                    return Err(format!("Result is not valid: {}", json).into())
                }
                let swords = json["data"].members().map(|s| Sword::deserialize(s)).collect::<Result<Vec<Sword>, _>>()?;
                Ok((swords, json["meta"]["has_next"].as_bool().unwrap()))
            }
        ).map_err(|e| e.to_string())?
    }

    pub async fn check(&self, owner: &String, id: Option<i64>) -> (usize, Option<Sword>) {
        let mut count = 0;
        let sword = if let Some(id) = id {
            self.cache.read().await
                .iter()
                .inspect(|_| count += 1)
                .filter(|s| id == s.id.unwrap_or(-1))
                .next()
                .cloned()
        } else {
            self.cache.read().await
                .iter()
                .filter(|s| s.owner == *owner)
                .inspect(|_| count += 1)
                .collect::<Vec<&Sword>>()
                .choose(&mut rand::rng())
                .map(|s| *s)
                .cloned()
        };
        (count, sword)
    }

    pub async fn draw(&self, owner: &String, needle: bool) -> Result<Sword, Box<dyn Error + Send + Sync>> {
        let mut sword = self.roll_sword(owner, false, needle);
        if let Quality::Artifact = sword.quality {
            let res = self.bestow_name(&mut sword);
            if res.is_err() || rand::random::<u8>() == 255 {
                if res.is_err() {
                    log::error!("Failed to bestow a name: {}", res.err().unwrap());
                }
                log::info!("{} receives the rarest of gifts...", owner);
                let name = format!("{:#010X}", rand::random::<u32>());
                sword.name = Some(name);
            }

            while self.is_unique(&sword).await? {
                sword = self.roll_sword(owner, true, needle);
            }
        }
        Ok(sword)
    }

    fn bestow_name(&self, sword: &mut Sword) -> Result<(), Box<dyn Error + Send + Sync>> {
        let i1 = rand::random_range(0..LANG_SIZE);
        let mut i2 = rand::random_range(0..LANG_SIZE);
        while i2 == i1 {
            i2 = rand::random_range(0..LANG_SIZE);
        }
        let file = File::open(&self.elven)?;
        let mut first = None;
        let mut second = None;
        for (i, line) in BufReader::new(file).lines().enumerate() {
            if i == i1 || i == i2 {
                let line = line?;
                let mut word = line.split_whitespace();
                let word = (
                    word.next().ok_or("Error obtaining regular word")?.to_owned(),
                    word.next().ok_or("Error obtaining elven word")?.to_owned()
                );
                if first.is_none() {
                    first = Some(word)
                } else {
                    second = Some(word)
                }
            }
        }
        let first = first.unwrap();
        let second = second.unwrap();

        sword.name = Some(to_title_case(format!("{}{}", first.1, second.1).as_str()));
        sword.real_name = Some(to_title_case(format!("{}{}", first.0, second.0).as_str()));
        Ok(())
    }
}

#[derive(Debug, PartialEq, Clone)]
enum Material {
    Rosewood,
    Plastic,
    Glass,
    Wood,
    Porcelain,
    Iron,
    Steel,
    Silver,
    Gold,
    Electrum,
    RoseGold,
    Lead,
    Tin,
    Copper,
    Bronze,
    Brass,
    Zinc,
    Mithril,
    Ruby,
    Sapphire,
    Emerald,
    Diamond,
    Adamantine
}

impl Material {
    pub fn parse(string: Option<&str>) -> Result<Option<Material>, Box<dyn Error + Send + Sync>> {
        if string.is_none() {
            return Ok(None)
        }
        let string = string.unwrap();
        match string {
            "None" => Ok(None),
            "plastic" => Ok(Some(Material::Plastic)),
            "glass" => Ok(Some(Material::Glass)),
            "lost rosewood" => Ok(Some(Material::Rosewood)),
            "wood" => Ok(Some(Material::Wood)),
            "fine porcelain" => Ok(Some(Material::Porcelain)),
            "iron" => Ok(Some(Material::Iron)),
            "steel" => Ok(Some(Material::Steel)),
            "silver" => Ok(Some(Material::Silver)),
            "gold" => Ok(Some(Material::Gold)),
            "electrum" => Ok(Some(Material::Electrum)),
            "rose gold" => Ok(Some(Material::RoseGold)),
            "lead" => Ok(Some(Material::Lead)),
            "tin" => Ok(Some(Material::Tin)),
            "copper" => Ok(Some(Material::Copper)),
            "bronze" => Ok(Some(Material::Bronze)),
            "brass" => Ok(Some(Material::Brass)),
            "zinc" => Ok(Some(Material::Zinc)),
            "mithril" => Ok(Some(Material::Mithril)),
            "ruby" => Ok(Some(Material::Ruby)),
            "sapphire" => Ok(Some(Material::Sapphire)),
            "emerald" => Ok(Some(Material::Emerald)),
            "diamond" => Ok(Some(Material::Diamond)),
            "adamantine" => Ok(Some(Material::Adamantine)),
            _ => Err(format!("Unknown material: {}", string).into()),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
enum Quality {
    Common,
    WellCrafted,
    Fine,
    Superior,
    Exceptional,
    Masterful,
    Artifact,
}

impl Quality {
    pub fn to_mark(&self) -> &str {
        match self {
            Quality::Common => " ",
            Quality::WellCrafted => "-",
            Quality::Fine => "+",
            Quality::Superior => "*",
            Quality::Exceptional => "≡",
            Quality::Masterful => "☼",
            Quality::Artifact => "?",
        }
    }
    pub fn parse(string: Option<&str>) -> Result<Self, Box<dyn Error + Send + Sync>> {
        if string.is_none() {
            return Err("Undefined quality".into());
        }
        let string = string.unwrap();
        match string {
            " " => Ok(Quality::Common),
            "-" => Ok(Quality::WellCrafted),
            "+" => Ok(Quality::Fine),
            "*" => Ok(Quality::Superior),
            "≡" => Ok(Quality::Exceptional),
            "☼" => Ok(Quality::Masterful),
            "?" => Ok(Quality::Artifact),
            _ => Err(format!("Unknown quality mark: {}", string).into())
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
enum SwordType {
    ShortSword,
    LongSword,
    Rapier,
    Cutlass,
    Scimitar,
    Katana,
    Zweihander,
    Dagger,
    Needle,
    Tooth,
}

impl SwordType {
    pub fn parse(string: Option<&str>) -> Result<SwordType, Box<dyn Error + Send + Sync>>{
        if string.is_none() {
            return Err("Undefined sword type".into());
        }
        let string = string.unwrap();
        match string {
            "shortsword" => Ok(SwordType::ShortSword),
            "longsword" => Ok(SwordType::LongSword),
            "rapier" => Ok(SwordType::Rapier),
            "cutlass" => Ok(SwordType::Cutlass),
            "scimitar" => Ok(SwordType::Scimitar),
            "katana" => Ok(SwordType::Katana),
            "zweihander" => Ok(SwordType::Zweihander),
            "dagger" => Ok(SwordType::Dagger),
            "needle" => Ok(SwordType::Needle),
            "tooth" => Ok(SwordType::Tooth),
            _ => Err(format!("Unknown sword type: {}", string).into()),
        }
    }
}

impl Distribution<SwordType> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> SwordType {
        match rng.random_range(0..=7) {
            0 => SwordType::ShortSword,
            1 => SwordType::LongSword,
            2 => SwordType::Rapier,
            3 => SwordType::Cutlass,
            4 => SwordType::Scimitar,
            5 => SwordType::Katana,
            6 => SwordType::Zweihander,
            _ => SwordType::Dagger,
        }
    }
}

impl Distribution<Quality> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Quality {
        let value = rng.random::<f64>();
        return if value < 0.40 {
            Quality::Common
        } else if value < 0.65 {
            Quality::WellCrafted
        } else if value < 0.8 {
            Quality::Fine
        } else if value < 0.9 {
            Quality::Superior
        } else if value < 0.96 {
            Quality::Exceptional
        } else if value < 0.99 {
            Quality::Masterful
        } else {
            Quality::Artifact
        }
    }
}

impl Distribution<Material> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Material {
        match rng.random_range(0..=21) {
            0 => Material::Plastic,
            1 => Material::Glass,
            2 => Material::Wood,
            3 => Material::Porcelain,
            4 => Material::Iron,
            5 => Material::Steel,
            6 => Material::Silver,
            7 => Material::Gold,
            8 => Material::Electrum,
            9 => Material::RoseGold,
            10 => Material::Lead,
            11 => Material::Tin,
            12 => Material::Copper,
            13 => Material::Bronze,
            14 => Material::Brass,
            15 => Material::Zinc,
            16 => Material::Mithril,
            17 => Material::Ruby,
            18 => Material::Sapphire,
            19 => Material::Emerald,
            20 => Material::Diamond,
            _ => Material::Adamantine,
        }
    }
}

impl fmt::Display for SwordType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let stype = match *self {
            SwordType::ShortSword => "shortsword",
            SwordType::LongSword => "longsword",
            SwordType::Rapier => "rapier",
            SwordType::Cutlass => "cutlass",
            SwordType::Scimitar => "scimitar",
            SwordType::Katana => "katana",
            SwordType::Zweihander => "zweihander",
            SwordType::Dagger => "dagger",
            SwordType::Needle => "needle",
            SwordType::Tooth => "tooth",
        };
        write!(f, "{}", stype)
    }
}

impl fmt::Display for Material {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let material = match *self {
            Material::Plastic => "plastic",
            Material::Glass => "glass",
            Material::Wood => "wood",
            Material::Rosewood => "lost rosewood",
            Material::Porcelain=> "fine porcelain",
            Material::Iron => "iron",
            Material::Steel => "steel",
            Material::Silver => "silver",
            Material::Gold => "gold",
            Material::Electrum => "electrum",
            Material::RoseGold => "rose gold",
            Material::Lead => "lead",
            Material::Tin => "tin",
            Material::Copper => "copper",
            Material::Bronze => "bronze",
            Material::Brass => "brass",
            Material::Zinc => "zinc",
            Material::Mithril => "mithril",
            Material::Ruby => "ruby",
            Material::Sapphire => "sapphire",
            Material::Emerald => "emerald",
            Material::Diamond => "diamond",
            Material::Adamantine => "adamantine"
        };
        write!(f, "{}", material)
    }
}

#[derive(Debug, Clone)]
pub struct Sword {
    pub id: Option<i64>,
    material: Material,
    handle: Option<Material>,
    sword_type: SwordType,
    quality: Quality,
    name: Option<String>,
    real_name: Option<String>,
    pub owner: String
}

impl Sword {
    fn parse_name(string: Option<&str>) -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
        if string.is_none() {
            return Err("Undefined name string".into());
        }
        let string = string.unwrap();
        if string == "None" {
            Ok(None)
        } else {
            Ok(Some(string.to_owned()))
        }
    }

    pub fn set_id(&mut self, id: i64) {
        self.id = Some(id);
    }

    pub fn serialize(&self) -> json::JsonValue {
        object!(
            material: self.material.to_string(),
            handle: self.handle.clone().map(|m| m.to_string()),
            sword_type: self.sword_type.to_string(),
            quality: self.quality.to_mark(),
            name: self.name.clone(),
            real_name: self.real_name.clone(),
            owner: self.owner.clone()
        )
    }

    pub fn deserialize(json: &json::JsonValue) -> Result<Sword, Box<dyn Error + Send + Sync>> {
        Ok(Sword {
            id: Some(json["id"].as_i64().ok_or("No identifier")?),
            material: Material::parse(json["material"].as_str())?.ok_or("Main material cannot be none")?,
            handle: Material::parse(json["handle"].as_str())?,
            sword_type: SwordType::parse(json["sword_type"].as_str())?,
            quality: Quality::parse(json["quality"].as_str())?,
            name: json["name"].as_str().map(str::to_owned),
            real_name:json["real_name"].as_str().map(str::to_owned),
            owner: json["owner"].as_str().map(str::to_owned).ok_or("Owner name is missing")?,
        })
    }

}

impl std::cmp::PartialEq for Sword {
    fn eq(&self, other: &Self) -> bool {
        self.material == other.material
            && self.handle == other.handle
            && self.sword_type == other.sword_type
            && self.quality == other.quality
    }
}

impl fmt::Display for Sword {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let proper_article = if is_vowel(self.material.to_string().chars().nth(0)) {
            "an"
        } else {
            "a"
        };
        let sword = match self.quality {
            Quality::Common => format!("{} {} {}", proper_article, self.material, self.sword_type),
            Quality::WellCrafted => format!("{} well-crafted -{} {}-", proper_article, self.material, self.sword_type),
            Quality::Fine => format!("a finely-crafted +{} {}+", self.material, self.sword_type),
            Quality::Superior => format!("{} *{} {}* of superior quality", proper_article, self.material, self.sword_type),
            Quality::Exceptional => format!("an exceptional ≡{} {}≡", self.material, self.sword_type),
            Quality::Masterful => format!("a masterwork ☼{} {}☼", self.material, self.sword_type),
            Quality::Artifact =>
                format!("The \"{}\" ({}), one of a kind {} {}, is of the highest quality",
                    self.name.as_ref().unwrap(), self.real_name.as_ref().unwrap(),
                    self.material, self.sword_type),
        };
        let handle = if self.sword_type == SwordType::Needle {
            String::new()
        } else if let Some(handle) = self.handle.as_ref() {
            format!(". Its handle is adorned with {}", handle)
        } else {
            String::new()
        };
        write!(f, "{}{}", sword, handle)
    }
}

const VOWELS: [char; 5] = ['a', 'e', 'i', 'o', 'u'];
fn is_vowel(character: Option<char>) -> bool {
    if let Some(character) = character {
        VOWELS.iter().any(|&v| v == character)
    } else {
        false
    }
}
