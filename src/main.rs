use std::{path::Path, sync::Arc};

use askama::Template;
use axum::{
    extract::State,
    response::{Html, Redirect},
    routing::{get, post},
    Form, Router,
};
use rusqlite::Connection;
use serde::Deserialize;
use tokio::{net::TcpListener, sync::Mutex};

#[derive(Template)]
#[template(path = "index.html")]
struct Index {}

async fn index() -> Html<String> {
    Index {}.render().unwrap().into()
}

async fn add_food(
    State(state): State<Arc<Mutex<App>>>,
    Form(food): Form<Food>,
) -> Redirect {
    state.lock().await.table.add_food(food).unwrap();
    Redirect::to("/")
}

#[allow(unused)]
#[derive(Debug, Deserialize)]
struct Food {
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
        Self { conn }
    }

    fn add_food(&self, food: Food) -> Result<(), rusqlite::Error> {
        eprintln!("adding {food:?} to database");
        self.conn.execute(
            "INSERT OR IGNORE INTO foods(
                name, calories, carbs, fat, protein, unit
            )
            VALUES (?, ?, ?, ?, ?, ?) ",
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
}

struct App {
    table: Table,
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(index))
        .route("/add-food", post(add_food))
        .with_state(Arc::new(Mutex::new(App {
            table: Table::new("macroni.sqlite"),
        })));
    let addr = "0.0.0.0:3333";
    eprintln!("serving on {addr}");
    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
