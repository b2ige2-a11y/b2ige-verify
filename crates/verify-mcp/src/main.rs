use std::{io, path::Path, process::ExitCode};
fn main() -> ExitCode {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args == ["--version"] {
        println!("b2ige-mcp {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }
    if args == ["--help"] {
        println!("Usage: b2ige-mcp --registry /trusted/project.json\nLocal stdio MCP; registered configs only; no arbitrary shell/file tools.");
        return ExitCode::SUCCESS;
    }
    if args.len() != 2 || args[0] != "--registry" {
        eprintln!("Expected --registry FILE");
        return ExitCode::from(64);
    }
    let result = verify_mcp::Server::open(Path::new(&args[1]))
        .and_then(|mut s| s.serve(io::stdin().lock(), io::stdout().lock()));
    if result.is_err() {
        eprintln!("MCP initialization or transport failed");
        ExitCode::from(3)
    } else {
        ExitCode::SUCCESS
    }
}
