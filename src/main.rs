//! macro tracker

use std::{
    error::Error,
    io::{self, stdout, Write},
    ops::{AddAssign, Mul},
    path::Path,
    str::FromStr,
};

use crossterm::{
    cursor::{self, MoveDown, MoveLeft, MoveTo, MoveUp},
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

struct FoodQuantity(Food, f64);

impl TryFrom<&[String; 7]> for FoodQuantity {
    type Error = Box<dyn Error>;

    fn try_from(value: &[String; 7]) -> Result<Self, Self::Error> {
        Ok(FoodQuantity(
            Food {
                name: value[0].to_owned(),
                calories: value[1].parse()?,
                carbs: value[2].parse()?,
                fat: value[3].parse()?,
                protein: value[4].parse()?,
                unit: value[5].to_owned(),
            },
            value[6].parse()?,
        ))
    }
}

impl AddAssign<Food> for Macros {
    fn add_assign(&mut self, rhs: Food) {
        self.calories += rhs.calories;
        self.protein += rhs.protein;
        self.carbs += rhs.carbs;
        self.fat += rhs.fat;
    }
}

impl Mul<f64> for Food {
    type Output = Food;

    fn mul(self, rhs: f64) -> Self::Output {
        Self {
            calories: self.calories * rhs,
            carbs: self.carbs * rhs,
            fat: self.fat * rhs,
            protein: self.protein * rhs,
            ..self
        }
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

#[derive(Default)]
struct Macros {
    calories: f64,
    carbs: f64,
    fat: f64,
    protein: f64,
}

/// the current state of the program
enum State {
    Main,
    AddFood,
}

impl State {
    /// Returns `true` if the state is [`AddFood`].
    ///
    /// [`AddFood`]: State::AddFood
    #[must_use]
    fn is_add_food(&self) -> bool {
        matches!(self, Self::AddFood)
    }
}

#[allow(unused)]
struct Tui<'a, W> {
    w: &'a mut W,
    cols: u16,
    rows: u16,
    foods: Vec<Food>,
    today: Macros,
    buf: [String; 7],
    state: State,
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
        const S: String = String::new();
        Self {
            w,
            cols,
            rows,
            foods,
            today: Macros::default(),
            state: State::Main,
            buf: [S; 7], // this has to be the same as the fields in Food + 1
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

    /// return the center of the screen
    fn center(&self) -> (u16, u16) {
        (self.cols / 2, self.rows / 2)
    }

    /// queue up a MoveTo command to x, y
    fn move_to(&mut self, x: u16, y: u16) -> io::Result<()> {
        self.queue(MoveTo(x, y))?;
        Ok(())
    }

    /// draw a bounding box around the whole window with unicode light box
    /// drawing characters. TODO factor out the code to draw any rectangle
    fn draw_boundary(&mut self) -> io::Result<()> {
        let (x1, y1) = (0, 0);
        let (x2, y2) = (self.cols, self.rows - HELP_HEIGHT);

        self.draw_rect(x1, y1, x2, y2)?;

        self.flush()?;

        Ok(())
    }

    /// draw the rectangle from the upper left corner (x1, y1) to the bottom
    /// right corner (x2, y2)
    fn draw_rect(
        &mut self,
        x1: u16,
        y1: u16,
        x2: u16,
        y2: u16,
    ) -> Result<(), io::Error> {
        for x in x1 + 1..x2 {
            self.queue(MoveTo(x, y1))?.write_all("─".as_bytes())?;
            self.queue(MoveTo(x, y2))?.write_all("─".as_bytes())?;
        }
        for y in y1 + 1..y2 {
            self.queue(MoveTo(x1, y))?.write_all("│".as_bytes())?;
            self.w.queue(MoveTo(x2, y))?.write_all("│".as_bytes())?;
        }
        self.queue(MoveTo(x1, y1))?.write_all("┌".as_bytes())?;
        self.queue(MoveTo(x2, y1))?.write_all("┐".as_bytes())?;
        self.queue(MoveTo(x1, y2))?.write_all("└".as_bytes())?;
        self.queue(MoveTo(x2, y2))?.write_all("┘".as_bytes())?;
        Ok(())
    }

    /// draw the help menu at the bottom of the screen
    fn draw_help(&mut self, labels: &[&str]) -> io::Result<()> {
        let mut n = 0;
        for (i, label) in labels.iter().enumerate() {
            self.move_to(
                1 + n as u16 + i as u16 * HELP_PAD,
                self.rows - HELP_HEIGHT + 1,
            )?;
            n += self.write_str(label)?;
        }
        self.flush()?;
        Ok(())
    }

    fn draw_today(&mut self) -> io::Result<()> {
        let (x, y) = self.center();
        let s = format!(
            "Calories: {:.0} Protein: {:.0} Carbs: {:.0} Fat: {:.0}",
            self.today.calories,
            self.today.protein,
            self.today.carbs,
            self.today.fat
        );
        let x = x - s.len() as u16 / 2;
        self.queue(MoveTo(x, y))?;
        self.write_str("Today:")?;
        self.move_to(x, y + 1)?;
        self.write_str(&s)?;
        self.flush()?;
        Ok(())
    }

    fn render_main(&mut self) -> io::Result<()> {
        self.state = State::Main;
        self.execute(cursor::Hide)?;
        self.execute(Clear(ClearType::All))?;
        self.draw_boundary()?;
        self.draw_help(&["q Quit", "a Add Food"])?;
        self.draw_today()
    }

    fn add_food(&mut self) -> io::Result<()> {
        self.execute(Clear(ClearType::All))?;
        self.draw_boundary()?;
        self.draw_help(&[
            "Tab Next",
            "S-Tab Prev",
            "Ret Submit",
            "Esc Cancel",
        ])?;
        self.state = State::AddFood;

        // the idea here is to replicate an HTML form essentially:
        //
        // Food Name: [___________________]
        //  Calories: [___________________]
        //
        // and so on, with Tab moving between the fields. We'll also need to
        // show the cursor again here. Basics are actually easy, showing the
        // completion candidates will be most of the work.

        const LABELS: [&str; 7] = [
            "Food Name:",
            " Calories:",
            "  Protein:",
            "    Carbs:",
            "      Fat:",
            "    Units:",
            " Quantity:",
        ];
        const MAX_WIDTH: u16 = 10;
        const INPUT_WIDTH: u16 = 50;

        // so we want to center 10 + 50 + 1 characters in the width of the
        // screen, and there are going to be 6 lines: 5 labels + accept

        let x = self.cols / 2 - (MAX_WIDTH + INPUT_WIDTH + 1) / 2;
        let y = self.rows / 2 - (3 * LABELS.len() + 1) as u16 / 2;

        for (i, label) in LABELS.iter().enumerate() {
            let i = 3 * i as u16;
            self.move_to(x, y + i)?;
            self.write_str(label)?;
            self.draw_rect(
                x + MAX_WIDTH + 1,
                y + i - 1,
                x + MAX_WIDTH + 1 + INPUT_WIDTH,
                y + i + 1,
            )?;
        }

        // move the cursor into the first box and show it
        self.move_to(x + MAX_WIDTH + 2, y)?;
        self.queue(cursor::Show)?;

        // let (cols, rows) = terminal::size()?;
        // // this is so stupid, just to avoid the double borrow
        // let foods = std::mem::take(&mut self.foods);
        // for (i, food) in foods.iter().enumerate() {
        //     self.queue(cursor::MoveTo(cols / 2, rows / 2 + i as u16))?;
        //     self.write_all(food.name.as_bytes())?;
        // }
        // self.foods = foods;

        self.flush()?;
        Ok(())
    }

    fn food_form(
        &mut self,
        event: crossterm::event::KeyEvent,
        right: &mut u16,
        field: &mut u16,
    ) -> Result<(), io::Error> {
        match event.code {
            KeyCode::Char(c) => {
                self.write_all(&[c as u8])?;
                self.buf[*field as usize].push(c);
                *right += 1;
                self.flush()?;
            }
            KeyCode::Backspace => {
                self.write_all(&[0x08, 0x20, 0x08])?;
                self.buf[*field as usize].pop();
                *right -= 1;
                self.flush()?;
            }
            KeyCode::Tab => {
                if *field < self.buf.len() as u16 - 1 {
                    *field += 1;
                    self.execute(MoveDown(3))?;
                    if *right != 0 {
                        // 0 defaults to 1...
                        self.execute(MoveLeft(*right))?;
                    }
                    // zero actually isn't right here or in backtab. I need to
                    // maintain the length of each field
                    *right = 0;
                }
            }
            KeyCode::BackTab => {
                if *field > 0 {
                    *field -= 1;
                    self.execute(MoveUp(3))?;
                    if *right != 0 {
                        // 0 defaults to 1...
                        self.execute(MoveLeft(*right))?;
                    }
                    *right = 0;
                }
            }
            KeyCode::Enter => {
                if let Ok(FoodQuantity(food, n)) =
                    FoodQuantity::try_from(&self.buf)
                {
                    // TODO also store the food in the database
                    self.today += food * n;
                }
                self.render_main()?;
            }
            _ => {}
        }
        Ok(())
    }
}

fn main() -> io::Result<()> {
    let path = "foods";
    let foods = load_foods(path);

    let mut stdout = stdout();
    let mut tui = Tui::new(&mut stdout, foods);

    tui.execute(cursor::SavePosition)?;

    tui.render_main()?;

    enable_raw_mode()?;

    let mut right = 0; // same as the 2 in x + MAX_WIDTH + 2 in add_food
    let mut field = 0;
    loop {
        match read()? {
            Event::Key(event) if tui.state.is_add_food() => {
                tui.food_form(event, &mut right, &mut field)?
            }
            Event::Key(event) if event.code == KeyCode::Char('q') => break,
            Event::Key(event) if event.code == KeyCode::Char('a') => {
                tui.add_food()?;
            }
            Event::Resize(width, height) => {
                tui.resize(width, height);
                // TODO what to render depends on tui.state
                tui.render_main()?;
            }
            _ => {}
        }
    }

    disable_raw_mode()?;

    tui.execute(Clear(ClearType::All))?;
    tui.flush()?;
    tui.execute(cursor::RestorePosition)?
        .execute(cursor::Show)?;

    Ok(())
}
