use std::fmt;
use rand::{
    distr::{Distribution, StandardUniform},
    Rng,
};

enum Material {
    Plastic,
    Glass,
    Wood,
    Bone,
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

enum Quality {
    Common,
    WellCrafted,
    Fine,
    Superior,
    Exceptional,
    Masterful,
    Artifact,
}

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
        match rng.random_range(0..=22) {
            0 => Material::Plastic,
            1 => Material::Glass,
            2 => Material::Wood,
            3 => Material::Bone,
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
            21 => Material::Adamantine,
            _ => panic!()
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
            Material::Bone => "bone",
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

pub struct Sword {
    material: Material,
    handle: Material,
    sword_type: SwordType,
    quality: Quality,
    name: Option<String>
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
                format!("the \"{}\", one of a kind {} {}. All craftsdwarfship is of the highest quality",
                    self.name.as_ref().unwrap(), self.material, self.sword_type),
        };

        let handle = format!("It's handle is made out of {}", self.handle);
        write!(f, "{}. {}.", sword, handle)
    }
}

pub fn draw<R: Rng>(rng: &mut R) -> Sword {
    let quality = rng.random();
    let handle = if let Quality::Common = quality {
        Material::Wood
    } else {
        rng.random()
    };
    let name = if let Quality::Artifact = quality {
        Some("TODO".to_owned())
    } else {
        None
    };
    Sword {
        material: rng.random(),
        sword_type: rng.random(),
        handle,
        quality,
        name
    }
}
