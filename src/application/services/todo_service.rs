use crate::domain::entities::Todo;

pub struct TodoService<R: TodoRepository> {
  repo: R,
}

impl<R: TodoRepository> TodoService<R> {
  pub fn new(repo: R) -> Self {
    Self { repo }
  }

  pub async fn get_todos(&self) -> Vec<Todo> {
    self.repo.get_all().await
  }
}

pub trait TodoRepository {
  async fn get_all(&self) -> Vec<Todo>;
}
