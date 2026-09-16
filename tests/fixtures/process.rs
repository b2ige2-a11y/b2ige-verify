//! Dedicated integration executable: no shell or mock process.
use std::io::Write;
fn main() {
    let mode = std::env::args().nth(1).expect("fixture mode");
    match mode.as_str() {
        "success" => {
            std::io::stdout().write_all(b"hello\n\x00\xff").unwrap();
        }
        "failure" => {
            eprintln!("target failure");
            std::process::exit(7);
        }
        "timeout" => {
            println!("before timeout");
            std::io::stdout().flush().unwrap();
            std::thread::sleep(std::time::Duration::from_secs(30));
        }
        "environment" => {
            println!("{}", std::env::var("P1B_VALUE").unwrap());
            println!("{}", std::env::current_dir().unwrap().display());
            println!("{}", std::env::var_os("PATH").is_none());
            println!("{}", std::env::args().nth(2).unwrap());
        }
        "flood" => {
            std::io::stdout()
                .write_all(&vec![b'x'; 2 * 1024 * 1024])
                .unwrap();
        }
        "both" => {
            for _ in 0..32 {
                std::io::stdout().write_all(&[b'a'; 8192]).unwrap();
                std::io::stderr().write_all(&[b'b'; 8192]).unwrap();
            }
        }
        "descendant" => {
            let mut child = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["delayed-marker", &std::env::args().nth(2).unwrap()])
                .spawn()
                .unwrap();
            child.wait().unwrap();
        }
        "delayed-marker" => {
            std::thread::sleep(std::time::Duration::from_millis(700));
            std::fs::write(std::env::args().nth(2).unwrap(), b"leaked child").unwrap();
        }
        "signal" => {
            #[cfg(unix)]
            {
                std::process::Command::new("/bin/kill")
                    .args(["-TERM", &std::process::id().to_string()])
                    .status()
                    .unwrap();
            }
            #[cfg(not(unix))]
            {
                std::process::exit(3);
            }
        }
        _ => panic!("unknown fixture mode"),
    }
}
