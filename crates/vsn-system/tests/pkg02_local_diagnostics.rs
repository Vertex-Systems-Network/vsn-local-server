use std::{
    fs,
    io::Write,
    net::TcpListener,
    path::PathBuf,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

fn temp_dir(label: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "vsn-pkg02-0214-{label}-{}-{stamp}",
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("create diagnostics test directory");
    path
}

#[test]
fn tcp_health_reports_success_failure_and_invalid_inputs_within_bounds() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback listener");
    let port = listener.local_addr().expect("listener address").port();

    let healthy = vsn_system::tcp_health("127.0.0.1", port, 500);
    assert!(healthy.healthy, "active loopback listener must be healthy");
    assert_eq!(healthy.kind, "tcp");

    drop(listener);
    let unhealthy = vsn_system::tcp_health("127.0.0.1", port, 500);
    assert!(
        !unhealthy.healthy,
        "closed loopback listener must be unhealthy"
    );

    let invalid_host = vsn_system::tcp_health("", port, 500);
    assert!(!invalid_host.healthy);
    assert!(invalid_host.detail.contains("invalid TCP endpoint"));

    let invalid_port = vsn_system::tcp_health("127.0.0.1", 0, 500);
    assert!(!invalid_port.healthy);
    assert!(invalid_port.detail.contains("invalid TCP endpoint"));

    let started = Instant::now();
    let bounded = vsn_system::tcp_health("203.0.113.1", 9, 100);
    assert!(!bounded.healthy);
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "100ms health request exceeded its bounded execution envelope"
    );
}

#[test]
fn log_tail_enforces_line_window_response_budget_and_errors() {
    let root = temp_dir("logs");
    let log = root.join("many-lines.log");
    {
        let mut file = fs::File::create(&log).expect("create many-lines log");
        for index in 1..=6000 {
            writeln!(file, "tail-{index:04}").expect("write log line");
        }
    }

    let lines = vsn_system::tail_log(&log, 6000).expect("tail many-lines log");
    assert_eq!(lines.len(), 5000);
    assert_eq!(lines.first().map(String::as_str), Some("tail-1001"));
    assert_eq!(lines.last().map(String::as_str), Some("tail-6000"));

    let huge = root.join("huge-line.log");
    fs::write(&huge, format!("{}\n", "x".repeat(700 * 1024))).expect("write huge line log");
    let huge_tail = vsn_system::tail_log(&huge, 10).expect("tail huge-line log");
    assert_eq!(huge_tail.len(), 1);
    assert!(huge_tail[0].starts_with("[truncated] "));
    assert!(
        serde_json::to_vec(&huge_tail)
            .expect("serialize huge tail")
            .len()
            < 600 * 1024,
        "bounded log response should stay well below the 1 MiB IPC frame"
    );

    assert!(vsn_system::tail_log(&root.join("missing.log"), 10).is_err());
    assert!(vsn_system::tail_log(&root, 10).is_err());
    fs::remove_dir_all(root).expect("remove diagnostics test directory");
}

#[test]
fn invalid_port_check_is_rejected() {
    assert!(vsn_system::port_conflicts(0).is_err());
}

#[cfg(windows)]
#[test]
fn windows_process_and_port_snapshots_are_structured_deterministic_and_bounded() {
    let processes = vsn_system::list_processes().expect("list Windows processes");
    assert!(!processes.is_empty());
    assert!(processes.len() <= 512);
    assert!(processes.windows(2).all(|pair| {
        pair[0].pid < pair[1].pid || (pair[0].pid == pair[1].pid && pair[0].name <= pair[1].name)
    }));
    assert!(processes.iter().all(|process| {
        process.name.len() <= 256
            && process
                .command
                .as_ref()
                .is_none_or(|command| command.len() <= 512)
    }));

    let metrics = vsn_system::process_metrics(std::process::id()).expect("current process metrics");
    assert_eq!(metrics.pid, std::process::id());
    assert!(metrics.memory_bytes.unwrap_or_default() > 0);

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind port-list test listener");
    let port = listener.local_addr().expect("listener address").port();
    let ports = vsn_system::list_ports().expect("list Windows ports");
    assert!(ports.len() <= 2048);
    assert!(ports.windows(2).all(|pair| {
        (pair[0].port, &pair[0].local_address, pair[0].pid)
            <= (pair[1].port, &pair[1].local_address, pair[1].pid)
    }));
    assert!(ports.iter().any(|entry| entry.port == port));
    let conflicts = vsn_system::port_conflicts(port).expect("check live port conflict");
    assert!(conflicts.iter().any(|entry| entry.port == port));
    drop(listener);
}
