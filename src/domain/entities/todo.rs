use chrono::NaiveDateTime;
use serde_derive::Serialize;

#[derive(Serialize, Debug, sqlx::FromRow)]
pub struct Todo {
  pub id: i32,
  pub title: String,
  pub status: String,
  pub description: String,
  pub user_id: String,
  pub created_at: NaiveDateTime,
  pub updated_at: NaiveDateTime,
}
