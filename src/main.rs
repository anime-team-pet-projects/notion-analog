use axum::{routing::{get, post, delete, patch}, http::StatusCode, Json, Router};
use serde::{Deserialize, Serialize};
use std::{env, sync::Arc};
use std::net::IpAddr;
use sqlx::{Pool, Postgres};
use sqlx::postgres::{PgPoolOptions};
use axum::extract::{Path, State};
use chrono::{NaiveDateTime};

#[derive(Clone)]
struct AppState {
  pool: Pool<Postgres>
}

pub struct AppConfig {
  pub app_host: IpAddr,
  pub app_port: u16,
  pub app_db_engine: String,
  pub postgres_db: String,
  pub postgres_user: String,
  pub postgres_password: String,
  pub postgres_host: String,
}

impl AppConfig {
  pub fn new() -> Self {
    let app_host = env::var("APP_HOST").expect("APP_HOST must be set").parse().expect("APP_HOST is not a valid IP address");
    let app_port = env::var("APP_PORT").expect("APP_PORT must be set").parse().expect("APP_PORT is not a valid port number");
    let app_db_engine = env::var("APP_DB_ENGINE").expect("APP_DB_ENGINE must be set");
    let postgres_db = env::var("POSTGRES_DB").expect("POSTGRES_DB must be set");
    let postgres_user = env::var("POSTGRES_USER").expect("POSTGRES_USER must be set");
    let postgres_password = env::var("POSTGRES_PASSWORD").expect("POSTGRES_PASSWORD must be set");
    let postgres_host = env::var("POSTGRES_HOST").expect("POSTGRES_HOST must be set");

    AppConfig {
      app_host,
      app_port,
      app_db_engine,
      postgres_db,
      postgres_user,
      postgres_password,
      postgres_host,
    }
  }

  // Функция для создания строки подключения к базе данных
  pub fn connect_url(&self) -> String {
    format!(
      "{}://{}:{}@{}/{}",
      self.app_db_engine,
      self.postgres_user,
      self.postgres_password,
      self.postgres_host,
      self.postgres_db
    )
  }
}

#[tokio::main]
async fn main() {
  tracing_subscriber::fmt()
    .init();
  dotenv::dotenv().ok();

  let config = AppConfig::new();
  let connect_url = config.connect_url();

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
    .route("/api/v1/todo", post({
      let shared_state = shared_state.clone();
      move |body| create_todo(body, shared_state.clone())
    }))
    .route("/api/v1/todo", get(get_todos))
    .route("/api/v1/todo/:id", get({
      let shared_state = shared_state.clone();
      move |id| get_todo(id, shared_state.clone())
    }))
    .route("/api/v1/todo/:id", patch({
      let shared_state = shared_state.clone();
      move |id, body| update_todo(id, body, shared_state.clone())
    }))
    .route("/api/v1/todo/:id", delete({
      let shared_state = shared_state.clone();
      move |id| delete_todo(id, shared_state.clone())
    }))
    .with_state(shared_state);

  let listener = tokio::net::TcpListener::bind(format!("{}:{}", {config.app_host}, {config.app_port})).await.unwrap();

  let _ = sqlx::migrate!().run(&pool).await;

  axum::serve(listener, app).await.unwrap();
}

async fn root() -> &'static str {
  "Hello, World!"
}

async fn create_todo(
  Json(payload): Json<CreateTodoDTO>,
  state: Arc<AppState>,
) -> StatusCode {
  let status = payload.status.unwrap_or("todo".to_string());
  let description = payload.description.unwrap_or("".to_string());

  let _ = sqlx::query("INSERT INTO todo (title, status, description, user_id) VALUES ($1, $2, $3, $4)")
    .bind(&payload.title)
    .bind(status)
    .bind(description)
    .bind(&payload.user_id)
    .execute(&state.pool)
    .await
    .map_err(internal_error)
    .unwrap();

  StatusCode::CREATED
}

async fn get_todos(
  State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<Todo>>, (StatusCode, String)> {
  let todos = sqlx::query_as::<_, Todo>("SELECT * FROM todo")
    .fetch_all(&state.pool)
    .await
    .map_err(internal_error)?;

  Ok(Json(todos))
}

async fn get_todo(
  Path(id): Path<i32>,
  state: Arc<AppState>,
) -> Result<Json<Todo>, (StatusCode, String)> {
  let result = sqlx::query_as::<_, Todo>("SELECT id, title, status, description, user_id, created_at, updated_at FROM todo WHERE id = $1")
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

  match result {
    Some(todo) => {
      println!("{:?}", todo);
      Ok(Json(todo))
    },
    None => Err((StatusCode::NOT_FOUND, "Задача не найдена".to_string())),
  }
}

async fn update_todo(
  Path(id): Path<i32>,
  Json(payload): Json<UpdateTodoDTO>,
  state: Arc<AppState>,
) -> Result<StatusCode, (StatusCode, String)> {
  let mut query = String::from("UPDATE todo SET ");
  let mut bind_values = Vec::new();
  let mut bind_index = 1;

  if let Some(status) = payload.status {
    query.push_str("status = $");
    query.push_str(&bind_index.to_string());
    bind_values.push(status);
    bind_index += 1;
  }

  if let Some(title) = payload.title {
    if !bind_values.is_empty() {
      query.push_str(", ");
    }
    query.push_str("title = $");
    query.push_str(&bind_index.to_string());
    bind_values.push(title);
    bind_index += 1;
  }

  if let Some(description) = payload.description {
    if !bind_values.is_empty() {
      query.push_str(", ");
    }
    query.push_str("description = $");
    query.push_str(&bind_index.to_string());
    bind_values.push(description);
    bind_index += 1;
  }

  if !bind_values.is_empty() {
    query.push_str(" WHERE id = $");
    query.push_str(&bind_index.to_string());
  }

  let mut query_builder = sqlx::query(&query);
  for value in bind_values {
    query_builder = query_builder.bind(value);
  }

  query_builder = query_builder.bind(id);

  let result = query_builder
    .execute(&state.pool)
    .await
    .map_err(internal_error)?;

  if result.rows_affected() > 0 {
    Ok(StatusCode::OK)
  } else {
    Err((StatusCode::NOT_FOUND, "Задача не найдена".to_string()))
  }
}

async fn delete_todo(
  Path(id): Path<i32>,
  state: Arc<AppState>,
) -> StatusCode {
  let _ = sqlx::query("DELETE FROM todo WHERE id = $1")
    .bind(id)
    .execute(&state.pool)
    .await
    .map_err(internal_error)
    .unwrap();

  StatusCode::OK
}

fn internal_error<E>(err: E) -> (StatusCode, String)
  where
    E: std::error::Error,
{
  (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

#[derive(Deserialize, Debug)]
struct CreateTodoDTO {
  title: String,
  status: Option<String>,
  description: Option<String>,
  user_id: String,
}

#[derive(Deserialize, Debug)]
struct UpdateTodoDTO {
  title: Option<String>,
  status: Option<String>,
  description: Option<String>,
}

#[derive(Serialize, Debug, sqlx::FromRow)]
pub struct Todo {
  id: i32,
  title: String,
  status: String,
  description: String,
  user_id: String,
  created_at: NaiveDateTime,
  updated_at: NaiveDateTime,
}
