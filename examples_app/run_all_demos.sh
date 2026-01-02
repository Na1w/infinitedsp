#!/bin/bash
cargo build --release

for file in src/bin/*.rs; do
    demo=$(basename "$file" .rs)
    cargo run --bin "$demo"
done
