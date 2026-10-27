//! M1 verify, B5: `turts_server --help` lists every option, every `server.ron` field, and the
//! HTTP endpoints. The names are read from the source of `config.rs`, so a flag or a field that
//! someone adds without a line in the help text turns this test red.

use std::process::{Command, Output};

const CONFIG_SOURCE: &str = include_str!("../src/config.rs");

/// Every `"--flag" =>` arm of `parse_cli`.
fn flags_in_parse_cli() -> Vec<String> {
    let flags: Vec<String> = CONFIG_SOURCE
        .lines()
        .map(str::trim_start)
        .filter(|line| line.starts_with("\"--") && line.contains("\" =>"))
        .map(|line| line[1..line.find("\" =>").unwrap()].to_string())
        .collect();
    assert_eq!(flags.len(), 5, "the scan found {flags:?}");
    flags
}

/// The field names of `struct HostConfig`.
fn fields_of_host_config() -> Vec<String> {
    let start = CONFIG_SOURCE
        .find("pub struct HostConfig {")
        .expect("the struct is in config.rs");
    let body = &CONFIG_SOURCE[start..];
    let body = &body[..body.find("\n}").expect("the struct ends")];
    let fields: Vec<String> = body
        .lines()
        .map(str::trim_start)
        .filter_map(|line| line.strip_prefix("pub "))
        .filter_map(|line| line.split_once(':'))
        .map(|(name, _)| name.to_string())
        .collect();
    assert_eq!(fields.len(), 6, "the scan found {fields:?}");
    fields
}

fn server(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_turts_server"))
        .args(args)
        .output()
        .expect("turts_server starts")
}

#[test]
fn help_lists_options_fields_and_endpoints() {
    for args in [&["--help"][..], &["-h"][..], &["some.ron", "--help"][..]] {
        let output = server(args);
        assert_eq!(output.status.code(), Some(0), "{args:?}: {output:?}");
        assert!(output.stderr.is_empty(), "{args:?}: {output:?}");
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(
            stdout.starts_with("usage: turts_server <server.ron>"),
            "{stdout}"
        );
        for flag in flags_in_parse_cli() {
            assert!(stdout.contains(&flag), "the help lacks {flag}:\n{stdout}");
        }
        for field in fields_of_host_config() {
            assert!(stdout.contains(&field), "the help lacks {field}:\n{stdout}");
        }
        for path in ["/advertisement.json", "/lockfile.json", "/packages/"] {
            assert!(stdout.contains(path), "the help lacks {path}:\n{stdout}");
        }
        assert!(stdout.contains("--help"), "{stdout}");
    }
}

#[test]
fn a_command_line_error_points_to_help() {
    for args in [
        &[][..],
        &["a.ron", "--bogus"][..],
        &["a.ron", "--stats"][..],
    ] {
        let output = server(args);
        assert_eq!(output.status.code(), Some(1), "{args:?}: {output:?}");
        assert!(output.stdout.is_empty(), "{args:?}: {output:?}");
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.starts_with("turts_server: "), "{stderr}");
        assert!(stderr.contains("--help"), "{args:?}: {stderr}");
    }
}
