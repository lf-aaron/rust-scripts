// use std::collections::HashMap;
// use hmap::hmap;
use itertools;
use std::{iter::{Iterator}, slice::Iter, collections::HashMap};
use itertools::Itertools;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum Modifier {
  FrameStyle,
  TriggerGuardMod,
  Caliber,
  // SlideCaliber,
  BarrelLength,
  DustCoverMod,
  RearCutMod,
  GripModRear,
  GripModFront,
  SlideMod,
  TriggerMod,
  SightType,
  BarrelMod,
  GripType
}


fn modifier_values(modifier: &Modifier) -> Vec<Option<&'static str>> {
  match modifier {
    Modifier::FrameStyle => vec![ Some("Std"), Some("Tac") ],
    Modifier::TriggerGuardMod => vec![ None, Some("Sqr") ],
    Modifier::Caliber => vec![ Some("45"), Some("9mm"), Some("10mm"), Some("38"), Some("40") ],
    // Modifier::SlideCaliber => vec![ Some("45"), Some("9mm") ],
    Modifier::BarrelLength => vec![ Some("Gov"), Some("Com") ],
    Modifier::DustCoverMod => vec![ None, Some("Ext") ],
    Modifier::RearCutMod => vec![ None, Some("Bob") ],
    Modifier::GripModRear => vec![ None, Some("RChk"), Some("RChain") ],
    Modifier::GripModFront => vec![ None, Some("FChk") ],
    Modifier::SlideMod => vec![ None, Some("Dual"), Some("Tri") ],
    Modifier::TriggerMod => vec![ None, Some("Skeleton") ],
    Modifier::SightType => vec![ Some("Novak"), Some("Tritium-1"), Some("Tritium-2"), Some("Fiber-Optic") ],
    Modifier::BarrelMod => vec![ None ],
    Modifier::GripType => vec![ Some("Smooth"), Some("Checkered") ],
  }
}

impl Modifier {
  fn default(&self) -> Option<&str> {
    *modifier_values(self).get(0).unwrap()
  }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Section {
  Barrel,
  Grips,
  MSH,
  Nose,
  Rear,
  Sights,
  Slide,
  Trigger
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum SectionType {
  Upper,
  Lower
}

impl Section {
  pub fn iterator() -> Iter<'static, Section> {
    static sections: [Section; 8] = [
      Section::Barrel,
      Section::Grips,
      Section::MSH,
      Section::Nose,
      Section::Rear,
      Section::Sights,
      Section::Slide,
      Section::Trigger
    ];
    sections.iter()
  }

  pub fn name(&self) -> &'static str {
    match self {
      Self::Barrel => "Barrel",
      Self::Grips => "Grips",
      Self::MSH => "MSH",
      Self::Nose => "Nose",
      Self::Rear => "Rear",
      Self::Sights => "Sights",
      Self::Slide => "Slide",
      Self::Trigger => "Trigger",
    }
  }

  pub fn matching_config(&self, config: &Vec<(Modifier, Option<&str>)>) -> String {
    let mods = section_mods(self);
    let mut values = vec![];
    for modifier in &mods {
      let mut value = match config.iter().find(|x| x.0 == *modifier) {
        Some(v) => apply_fallbacks(self, v.1),
        None => modifier.default()
      };
      values.push(value);
    }

    return config_name(self.name(), values);
    // mods.iter().map(|x| modifier_values(x).iter().map(|y| apply_fallbacks(self, y)));
  }

  pub fn section_type(&self) -> SectionType {
    match self {
      Self::Barrel => SectionType::Upper,
      Self::Grips => SectionType::Lower,
      Self::MSH => SectionType::Lower,
      Self::Nose => SectionType::Lower,
      Self::Rear => SectionType::Lower,
      Self::Sights => SectionType::Upper,
      Self::Slide => SectionType::Upper,
      Self::Trigger => SectionType::Lower,
    }
  }

