use crate::domain::entities::todo::Todo;
use crate::domain::repositories::todo_repository::TodoRepository;
use sqlx::PgPool;

pub struct PostgresTodoRepository {
  pub pool: PgPool,
}

impl TodoRepository for PostgresTodoRepository {
  async fn get_all(&self) -> Vec<Todo> {
    let todos = sqlx::query_as::<_, Todo>(
      "SELECT id, title, status, description, user_id, created_at, updated_at FROM todo",
    )
      .fetch_all(&self.pool)
      .await
      .expect("Failed to fetch todos");

    todos
  }

  // Реализуйте другие методы здесь
}
