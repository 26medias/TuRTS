//! Task 8a-1: client options from the command line and from the browser query.

use std::path::PathBuf;

use turts_client::options::{ClientOptions, options_from_query, parse_cli, random_player_id};

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|arg| arg.to_string()).collect()
}

#[test]
fn options_from_query_reads_server_and_ws() {
    let options = options_from_query("?server=http://h:7000&ws=ws://h:7001").unwrap();
    assert_eq!(options.server_url, "http://h:7000");
    assert_eq!(options.ws_url, "ws://h:7001");
    assert!(!options.headless);

    // The same values, percent encoded, with an unrelated key and no leading `?`.
    let options =
        options_from_query("x=1&server=http%3A%2F%2Fh%3A7000&ws=ws%3A%2F%2Fh%3A7001").unwrap();
    assert_eq!(options.server_url, "http://h:7000");
    assert_eq!(options.ws_url, "ws://h:7001");

    let error = options_from_query("").unwrap_err();
    assert!(error.contains("server"), "{error}");
    let error = options_from_query("?server=http://h:7000").unwrap_err();
    assert!(error.contains("ws"), "{error}");
    let error = options_from_query("?server=&ws=ws://h:7001").unwrap_err();
    assert!(error.contains("server"), "{error}");
}

#[test]
fn parse_cli_reads_hooks() {
    let options = parse_cli(&args(&[
        "--server",
        "http://h:7000",
        "--ws",
        "ws://h:7001",
        "--headless",
        "--player-id",
        "42",
        "--frame-log",
        "frames.jsonl",
        "--out",
        "out.json",
        "--move",
        "20.5,30",
        "--move-after-ticks",
        "9",
        "--move-unit",
        "1",
        "--hello-hash",
        "x",
        "--run-secs",
        "2.5",
        "--drop-delta-ticks",
        "10,90",
        "--drop-arrival-delta",
        "--hold-resync-snapshots",
        "3",
    ]))
    .unwrap();
    assert_eq!(options.server_url, "http://h:7000");
    assert_eq!(options.ws_url, "ws://h:7001");
    assert!(options.headless);
    assert_eq!(options.player_id, Some(42));
    assert_eq!(options.frame_log, Some(PathBuf::from("frames.jsonl")));
    assert_eq!(options.out, Some(PathBuf::from("out.json")));
    assert_eq!(options.auto_move, Some((20.5, 30.0)));
    assert_eq!(options.auto_move_after_ticks, 9);
    assert_eq!(options.move_unit, Some(1));
    assert_eq!(options.hello_hash, Some("x".to_string()));
    assert_eq!(options.run_secs, Some(2.5));
    assert_eq!(options.drop_delta_offsets, vec![10, 90]);
    assert!(options.drop_arrival_delta);
    assert_eq!(options.hold_resync_snapshots, 3);
}

#[test]
fn parse_cli_defaults_and_errors() {
    let options = parse_cli(&args(&["--server", "http://h:7000", "--ws", "ws://h:7001"])).unwrap();
    let default = ClientOptions::default();
    assert_eq!(default.auto_move_after_ticks, 5);
    assert_eq!(options.auto_move_after_ticks, 5);
    assert!(!options.headless);
    assert_eq!(options.player_id, None);
    assert_eq!(options.auto_move, None);
    assert!(options.drop_delta_offsets.is_empty());
    assert!(!options.drop_arrival_delta);
    assert_eq!(options.hold_resync_snapshots, 0);

    let error = parse_cli(&args(&["--ws", "ws://h:7001"])).unwrap_err();
    assert!(error.contains("--server"), "{error}");
    let error = parse_cli(&args(&["--server", "http://h:7000"])).unwrap_err();
    assert!(error.contains("--ws"), "{error}");

    let base = ["--server", "http://h:7000", "--ws", "ws://h:7001"];
    let with = |extra: &[&str]| args(&[&base[..], extra].concat());
    let error = parse_cli(&with(&["--bogus"])).unwrap_err();
    assert!(error.contains("--bogus"), "{error}");
    let error = parse_cli(&with(&["--player-id"])).unwrap_err();
    assert!(error.contains("--player-id"), "{error}");
    let error = parse_cli(&with(&["--player-id", "abc"])).unwrap_err();
    assert!(error.contains("--player-id"), "{error}");
    let error = parse_cli(&with(&["--move", "1"])).unwrap_err();
    assert!(error.contains("--move"), "{error}");
    let error = parse_cli(&with(&["--drop-delta-ticks", "1,x"])).unwrap_err();
    assert!(error.contains("--drop-delta-ticks"), "{error}");
}

#[test]
fn random_player_ids_differ() {
    assert_ne!(random_player_id(), random_player_id());
}
