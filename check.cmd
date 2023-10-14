cargo check --workspace 
cargo check --workspace --all-features --lib 
cargo fmt --all -- --check
cargo clippy --workspace --all-features --  -D warnings -W clippy::all
cargo test --workspace --all-features
cargo test --workspace --doc
trunk build
