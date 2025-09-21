# Rusty-Checker 🧪

Built with ❤️ in Rust

Rusty-Checker is a **memory testing and analysis tool** written in Rust that helps detect memory bit flips, test stability, and analyze reliability across configurable patterns and workloads.

## Features

-  **RowHammer-Inspired Memory Stress Tests** – hammer memory cells to expose instability
-  **Test Modes**
  - Sequential
  - Random (with PRNG)
  - Checkerboard (0xAAAA / 0x5555)
-  **Configurable Parameters**
  - Memory size (safe allocation limits enforced)
  - Number of hammering iterations
  - Verbose output toggle
  - Log file path
-  **Structured Logging**
  - Chrono-based timestamped logs
  - Detailed progress and anomaly reporting
-  **Anomaly Tracking** – detect and record mismatches with memory addresses
-  **Progress Reporting** – clear percentage updates during test runs
-  **Safe Memory Allocation** – prevents excessive allocations and handles OOM errors gracefully
-  **Error Handling** – robust exit messages with clear guidance

## 📦 Installation

Clone the repo and build with Cargo:

```bash
git clone https://github.com/<your-username>/rusty_checker.git
cd rusty_checker
cargo build --release
```

or install from crates.io:

```bash
cargo install rusty_checker
```

## Usage
```bash
rusty_checker <memory_in_mb> <hammer_count> [--random] [--verbose] [--log <file>]
```

## Example runs

- Run a sequential test on 64 MB, 10 hammer cycles:

```bash
rusty_checker 64 10
```

- Run with random access mode, verbose output, and log results:

```bash
rusty_checker 128 20 --random --verbose --log rusty_log.txt
```

## DISCLAIMER

Rusty-Checker performs low-level stress testing. While safe by design, improper use on unstable systems may cause crashes or instability. Use at your own risk.
