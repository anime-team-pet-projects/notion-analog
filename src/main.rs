use axum::{routing::{get, post}, http::StatusCode, Json, Router};
use serde::{Deserialize, Serialize};
use std::{env, sync::Arc};
use sqlx::{FromRow, Pool, Postgres, Row};
use sqlx::postgres::{PgPoolOptions, PgRow};
use rust_todo::run;
use axum::extract::State;

#[derive(Clone)]
struct AppState {
  pool: Pool<Postgres>
}

#[tokio::main]
async fn main() {
  tracing_subscriber::fmt()
    .init();
  dotenv::dotenv().ok();

  let app_host = env::var("APP_HOST").expect("APP_HOST must be set");
  let app_port = env::var("APP_PORT").expect("APP_PORT must be set");
  let app_db_engine = env::var("APP_DB_ENGINE").expect("APP_DB_ENGINE must be set");
  let postgres_db = env::var("POSTGRES_DB").expect("POSTGRES_DB must be set");
  let postgres_user = env::var("POSTGRES_USER").expect("POSTGRES_USER must be set");
  let postgres_password = env::var("POSTGRES_PASSWORD").expect("POSTGRES_PASSWORD must be set");
  let postgres_host = env::var("POSTGRES_HOST").expect("POSTGRES_HOST must be set");

  let app_db_engine_result: String = app_db_engine.parse().expect("APP_DB_ENGINE is not a valid DB engine");
  let ip: std::net::IpAddr = app_host.parse().expect("APP_HOST is not a valid IP address");
  let port: u16 = app_port.parse().expect("APP_PORT is not a valid port number");
  let postgres_db_result: String = postgres_db.parse().expect("POSTGRES_DB is not a valid database name");
  let postgres_user_result: String = postgres_user.parse().expect("POSTGRES_USER is not a valid user name");
  let postgres_password_result: String = postgres_password.parse().expect("POSTGRES_PASSWORD is not a valid password");
  let postgres_host_result: String = postgres_host.parse().expect("POSTGRES_HOST is not a valid host name");

  let connect_url = format!(
    "{}://{}:{}@{}/{}",
    app_db_engine_result, postgres_user_result, postgres_password_result, postgres_host_result, postgres_db_result
  );

  let pool = PgPoolOptions::new()
    .max_connections(5)
    .connect(&connect_url)
    .await
    .expect("Error");

  let shared_state = Arc::new(AppState {
    pool: pool.clone(),
  });

  // build our application with a route
  let app = Router::new()
    .route("/", get(root))
    .route("/todo", post({
      let shared_state = shared_state.clone();
      move |body| create_todo(body, shared_state.clone())
    }))
    .route("/todo", get(get_todos))
    .with_state(shared_state);

  let listener = tokio::net::TcpListener::bind(format!("{}:{}", {ip}, {port})).await.unwrap();

  run(ip, port);

  let _ = sqlx::migrate!().run(&pool).await;

  axum::serve(listener, app).await.unwrap();
}

// basic handler that responds with a static string
async fn root() -> &'static str {
  "Hello, World!"
}

async fn create_todo(
  Json(payload): Json<CreateTodoDTO>,
  state: Arc<AppState>,
) -> (StatusCode, String) {
  let mut conn = state.pool.acquire().await.unwrap();
  let todo = sqlx::query_as::<_, Todo>("INSERT INTO todo (title) VALUES ($1) RETURNING id, title")
    .bind(&payload.title)
    .fetch_one(&mut *conn)
    .await
    .map_err(internal_error)
    .unwrap();
  (StatusCode::CREATED, format!("Created todo with id: {}, title: {}", todo.id, todo.title))
}

async fn get_todos(
  State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<Todo>>, (StatusCode, String)> {
  let todos = sqlx::query_as::<_, Todo>("SELECT id, title FROM todo")
    .fetch_all(&state.pool)
    .await
    .map_err(internal_error)?;
  Ok(Json(todos))
}

fn internal_error<E>(err: E) -> (StatusCode, String)
  where
    E: std::error::Error,
{
  (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

// the input to our `create_todo` handler
#[derive(Deserialize, Debug)]
struct CreateTodoDTO {
  title: String,
}

// the output to our `create_todo` handler
#[derive(Serialize)]
struct Todo {
  id: i32,
  title: String,
}

impl FromRow<'_,PgRow> for Todo {
  fn from_row(row: &PgRow) -> Result<Self, sqlx::Error> {
    Ok(Self {
      id: row.try_get("id")?,
      title: row.try_get("title")?,
    })
  }
}
