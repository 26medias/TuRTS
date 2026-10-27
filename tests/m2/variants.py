#!/usr/bin/env python3
"""Validated Python entry point for the test-only Rust variant generator."""
import argparse
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]

def main(argv=None):
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--seed", required=True, type=int)
    parser.add_argument("--mode", required=True, choices=("rename", "balance", "physical"))
    parser.add_argument("--payment", required=True, choices=("charge", "reserve"))
    args = parser.parse_args(argv)
    caller_dir = Path.cwd()
    source = args.source if args.source.is_absolute() else caller_dir / args.source
    output = args.out if args.out.is_absolute() else caller_dir / args.out
    command = ["cargo", "run", "--quiet", "--manifest-path", str(ROOT / "crates/turts_checks/Cargo.toml"), "--example", "m2_variants", "--", "--source", str(source.absolute()), "--out", str(output.absolute()), "--seed", str(args.seed), "--mode", args.mode, "--payment", args.payment]
    return subprocess.run(command, cwd=ROOT, check=False).returncode

if __name__ == "__main__":
    raise SystemExit(main())
