use std::fmt::Display;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub trait Logable {
    fn handle<D: Display>(&self, msg: D);
}

impl<T, E: Display> Logable for std::result::Result<T, E> {
    fn handle<D: Display>(&self, msg: D) {
        if let Err(err) = self {
            error!("Error: {}: {}", msg, err);
        }
    }
}
