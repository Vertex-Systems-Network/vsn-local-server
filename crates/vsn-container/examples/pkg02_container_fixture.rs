use std::{env, fs, process::ExitCode, thread, time::Duration};

fn main() -> ExitCode {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        eprintln!("fake docker requires arguments");
        return ExitCode::from(2);
    }

    let mode = env::var_os("VSN_PKG02_0215_MODE_FILE")
        .and_then(|path| fs::read_to_string(path).ok())
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "reachable".into());

    if args.as_slice() == ["--version"] {
        println!("Docker version 99.0.0-vsn-fixture, build pkg02-0215");
        return ExitCode::SUCCESS;
    }

    if args.first().map(String::as_str) == Some("info") {
        return match mode.as_str() {
            "reachable" | "flood" => {
                println!("99.0.0-vsn-fixture");
                ExitCode::SUCCESS
            }
            "hang" => {
                thread::sleep(Duration::from_secs(30));
                println!("unexpected late daemon response");
                ExitCode::SUCCESS
            }
            _ => {
                eprintln!("fixture daemon unavailable");
                ExitCode::FAILURE
            }
        };
    }

    if mode == "hang" {
        thread::sleep(Duration::from_secs(30));
        return ExitCode::SUCCESS;
    }
    if mode == "daemon-down" {
        eprintln!("fixture daemon unavailable");
        return ExitCode::FAILURE;
    }

    if args.first().map(String::as_str) == Some("ps") {
        println!("abc123\tvsn-fixture\tvsn/fake:1\tUp 10 seconds\t127.0.0.1:18080->8080/tcp");
        return ExitCode::SUCCESS;
    }
    if args.starts_with(&["image".into(), "ls".into()]) {
        println!("img123\tvsn/fake:1\t12.3MB");
        return ExitCode::SUCCESS;
    }
    if args.starts_with(&["volume".into(), "ls".into()]) {
        println!("vol123\tvol123\tlocal");
        return ExitCode::SUCCESS;
    }
    if args.starts_with(&["network".into(), "ls".into()]) {
        println!("net123\tvsn-net\tbridge");
        return ExitCode::SUCCESS;
    }
    if args.first().map(String::as_str) == Some("logs") {
        if mode == "flood" {
            print!("{}", "x".repeat(3 * 1024 * 1024));
        } else {
            println!("fixture stdout log");
            eprintln!("fixture stderr log");
        }
        return ExitCode::SUCCESS;
    }
    if args.first().map(String::as_str) == Some("inspect") {
        println!(r#"[{"Id":"abc123","Name":"/vsn-fixture"}]"#);
        return ExitCode::SUCCESS;
    }
    if args.first().map(String::as_str) == Some("stats") {
        println!("vsn-fixture\t0.10%\t12MiB / 1GiB\t1kB / 2kB\t3kB / 4kB\t2");
        return ExitCode::SUCCESS;
    }
    if matches!(
        args.first().map(String::as_str),
        Some("start" | "stop" | "restart" | "pause" | "unpause")
    ) {
        println!("{}", args.get(1).map(String::as_str).unwrap_or("unknown"));
        return ExitCode::SUCCESS;
    }

    eprintln!("unsupported fake docker command: {}", args.join(" "));
    ExitCode::from(2)
}
