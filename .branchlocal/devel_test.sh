#!/bin/sh
# branch:master
# branch:devel
# branch:feature/basicbox
# branch:feature/*
#export CARGO_INCREMENTAL=0
cargo fmt --all -- --check &&
    cargo clippy 2>&1 | tee /dev/stderr | awk '/warning:|error:/{exit 1}' &&
    cargo test -- --nocapture
echo $?
