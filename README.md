# Rusty-Checker 🧪

Built with ❤️ in Rust

Rusty-Checker is a **comprehensive memory testing tool** written in Rust that helps detect memory bit flips, test memory stability, and analyze system memory reliability across different access patterns and workloads.

## ✨ Features

- 🔨 **Row Hammer Test** – stress adjacent memory rows for vulnerability detection
- 📊 **Sequential, Random, and Checkerboard Patterns**
- 🧩 **Configurable Memory Allocation & Stress Testing**
- 🔢 **Multiple Test Patterns**:
  - `0xAAAA`
  - `0x5555`
  - `0xFFFF`
  - `0x0000`
- 📝 **Detailed Logging** with timestamps and progress reporting
- 🚀 **Performance Metrics** including memory bandwidth calculation
- ⚙️ **Customizable Parameters**: step sizes, verbose output options, log file paths
- 🔍 **Anomaly Detection** with error location tracking
- 🛡️ **Graceful Error Handling** and memory allocation safety

## 📦 Installation

Clone the repo and build with Cargo:

```bash
cargo install rusty_checker
./rusty_checker -- <memory> <hammer_count>
