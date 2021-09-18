pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub trait Logable {
    fn handle(&self, msg: &str);
}

impl Logable for Result<()> {
    fn handle(&self, msg: &str) {
        if let Err(err) = self {
            error!("Error: {}: {}", msg, err);
        }
    }
}
