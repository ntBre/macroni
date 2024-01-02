//! macro tracker

use std::{
    error::Error,
    io::{self, stdout, Write},
    path::Path,
    str::FromStr,
};

use crossterm::{
    cursor::{self, MoveTo},
    event::{read, Event, KeyCode},
    terminal::{self, disable_raw_mode, enable_raw_mode, Clear, ClearType},
    ExecutableCommand, QueueableCommand,
};

#[allow(unused)]
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

/// draw a bounding box around the whole window with unicode light box drawing
/// characters. TODO factor out the code to draw any rectangle
fn draw_boundary<W>(w: &mut W) -> io::Result<()>
where
    W: QueueableCommand + Write,
{
    let (cols, rows) = terminal::size()?;
    let rows = rows - 3; // reserve for command help

    // top bar + top corners
    w.queue(MoveTo(0, 0))?.write_all("┌".as_bytes())?;
    for x in 1..cols - 1 {
        w.queue(MoveTo(x, 0))?.write_all("─".as_bytes())?;
    }
    w.queue(MoveTo(cols, 0))?.write_all("┐".as_bytes())?;

    // sides
    for y in 1..rows {
        w.queue(MoveTo(0, y))?.write_all("│".as_bytes())?;
        w.queue(MoveTo(cols, y))?.write_all("│".as_bytes())?;
    }

    // bottom bar + bottom corners
    w.queue(MoveTo(0, rows))?.write_all("└".as_bytes())?;
    for x in 1..cols - 1 {
        w.queue(MoveTo(x, rows))?.write_all("─".as_bytes())?;
    }
    w.queue(MoveTo(cols, rows))?.write_all("┘".as_bytes())?;

    Ok(())
}

fn main() -> io::Result<()> {
    let path = "foods";
    let foods = load_foods(path);

    let mut stdout = stdout();
    stdout.execute(Clear(ClearType::All))?;

    draw_boundary(&mut stdout)?;

    let (cols, rows) = terminal::size()?;
    for (i, food) in foods.iter().enumerate() {
        stdout.queue(cursor::MoveTo(cols / 2, rows / 2 + i as u16))?;
        stdout.write_all(food.name.as_bytes())?;
    }
    stdout.flush()?;

    enable_raw_mode()?;

    loop {
        match read()? {
            Event::FocusGained => eprintln!("FocusGained"),
            Event::FocusLost => eprintln!("FocusLost"),
            Event::Key(event) if event.code == KeyCode::Char('q') => {
                break;
            }
            Event::Key(event) => eprintln!("{:?}", event),
            Event::Mouse(event) => eprintln!("{:?}", event),
            Event::Paste(data) => eprintln!("{:?}", data),
            Event::Resize(width, height) => {
                eprintln!("New size {}x{}", width, height)
            }
        }
    }

    disable_raw_mode()?;

    stdout.execute(Clear(ClearType::All))?;
    stdout.flush()?;

    Ok(())
}
