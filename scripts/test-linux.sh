#!/bin/sh
set -eu
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
cargo build --release --locked
python3 tests/netns.py target/release/udp-discovery-relay
