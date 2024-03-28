use axum::{routing::{get, post, delete, put}, http::StatusCode, Json, Router};
use serde::{Deserialize, Serialize};
use std::{env, sync::Arc};
use std::convert::Infallible;
use sqlx::{Pool, Postgres};
use sqlx::postgres::{PgPoolOptions};
use rust_todo::run;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use chrono::{NaiveDateTime};

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
    .route("/api/v1/todo", post({
      let shared_state = shared_state.clone();
      move |body| create_todo(body, shared_state.clone())
    }))
    .route("/api/v1/todo", get(get_todos))
    .route("/api/v1/todo/:id", get({
      let shared_state = shared_state.clone();
      move |id| get_todo(id, shared_state.clone())
    }))
    .route("/api/v1/todo/:id", put({
      let shared_state = shared_state.clone();
      move |body| update_todo(body, shared_state.clone())
    }))
    .route("/api/v1/todo/:id", delete({
      let shared_state = shared_state.clone();
      move |id| delete_todo(id, shared_state.clone())
    }))
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
    .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))
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
  Json(payload): Json<UpdateTodoDTO>,
  state: Arc<AppState>,
) -> (StatusCode, String) {
  let todo =  sqlx::query_as::<_, Todo>("UPDATE todo SET title = $1, status = $2, description = $3 WHERE id = $4 RETURNING id, title")
    .bind(&payload.title)
    .bind(&payload.status)
    .bind(&payload.description)
    .bind(&payload.id)
    .fetch_one(&state.pool)
    .await
    .map_err(internal_error)
    .unwrap();

  (StatusCode::OK, format!("Created todo with id: {}, title: {}", todo.id, todo.title))
}

async fn delete_todo(
  Path(id): Path<i32>,
  state: Arc<AppState>,
) -> Result<impl IntoResponse, Infallible> {
  let todo = sqlx::query_as::<_, Todo>("DELETE FROM todo WHERE id = $1 RETURNING id, title")
    .bind(id)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Error fetching todo".to_string()))
    .unwrap();

  Ok(Json(todo))
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
  id: i32,
  title: String,
  status: String,
  description: String,
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
