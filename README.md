# Rusty-Checker

<<<<<<< HEAD
Rusty-Checker is a simple open-source RowHammer anomaly simulator and RAM stress test tool built with ❤️ in Rust.

## Features
- Allocates and hammers memory to detect unusual latency or potential vulnerability indicators.
- Extremely lightweight and safe (does not attempt to exploit or flip actual DRAM bits).
- Fast hammering simulation with timing.

## Usage
=======
⚡ A simple Rust-based RowHammer vulnerability checker

Rusty-Checker allocates a block of memory and simulates hammering adjacent rows to detect any bit flips or anomalies that may indicate a RowHammer vulnerability on your machine.

> ⚠️ **WARNING:** This tool performs memory access patterns that may cause unintended side effects on vulnerable hardware.  
> It is intended for **educational and diagnostic purposes only**.  
> **Use at your own risk.**  
> I am **not responsible** for any data corruption, hardware instability, or other damage caused by the use of this code.

## 🚀 Features

- Memory hammering simulation in Rust.
- Detects possible RowHammer-style anomalies.
- CLI support for configurable memory size (in MB).

## 🛠 Usage
>>>>>>> tweaks

```bash
git clone git@github.com:KushalMeghani1644/Rusty-Checker.git
cd Rusty-Checker
<<<<<<< HEAD
cargo run --release
```
# BUILT WITH ❤️ IN RUST
=======
cargo run --release -- <memory_size_mb>
>>>>>>> tweaks
