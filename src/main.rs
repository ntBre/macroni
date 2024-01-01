//! macro tracker

use std::{error::Error, path::Path, str::FromStr};

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

fn load_foods(path: impl AsRef<Path>) -> Vec<Food> {
    let s = std::fs::read_to_string(path).unwrap();
    let foods: Vec<Food> = s
        .lines()
        .filter_map(|line| {
            if line.starts_with('#') {
                return None;
            }
            line.parse().ok()
        })
        .collect();
    foods
}

// Basic Interface:
// 1. Search for foods in database (fuzzy search ideal)
// 2. Select quantity in saved units
// 3. Add to totals for the current day
//
// For example, I want to be able to say "3 hamburger buns, 3 slices of cheese,
// and 16 oz of cooked ground beef" for my dinner and see the macro information
// for that. I'm not particularly concerned about breaking it up by meals, but I
// do need it by day
//
// Interface enhancements:
// 1. Edit/Delete previous entries
// 2. Navigate between dates
//
// Other enhancements:
// 1. Use a real database, not a tsv file

fn main() {
    let path = "foods";
    let foods = load_foods(path);
    dbg!(foods);
}
