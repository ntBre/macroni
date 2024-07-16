use std::{path::Path, sync::Arc};

use askama::Template;
use axum::{
    extract::{Query, State},
    response::{Html, Redirect},
    routing::{get, post},
    Form, Router,
};
use rusqlite::{Connection, Row};
use serde::Deserialize;
use tokio::{net::TcpListener, sync::Mutex};
use tower_http::services::ServeDir;

#[derive(Template)]
#[template(path = "index.html")]
struct Index {
    foods: Vec<Food>,
}

async fn index(State(state): State<Arc<Mutex<App>>>) -> Html<String> {
    Index { foods: state.lock().await.table.get_foods().unwrap() }
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

async fn delete_food(
    State(state): State<Arc<Mutex<App>>>,
    Query(DeleteFood { id }): Query<DeleteFood>,
) -> Redirect {
    state.lock().await.table.delete_food(id).unwrap();
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

    fn delete_food(&self, id: usize) -> Result<(), rusqlite::Error> {
        eprintln!("deleting food {id} from database");
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

struct App {
    table: Table,
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(index))
        .route("/add-food", post(add_food))
        .route("/delete-food", post(delete_food))
        .nest_service("/js", ServeDir::new("js"))
        .with_state(Arc::new(Mutex::new(App {
            table: Table::new("macroni.sqlite"),
        })));
    let addr = "0.0.0.0:3333";
    eprintln!("serving on {addr}");
    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
