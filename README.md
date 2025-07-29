# Rusty-Checker

⚡ A simple Rust-based RowHammer vulnerability checker

Rusty-Checker allocates a block of memory and simulates hammering adjacent rows to detect any bit flips or anomalies that may indicate a RowHammer vulnerability on your machine.

## 🚀 Features

- Memory hammering simulation in Rust.
- Detects possible RowHammer-style anomalies.
- CLI support for configurable memory size (in MB).

## 🛠 Usage

```bash
git clone git@github.com:KushalMeghani1644/Rusty-Checker.git
cd Rusty-Checker
cargo run --release -- <memory_size_mb>
