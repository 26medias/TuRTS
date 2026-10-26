//! M1 verify, B5: `turts_client --help` lists every option and test hook, and says what
//! `--move` moves. The list of options is read from the source of `parse_cli`, so a flag that
//! someone adds without a line in the usage text turns this test red.

use std::process::{Command, Output};

/// Every `"--flag" =>` arm of `parse_cli`.
fn flags_in_parse_cli() -> Vec<String> {
    let source = include_str!("../src/options.rs");
    let flags: Vec<String> = source
        .lines()
        .map(str::trim_start)
        .filter(|line| line.starts_with("\"--") && line.contains("\" =>"))
        .map(|line| line[1..line.find("\" =>").unwrap()].to_string())
        .collect();
    assert!(flags.len() >= 15, "the scan found too few flags: {flags:?}");
    flags
}

fn client(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_turts_client"))
        .args(args)
        .output()
        .expect("turts_client starts")
}

#[test]
fn help_prints_the_usage_on_stdout_and_exits_0() {
    for args in [
        &["--help"][..],
        &["-h"][..],
        &["--server", "http://h:1", "--help"][..],
    ] {
        let output = client(args);
        assert_eq!(output.status.code(), Some(0), "{args:?}: {output:?}");
        assert!(output.stderr.is_empty(), "{args:?}: {output:?}");
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(stdout.starts_with("usage: turts_client"), "{stdout}");
        for flag in flags_in_parse_cli() {
            assert!(stdout.contains(&flag), "the usage lacks {flag}:\n{stdout}");
        }
        assert!(stdout.contains("--help"), "{stdout}");
    }
}

#[test]
fn help_says_what_move_moves_and_what_the_exit_codes_mean() {
    let output = client(&["--help"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let move_line = stdout
        .lines()
        .position(|line| line.trim_start().starts_with("--move X,Y"))
        .expect("the usage has a --move line");
    let move_text: Vec<&str> = stdout.lines().skip(move_line).take(4).collect();
    let move_text = move_text.join(" ");
    assert!(move_text.contains("all the units"), "{move_text}");
    assert!(move_text.contains("--move-unit"), "{move_text}");
    for code in ["0 ", "1 ", "2 "] {
        assert!(
            stdout
                .lines()
                .any(|line| line.trim_start().starts_with(code)),
            "the usage lacks the exit code {code}:\n{stdout}"
        );
    }
}

#[test]
fn a_command_line_error_points_to_help() {
    for args in [&[][..], &["--bogus"][..], &["--server"][..]] {
        let output = client(args);
        assert_eq!(output.status.code(), Some(2), "{args:?}: {output:?}");
        assert!(output.stdout.is_empty(), "{args:?}: {output:?}");
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.starts_with("turts_client: "), "{stderr}");
        assert!(stderr.contains("--help"), "{args:?}: {stderr}");
    }
}
