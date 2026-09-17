use std::io::{self, Write};
fn status(command: &str) -> Result<&'static str, &'static str> {
    match command {
        "base" => Ok("same"),
        "--help" => Ok("status-cli base"),
        _ => Err("unknown command"),
    }
}
fn main() {
    let command = std::env::args().nth(1).unwrap_or_else(|| "--help".into());
    match status(&command) {
        Ok(text) => io::stdout().write_all(text.as_bytes()).unwrap(),
        Err(text) => { eprintln!("{text}"); std::process::exit(2); }
    }
}
