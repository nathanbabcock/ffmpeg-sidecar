# Backtrace is on by default, and does not respect Cargo.toml
# [env] section, even with force = true.
RUST_BACKTRACE=0 cargo test
