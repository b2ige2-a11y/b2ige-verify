//! P1C real process fixture. Append-only marker proves a second execution.
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let prior = std::fs::read("executions").unwrap_or_default();
    let mut next = prior.clone();
    next.push(b'x');
    std::fs::write("executions", next).unwrap();
    if args[1] == "diverge" {
        println!("execution {}", prior.len());
    } else {
        println!("{}", std::env::var("REPLAY_VALUE").unwrap());
        println!("{}", args[2]);
        println!("{}", std::env::current_dir().unwrap().display());
        println!("PATH absent: {}", std::env::var_os("PATH").is_none());
    }
    if args[1] == "nonzero" {
        std::process::exit(7);
    }
}
