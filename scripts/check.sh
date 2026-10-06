#!/bin/sh
set -eu

cargo test --offline
cargo check --offline

