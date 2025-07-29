# Rusty-Checker

Rusty-Checker is a simple open-source RowHammer anomaly simulator and RAM stress test tool built with ❤️ in Rust.

> ⚠️ **WARNING:** This tool performs memory access patterns that may cause unintended side effects on vulnerable hardware.  
> It is intended for **educational and diagnostic purposes only**.  
> **Use at your own risk.**  
> I am **not responsible** for any data corruption, hardware instability, or other damage caused by the use of this code.

## Features
- Allocates and hammers memory to detect unusual latency or potential vulnerability indicators.
- Extremely lightweight and safe (does not attempt to exploit or flip actual DRAM bits).
- Fast hammering simulation with timing.

## Usage

```bash
git clone git@github.com:KushalMeghani1644/Rusty-Checker.git
cd Rusty-Checker
cargo run --release
```
# BUILT WITH ❤️ IN RUST
