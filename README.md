# Rusty-Checker

⚡ A simple Rust-based **RowHammer vulnerability checker**

Rusty-Checker allocates a block of memory and simulates hammering adjacent rows to detect any bit flips or anomalies that may indicate a RowHammer vulnerability on your system.

> ⚠️ **WARNING**  
> This tool performs memory access patterns that may cause unintended side effects on vulnerable hardware.  
> It is intended for educational and diagnostic purposes **only**.  
> **Use at your own risk**. The author is **not responsible** for any data corruption, hardware instability, or other damage caused by the use of this code.

## Trademark Notice
The names “Rusty-Suite”, “RustyTodos”, “RustyBoot”, and “Rusty-Checker” are part of this project’s identity.  
See [TRADEMARK.md](TRADEMARK.md) for details.

---

## 🚀 Features

- Memory hammering simulation in pure Rust.
- Detects possible RowHammer-style anomalies via timing and access patterns.
- Lightweight and safe (does **not** attempt to exploit or flip actual DRAM bits).
- Configurable via CLI: memory size (in MB), hammer count, and more.

---

## 🛠 Usage

```bash
git clone git@github.com:KushalMeghani1644/Rusty-Checker.git
cd Rusty-Checker
cargo run --release
cd targets/release
./Rusty-Checker <RAM in MB> <HAMMER COUNT>
```

# BUILT WITH ❤️ IN RUST