  pub fn configurations(&self) -> Vec<String> {
    let mut mods = section_mods(self);

    let values = mods.iter().map(|x| {
      if *self == Self::Slide && *x == Modifier::Caliber { modifier_values(x)[0..2].to_vec() }
      else { modifier_values(x) }
    });
    
    let product = values.multi_cartesian_product();
    let config_names = product.map(|x| config_name(self.name(), x)).collect::<Vec<String>>();
    return config_names;
  }
}

type Mod = Modifier;

fn section_mods(section: &Section) -> Vec<Mod> {
  let mut mods = match section {
    Section::Barrel => vec![Mod::Caliber, Mod::BarrelLength, Mod::BarrelMod],
    Section::Grips => vec![Mod::RearCutMod, Mod::GripType],
    Section::MSH => vec![Mod::RearCutMod, Mod::GripModRear],
    Section::Nose => vec![Mod::FrameStyle, Mod::TriggerGuardMod, Mod::BarrelLength, Mod::DustCoverMod],
    Section::Rear => vec![Mod::RearCutMod, Mod::GripModFront],
    Section::Sights => vec![Mod::BarrelLength, Mod::SightType],
    Section::Slide => vec![Mod::Caliber, Mod::BarrelLength, Mod::DustCoverMod, Mod::SlideMod],
    Section::Trigger => vec![Mod::TriggerMod],
  };
  mods.sort();
  mods
}

fn section_type_mods(section_type: &SectionType) -> Vec<Mod> {
  let mut mods = match section_type {
    SectionType::Upper => vec![Mod::Caliber, Mod::BarrelLength, Mod::DustCoverMod, Mod::SightType, Mod::SlideMod, Mod::BarrelMod],
    SectionType::Lower => vec![Mod::RearCutMod, Mod::GripType, Mod::TriggerGuardMod, Mod::FrameStyle, Mod::BarrelLength, Mod::DustCoverMod, Mod::GripModRear, Mod::GripModFront, Mod::TriggerMod],
  };
  mods.sort();
  mods
}

pub fn config_name(section_name: &str, values: Vec<Option<&str>>) -> String {
  let mut name = section_name.to_owned() + "-" + values.iter().filter_map(|&x| x).collect::<Vec<&str>>().join("-").as_str();
  name.make_ascii_lowercase();
  return name;
}

impl SectionType {
  pub fn iterator() -> Iter<'static, SectionType> {
    static sections: [SectionType; 2] = [
      SectionType::Upper,
      SectionType::Lower,
    ];
    sections.iter()
  }

  pub fn name(&self) -> &'static str {
    match self {
      Self::Upper => "Upper",
      Self::Lower => "Lower",
    }
  }

  pub fn configurations(&self) -> Vec<Vec<(Modifier, Option<&str>)>> {
    let mods = section_type_mods(self);
    let values = mods.iter().map(|x| modifier_values(x).iter().map(|y| (x.clone(), *y)).collect::<Vec<(Modifier, Option<&str>)>>());
    values.multi_cartesian_product().collect::<Vec<Vec<(Modifier, Option<&str>)>>>()
    // let config_names = product.map(|x| config_name(self.name(), x)).collect::<Vec<String>>();
    // return config_names;
  }
}

// const fallbacks: HashMap<&str, HashMap<Option<&str>, Option<&str>>> = HashMap::from([
//   (Section::Slide.name(), HashMap::from([
//     (Some("10mm"), Some("9mm")),
//     (Some("38"), Some("9mm")),
//     (Some("40"), Some("9mm")),
//   ]))
// ]);

fn apply_fallbacks<'a>(section: &Section, value: Option<&'a str>) -> Option<&'a str> {
  // match fallbacks.get(section.name()) {
  //   None => value,
  //   Some(x) => match x.get(&value) {
  //     None => value,
  //     Some(y) => y.to_owned(),
  //   }
  // }
  match section {
    Section::Slide => {
      match value {
        Some("10mm") => Some("9mm"),
        Some("38") => Some("9mm"),
        Some("40") => Some("9mm"),
        _ => value,
      }
    }
    _ => value,
  }
}
