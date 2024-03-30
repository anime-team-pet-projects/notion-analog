use axum::{Router};
use std::sync::Arc;
use axum::routing::get;
use crate::application::services::TodoService;
use crate::domain::repositories::TodoRepository;

pub fn routes<R: TodoRepository + Send + Sync + 'static>(todo_service: Arc<TodoService<R>>) -> Router<()> {
  Router::new()
    .route("/todos", get(move || async move {
      let todos = todo_service.get_todos().await;
      // Assuming `todos` can be converted into a response.
      // You might need to adjust this part based on your actual response type.
      todos.into_response()
    }))
  // Add other routes here
}
