//! Tests of the command line and `server.ron` parse. `config.rs` is included by path because
//! `turts_server` is a binary crate.

#[allow(dead_code)]
#[path = "../src/config.rs"]
mod config;

use std::path::PathBuf;

use config::{HostConfig, parse_cli};

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|arg| arg.to_string()).collect()
}

#[test]
fn parse_cli_reads_every_option() {
    let cli = parse_cli(&args(&[
        "s.ron",
        "--tick-log",
        "t.log",
        "--stats",
        "s.json",
        "--synthetic-units",
        "50",
        "--run-ticks",
        "100",
    ]))
    .unwrap();
    assert_eq!(cli.config, PathBuf::from("s.ron"));
    assert_eq!(cli.tick_log, Some(PathBuf::from("t.log")));
    assert_eq!(cli.stats, Some(PathBuf::from("s.json")));
    assert_eq!(cli.synthetic_units, 50);
    assert_eq!(cli.run_ticks, Some(100));
}

#[test]
fn parse_cli_refuses_bad_input() {
    let error = |list: &[&str]| parse_cli(&args(list)).unwrap_err();
    assert!(error(&[]).contains("no server.ron"));
    assert!(error(&["a", "b"]).contains("more than one"));
    assert!(error(&["a", "--nope"]).contains("--nope"));
    assert!(error(&["a", "--stats"]).contains("needs a value"));
    assert!(error(&["a", "--run-ticks", "x"]).contains("whole number"));
}

#[test]
fn from_ron_reads_the_fields_and_refuses_bad_ones() {
    let text = r#"(game_package: "p.tar", http_listen: "127.0.0.1:8000",
        ws_listen: "127.0.0.1:8001", public_url: "http://h:8000", tick_hz: 20)"#;
    let config = HostConfig::from_ron(text).unwrap();
    assert_eq!(config.tick_hz, 20);
    assert_eq!(config.static_dir, None);
    for written in [r#""dist""#, r#"Some("dist")"#] {
        let with_dir = text.replace(
            "tick_hz: 20",
            &format!("tick_hz: 20, static_dir: {written}"),
        );
        let config = HostConfig::from_ron(&with_dir).unwrap();
        assert_eq!(config.static_dir.as_deref(), Some("dist"), "{written}");
    }
    let zero = text.replace("tick_hz: 20", "tick_hz: 0");
    assert!(HostConfig::from_ron(&zero).unwrap_err().contains("tick_hz"));
    let bad = text.replace("127.0.0.1:8000", "nowhere");
    assert!(
        HostConfig::from_ron(&bad)
            .unwrap_err()
            .contains("http_listen")
    );
}
