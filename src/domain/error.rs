use std::{error::Error};

#[derive(Debug)]
pub struct ApiError {
	pub code: u16,
	pub message: String,
	pub error: Option<Box<dyn Error>>,
}
