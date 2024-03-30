use crate::domain::entities::todo::Todo;

pub trait TodoRepository {
  async fn get_all(&self) -> Vec<Todo>;
  // Добавьте другие методы здесь
}
