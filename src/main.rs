use std::{path::Path, sync::Arc};

use askama::Template;
use axum::{
    extract::{Query, State},
    response::{Html, Redirect},
    routing::{get, post},
    Form, Router,
};
use log::debug;
use rusqlite::{Connection, Row};
use serde::Deserialize;
use time::OffsetDateTime;
use tokio::{net::TcpListener, sync::Mutex};
use tower_http::services::ServeDir;

struct Today {
    #[allow(unused)]
    id: Option<usize>,
    date: String,
    calories: f64,
    carbs: f64,
    fat: f64,
    protein: f64,
}

#[derive(Template)]
#[template(path = "index.html")]
struct Index {
    today: Today,
    foods: Vec<Food>,
}

async fn index(State(state): State<Arc<Mutex<App>>>) -> Html<String> {
    let now = OffsetDateTime::now_utc();
    let date =
        format!("{}-{:02}-{:02}", now.year(), now.month() as u8, now.day());
    let lock = state.lock().await;
    Index {
        today: lock.table.get_today(&date).unwrap(),
        foods: lock.table.get_foods().unwrap(),
    }
    .render()
    .unwrap()
    .into()
}

async fn add_food(
    State(state): State<Arc<Mutex<App>>>,
    Form(food): Form<Food>,
) -> Redirect {
    state.lock().await.table.add_food(food).unwrap();
    Redirect::to("/")
}

#[derive(Deserialize)]
struct DeleteFood {
    id: usize,
}

#[derive(Deserialize)]
struct AddFoodToday {
    id: usize,
    date: String,
}

async fn delete_food(
    State(state): State<Arc<Mutex<App>>>,
    Query(DeleteFood { id }): Query<DeleteFood>,
) -> Redirect {
    state.lock().await.table.delete_food(id).unwrap();
    Redirect::to("/")
}

async fn add_food_today(
    State(state): State<Arc<Mutex<App>>>,
    Query(AddFoodToday { id, date }): Query<AddFoodToday>,
) -> Redirect {
    state.lock().await.table.add_food_today(id, &date).unwrap();
    Redirect::to("/")
}

#[derive(Debug, Deserialize)]
struct Food {
    id: Option<usize>,
    name: String,
    calories: f64,
    carbs: f64,
    fat: f64,
    protein: f64,
    unit: String,
}

struct Table {
    conn: Connection,
}

impl Table {
    fn new(filename: impl AsRef<Path>) -> Self {
        let conn = Connection::open(filename).unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS foods (
                id INTEGER PRIMARY KEY,
                name TEXT UNIQUE,
                calories REAL,
                carbs REAL,
                fat REAL,
                protein REAL,
                unit TEXT
            )",
            (),
        )
        .unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS days (
                id INTEGER PRIMARY KEY,
                date TEXT UNIQUE,
                calories REAL DEFAULT 0,
                carbs REAL DEFAULT 0,
                fat REAL DEFAULT 0,
                protein REAL DEFAULT 0
            )",
            (),
        )
        .unwrap();
        Self { conn }
    }

    fn add_food(&self, food: Food) -> Result<(), rusqlite::Error> {
        debug!("adding {food:?} to database");
        self.conn.execute(
            "INSERT OR IGNORE INTO foods(
                name, calories, carbs, fat, protein, unit
            )
            VALUES (?, ?, ?, ?, ?, ?)",
            (
                food.name,
                food.calories,
                food.carbs,
                food.fat,
                food.protein,
                food.unit,
            ),
        )?;
        Ok(())
    }

    fn get_food(&self, id: usize) -> Result<Food, rusqlite::Error> {
        debug!("retrieving food {id} from database");
        let mut stmt = self.conn.prepare("SELECT * FROM foods WHERE id = ?")?;
        stmt.query_row((id,), |row| Food::try_from(row))
    }

    fn delete_food(&self, id: usize) -> Result<(), rusqlite::Error> {
        debug!("deleting food {id} from database");
        self.conn
            .execute("DELETE FROM foods WHERE id = ?", (id,))
            .map(|_| ())
    }

    fn get_foods(&self) -> Result<Vec<Food>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, calories, carbs, fat, protein, unit FROM foods",
        )?;
        let res = stmt.query_map((), |row| Food::try_from(row))?;
        res.collect()
    }

    fn insert_today(&self, date: &str) -> Result<(), rusqlite::Error> {
        debug!("inserting today: {date}");
        self.conn
            .execute("INSERT OR IGNORE INTO days (date) VALUES (?)", (date,))
            .map(|_| ())
    }

    fn get_today(&self, date: &str) -> Result<Today, rusqlite::Error> {
        debug!("retrieving date: {date} from db");
        let mut stmt = self.conn
            .prepare("SELECT id, date, calories, carbs, fat, protein FROM days WHERE date = ?")?;
        match stmt.query_row((date,), |row| Today::try_from(row)) {
            Ok(it) => Ok(it),
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                self.insert_today(date)?;
                self.get_today(date)
            }
            Err(e) => Err(e),
        }
    }

    fn add_food_today(
        &self,
        id: usize,
        date: &str,
    ) -> Result<(), rusqlite::Error> {
        let food = self.get_food(id)?;
        debug!("adding {food:?} to entry for {date}");
        let cur = self.get_today(date)?;
        let mut stmt = self.conn.prepare(
            "UPDATE days SET (calories, carbs, fat, protein) = (?, ?, ?, ?) where date = ?",
        )?;
        stmt.execute((
            cur.calories + food.calories,
            cur.carbs + food.carbs,
            cur.fat + food.fat,
            cur.protein + food.protein,
            date,
        ))
        .map(|_| ())
    }
}

impl TryFrom<&Row<'_>> for Food {
    type Error = rusqlite::Error;

    fn try_from(row: &Row<'_>) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.get(0)?,
            name: row.get(1)?,
            calories: row.get(2)?,
            carbs: row.get(3)?,
            fat: row.get(4)?,
            protein: row.get(5)?,
            unit: row.get(6)?,
        })
    }
}

impl TryFrom<&Row<'_>> for Today {
    type Error = rusqlite::Error;

    fn try_from(row: &Row<'_>) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.get(0)?,
            date: row.get(1)?,
            calories: row.get(2)?,
            carbs: row.get(3)?,
            fat: row.get(4)?,
            protein: row.get(5)?,
        })
    }
}

struct App {
    table: Table,
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let app = Router::new()
        .route("/", get(index))
        .route("/add-food", post(add_food))
        .route("/delete-food", post(delete_food))
        .route("/add-food-today", post(add_food_today))
        .nest_service("/js", ServeDir::new("js"))
        .with_state(Arc::new(Mutex::new(App {
            table: Table::new("macroni.sqlite"),
        })));
    let addr = "0.0.0.0:3333";
    eprintln!("serving on {addr}");
    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
