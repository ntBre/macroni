//! macro tracker

use std::{error::Error, str::FromStr};

#[derive(Debug)]
struct Food {
    name: String,
    calories: f64,
    carbs: f64,
    fat: f64,
    protein: f64,
    unit: String,
}

impl FromStr for Food {
    type Err = Box<dyn Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let fields: Vec<&str> = s.split('\t').collect();
        if fields.len() != 6 {
            Err("invalid field number")?;
        }
        Ok(Self {
            name: fields[0].to_owned(),
            calories: fields[1].parse()?,
            carbs: fields[2].parse()?,
            fat: fields[3].parse()?,
            protein: fields[4].parse()?,
            unit: fields[5].to_owned(),
        })
    }
}

fn main() {
    let s = std::fs::read_to_string("foods").unwrap();
    let foods: Vec<Food> = s
        .lines()
        .map(|line| {
            if line.starts_with('#') {
                return None;
            }
            line.parse().ok()
        })
        .flatten()
        .collect();
    dbg!(foods);
}
