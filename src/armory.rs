use std::fmt;
use std::error::Error;
use std::path::PathBuf;

use std::io::{BufReader, BufRead, Write};
use std::fs::{File, OpenOptions, self};

use cruet::to_title_case;
use tokio::sync::RwLock;
use rand::{
    distr::{Distribution, StandardUniform},
    Rng
};

const LANG_SIZE: usize = 2222;
const SEPARATOR: &str = "|";

pub struct Swords {
    swords: RwLock<PathBuf>,
    elven: PathBuf
}

impl Swords {
    pub async fn new(swords: PathBuf, elven: PathBuf) -> Result<Self, Box<dyn Error + Send + Sync>> {
        Ok(Self {
            elven,
            swords: RwLock::new(swords)
        })
    }

    fn roll_sword(&self, owner: &String, guarantee_artifact: bool) -> Sword {
        let quality = if guarantee_artifact {
            Quality::Artifact
        } else {
            rand::random()
        };
        let handle = match quality {
            Quality::Common => Material::Wood,
            _ => rand::random()
        };

        Sword {
            material: rand::random(),
            sword_type: rand::random(),
            name: None,
            real_name: None,
            handle, quality, owner: owner.clone()
        }
    }

    async fn is_unique(&self, sword: &Sword) -> Result<bool, Box<dyn Error + Send + Sync>> {
        let swords = self.swords.read().await;
        for (n, sword_db) in fs::read_to_string(&*swords)?.lines().enumerate() {
            match Sword::deserialize(sword_db) {
                Ok(sword_db) => {
                    if sword_db == *sword {
                        return Ok(false)
                    }
                }
                Err(e) => {
                    log::error!("Error parsing sword at {}: {}", n, e);
                }
            }
        }
        Ok(true)
    }

    pub async fn log(&self, sword: Sword) -> Result<(), Box<dyn Error + Send + Sync>> {
        let swords = self.swords.write().await;
        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .open(&*swords)?;

        writeln!(file, "{}", sword.serialize()).map_err(|e| e.to_string().into())
    }

    pub async fn check(&self, owner: &String) -> Result<(usize, Option<Sword>), Box<dyn Error + Send + Sync>> {
        let swords = {
            let swords = self.swords.read().await;
            fs::read_to_string(&*swords)?
                .lines()
                .filter_map(|line| Sword::deserialize(line).ok())
                .filter(|sword| sword.owner == *owner)
                .collect::<Vec<Sword>>()
        };
        if swords.len() == 0 {
            return Ok((0, None));
        }
        let index = rand::random_range(0..swords.len());
        let example = swords[index].clone();
        Ok((swords.len(), Some(example)))
    }

    pub async fn draw(&self, owner: &String) -> Result<Sword, Box<dyn Error + Send + Sync>> {
        let mut sword = self.roll_sword(owner, false);
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
                sword = self.roll_sword(owner, true);
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
    pub fn parse(string: Option<&str>) -> Result<Material, Box<dyn Error + Send + Sync>> {
        if string.is_none() {
            return Err("Undefined material".into());
        }
        let string = string.unwrap();
        match string {
            "plastic" => Ok(Material::Plastic),
            "glass" => Ok(Material::Glass),
            "wood" => Ok(Material::Wood),
            "fine porcelain" => Ok(Material::Porcelain),
            "iron" => Ok(Material::Iron),
            "steel" => Ok(Material::Steel),
            "silver" => Ok(Material::Silver),
            "gold" => Ok(Material::Gold),
            "electrum" => Ok(Material::Electrum),
            "rose gold" => Ok(Material::RoseGold),
            "lead" => Ok(Material::Lead),
            "tin" => Ok(Material::Tin),
            "copper" => Ok(Material::Copper),
            "bronze" => Ok(Material::Bronze),
            "brass" => Ok(Material::Brass),
            "zinc" => Ok(Material::Zinc),
            "mithril" => Ok(Material::Mithril),
            "ruby" => Ok(Material::Ruby),
            "sapphire" => Ok(Material::Sapphire),
            "emerald" => Ok(Material::Emerald),
            "diamond" => Ok(Material::Diamond),
            "adamantine" => Ok(Material::Adamantine),
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
            _ => Err(format!("Unknownsword type: {}", string).into()),
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
    material: Material,
    handle: Material,
    sword_type: SwordType,
    quality: Quality,
    name: Option<String>,
    real_name: Option<String>,
    owner: String
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

    pub fn serialize(&self) -> String {
        [
            self.material.to_string(),
            self.handle.to_string(),
            self.sword_type.to_string(),
            self.quality.to_mark().to_owned(),
            self.name.clone().unwrap_or("None".to_owned()),
            self.real_name.clone().unwrap_or("None".to_owned()),
            self.owner.clone(),
        ].join(SEPARATOR)
    }

    pub fn deserialize(string: &str) -> Result<Sword, Box<dyn Error + Send + Sync>> {
        let mut data = string.split(SEPARATOR);
        Ok(Sword {
            material: Material::parse(data.next())?,
            handle: Material::parse(data.next())?,
            sword_type: SwordType::parse(data.next())?,
            quality: Quality::parse(data.next())?,
            name: Self::parse_name(data.next())?,
            real_name: Self::parse_name(data.next())?,
            owner: data.next().ok_or("Undefined owner")?.to_owned()
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
        let sword = match self.quality {
            Quality::Common => format!("a {} {}", self.material, self.sword_type),
            Quality::WellCrafted => format!("a well-crafted -{} {}-", self.material, self.sword_type),
            Quality::Fine => format!("a finely-crafted +{} {}+", self.material, self.sword_type),
            Quality::Superior => format!("a *{} {}* of superior quality", self.material, self.sword_type),
            Quality::Exceptional => format!("an exceptional ≡{} {}≡", self.material, self.sword_type),
            Quality::Masterful => format!("a masterwork ☼{} {}☼", self.material, self.sword_type),
            Quality::Artifact =>
                format!("The \"{}\" ({}), one of a kind {} {}, is of the highest quality",
                    self.name.as_ref().unwrap(), self.real_name.as_ref().unwrap(),
                    self.material, self.sword_type),
        };

        let handle = format!("It's handle is made out of {}", self.handle);
        write!(f, "{}. {}.", sword, handle)
    }
}
