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

struct Tui<'a, W> {
    w: &'a mut W,
    cols: u16,
    rows: u16,
    foods: Vec<Food>,
}

impl<'a, W> Write for Tui<'a, W>
where
    W: QueueableCommand + Write,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.w.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.w.flush()
    }
}

const HELP_HEIGHT: u16 = 3;
const HELP_PAD: u16 = 5;

impl<'a, W> Tui<'a, W>
where
    W: QueueableCommand + Write,
{
    fn new(w: &'a mut W, foods: Vec<Food>) -> Self {
        let (cols, rows) = terminal::size().unwrap();
        Self {
            w,
            cols,
            rows,
            foods,
        }
    }

    /// calls `write_all` but also returns the number of chars written
    fn write_str(&mut self, s: &str) -> io::Result<usize> {
        let ret = s.chars().count();
        self.write_all(s.as_bytes())?;
        Ok(ret)
    }

    fn resize(&mut self, w: u16, h: u16) {
        self.cols = w;
        self.rows = h;
    }

    /// draw a bounding box around the whole window with unicode light box
    /// drawing characters. TODO factor out the code to draw any rectangle
    fn draw_boundary(&mut self) -> io::Result<()> {
        let rows = self.rows - HELP_HEIGHT; // reserve for command help

        // top bar + top corners
        self.queue(MoveTo(0, 0))?.write_all("┌".as_bytes())?;
        for x in 1..self.cols - 1 {
            self.queue(MoveTo(x, 0))?.write_all("─".as_bytes())?;
        }
        self.queue(MoveTo(self.cols, 0))?
            .write_all("┐".as_bytes())?;

        // sides
        for y in 1..rows {
            self.queue(MoveTo(0, y))?.write_all("│".as_bytes())?;
            self.w
                .queue(MoveTo(self.cols, y))?
                .write_all("│".as_bytes())?;
        }

        // bottom bar + bottom corners
        self.queue(MoveTo(0, rows))?.write_all("└".as_bytes())?;
        for x in 1..self.cols - 1 {
            self.queue(MoveTo(x, rows))?.write_all("─".as_bytes())?;
        }
        self.queue(MoveTo(self.cols, rows))?
            .write_all("┘".as_bytes())?;

        self.flush()?;

        Ok(())
    }

    /// draw the help menu at the bottom of the screen
    fn draw_help(&mut self) -> io::Result<()> {
        self.queue(MoveTo(1, self.rows - HELP_HEIGHT + 1))?;
        let n = self.write_str("q Quit")?;
        self.queue(MoveTo(
            1 + n as u16 + HELP_PAD,
            self.rows - HELP_HEIGHT + 1,
        ))?;
        self.write_str("a Add Food")?;

        self.flush()?;
        Ok(())
    }

    fn render(&mut self) -> io::Result<()> {
        self.execute(Clear(ClearType::All))?;
        self.draw_boundary()?;
        self.draw_help()
    }

    fn add_food(&mut self) -> io::Result<()> {
        let (cols, rows) = terminal::size()?;
        // this is so stupid, just to avoid the double borrow
        let foods = std::mem::take(&mut self.foods);
        for (i, food) in foods.iter().enumerate() {
            self.queue(cursor::MoveTo(cols / 2, rows / 2 + i as u16))?;
            self.write_all(food.name.as_bytes())?;
        }
        self.flush()?;
        self.foods = foods;
        Ok(())
    }
}

fn main() -> io::Result<()> {
    let path = "foods";
    let foods = load_foods(path);

    let mut stdout = stdout();
    let mut tui = Tui::new(&mut stdout, foods);

    tui.render()?;

    enable_raw_mode()?;

    loop {
        match read()? {
            Event::FocusGained => eprintln!("FocusGained"),
            Event::FocusLost => eprintln!("FocusLost"),
            Event::Key(event) if event.code == KeyCode::Char('q') => break,
            Event::Key(event) if event.code == KeyCode::Char('a') => {
                tui.add_food()?;
            }
            Event::Key(event) => eprintln!("{:?}", event),
            Event::Mouse(event) => eprintln!("{:?}", event),
            Event::Paste(data) => eprintln!("{:?}", data),
            Event::Resize(width, height) => {
                tui.resize(width, height);
                tui.render()?;
            }
        }
    }

    disable_raw_mode()?;

    tui.execute(Clear(ClearType::All))?;
    tui.flush()?;

    Ok(())
}
