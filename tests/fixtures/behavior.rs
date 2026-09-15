//! Two compiled targets, identical explicit args/env, actual different code.
use std::fs::{self, OpenOptions};
use std::io::Write;

pub fn run(after: bool) {
    let args: Vec<_> = std::env::args().collect();
    if let Ok(marker) = std::env::var("EXECUTION_MARKER") {
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(marker)
            .unwrap()
            .write_all(if after { b"a" } else { b"b" })
            .unwrap();
    }
    match args[1].as_str() {
        "equal" => {
            std::io::stdout().write_all(b"ready\n\x00\xff").unwrap();
            std::io::stderr().write_all(b"warning\x00\xfe").unwrap();
        }
        "stdout" => {
            std::io::stdout()
                .write_all(if after {
                    b"error\x00\xff"
                } else {
                    b"ready\x00\xff"
                })
                .unwrap();
        }
        "stderr" => {
            std::io::stderr()
                .write_all(if after {
                    b"error\x00\xff"
                } else {
                    b"ready\x00\xff"
                })
                .unwrap();
        }
        "exit" => std::process::exit(i32::from(after)),
        "nonzero" => std::process::exit(7),
        "signal" => {
            use std::os::unix::process::CommandExt;
            // Replace this process and use the shell builtin: a separate /bin/kill
            // can outlive its parent and race the observer's process-group cleanup
            // on macOS. Keep the same PID and actual terminating signal, no helper.
            let error = std::process::Command::new("/bin/sh")
                .args([
                    "-c",
                    if after {
                        "kill -KILL \"$$\""
                    } else {
                        "kill -TERM \"$$\""
                    },
                ])
                .exec();
            panic!("signal fixture exec failed: {error}");
        }
        "timeout" | "one-timeout" => {
            println!("observed prefix");
            std::io::stdout().flush().unwrap();
            if !after || args[1] == "timeout" {
                std::thread::sleep(std::time::Duration::from_secs(30));
            }
        }
        "flood" => {
            std::io::stdout()
                .write_all(&vec![b'x'; 2 * 1024 * 1024])
                .unwrap();
        }
        "workspace" | "source-change" => {
            std::io::stdout()
                .write_all(&fs::read("input.txt").unwrap())
                .unwrap();
            assert!(!std::path::Path::new("created").exists());
            fs::write("input.txt", b"changed by target").unwrap();
            fs::create_dir("created").unwrap();
            std::env::set_current_dir("created").unwrap();
            fs::write("marker", b"local change").unwrap();
            if !after && args[1] == "source-change" {
                fs::write(std::env::var("SOURCE_FILE").unwrap(), b"mutated source").unwrap();
            }
        }
        "inputs" => {
            println!("{}", args[2]);
            println!("{}", std::env::var("EXPLICIT_VALUE").unwrap());
            println!("PATH absent: {}", std::env::var_os("PATH").is_none());
            let mut stdin = String::new();
            std::io::stdin().read_line(&mut stdin).unwrap();
            assert!(stdin.is_empty());
        }
        "corrupt-before" => {
            if after {
                fs::write(std::env::var("BEFORE_EVIDENCE").unwrap(), b"{}").unwrap();
            }
        }
        "change-after-target" => {
            if !after {
                fs::write(
                    std::env::var("AFTER_TARGET").unwrap(),
                    b"replaced executable",
                )
                .unwrap();
            }
        }
        _ => panic!("unknown behavior fixture"),
    }
}
