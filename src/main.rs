// rusty_checker_v2.rs
// Polished single-file release-ready version of Rusty-Checker
// Features added:
// - clap for robust CLI parsing
// - chrono for proper timestamps
// - rand for better random testing
// - improved error handling and clearer logging
// - sensible caps and guards to avoid accidental huge allocations
// - cleaner progress messages and structured TestResults

use chrono::{DateTime, Utc};
use clap::{ArgGroup, Parser, ValueEnum};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

const MAX_MEMORY_MB: usize = 32 * 1024; // 32 GB cap by default for safety
const DEFAULT_STEP: usize = 4096; // in elements (u64 units)
const DEFAULT_SEED: u64 = 0xDEADBEEFCAFEBABE;

#[derive(thiserror::Error, Debug)]
enum TestError {
    #[error("Memory allocation error: {0}")]
    MemoryAllocation(String),

    #[error("I/O error: {0}")]
    IoError(#[from] io::Error),

    #[error("Configuration error: {0}")]
    InvalidConfig(String),
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum Mode {
    Rowhammer,
    Sequential,
    Random,
    Checkerboard,
}

#[derive(Parser, Debug)]
#[command(author, version, about = "Rusty-Checker — memory stress & RowHammer style tester", long_about = None)]
#[command(group(ArgGroup::new("logging").args(["log_path"])))]
struct Cli {
    /// Memory size in megabytes to allocate and test
    memory_size_mb: usize,

    /// Number of hammer iterations (per location / per pattern)
    hammer_count: usize,

    /// Test mode (rowhammer, sequential, random, checkerboard)
    #[arg(value_enum, default_value_t = Mode::Rowhammer)]
    mode: Mode,

    /// Be chatty about progress
    #[arg(short, long)]
    verbose: bool,

    /// Step size in bytes between "rows" for rowhammer mode (default: 4096)
    #[arg(long, default_value_t = DEFAULT_STEP)]
    step_size: usize,

    /// Optional log path (directory or full file path). If directory provided, rusty_checker.log will be used inside it.
    #[arg(long)]
    log_path: Option<PathBuf>,

    /// Seed for deterministic random mode (optional)
    #[arg(long, default_value_t = DEFAULT_SEED)]
    seed: u64,
}

#[derive(Debug, Clone)]
struct ErrorLocation {
    index: usize,
    expected: u64,
    actual: u64,
    pattern_id: usize,
}

#[derive(Debug)]
struct TestResults {
    mode: Mode,
    memory_size_mb: usize,
    hammer_count: usize,
    step_size: usize,
    total_tests: usize,
    anomalies_found: usize,
    test_duration: Duration,
    memory_bandwidth_mbps: f64,
    error_locations: Vec<ErrorLocation>,
}

const PATTERNS: [u64; 4] = [
    0xAAAAAAAAAAAAAAAA,
    0x5555555555555555,
    0xFFFFFFFFFFFFFFFF,
    0x0000000000000000,
];

struct MemoryTester {
    memory_size_mb: usize,
    hammer_count: usize,
    mode: Mode,
    verbose: bool,
    step_size: usize,
    log_path: Option<PathBuf>,
    rng_seed: u64,
}

impl MemoryTester {
    fn new(cli: Cli) -> Result<Self, TestError> {
        if cli.memory_size_mb == 0 {
            return Err(TestError::InvalidConfig(
                "memory_size_mb must be > 0".into(),
            ));
        }
        if cli.hammer_count == 0 {
            return Err(TestError::InvalidConfig("hammer_count must be > 0".into()));
        }
        if cli.memory_size_mb > MAX_MEMORY_MB {
            return Err(TestError::InvalidConfig(format!(
                "memory_size_mb is capped to {} MB for safety. Requested: {} MB",
                MAX_MEMORY_MB, cli.memory_size_mb
            )));
        }
        if cli.step_size == 0 {
            return Err(TestError::InvalidConfig("step_size must be > 0".into()));
        }

        Ok(Self {
            memory_size_mb: cli.memory_size_mb,
            hammer_count: cli.hammer_count,
            mode: cli.mode,
            verbose: cli.verbose,
            step_size: cli.step_size,
            log_path: cli.log_path,
            rng_seed: cli.seed,
        })
    }

    fn timestamp() -> String {
        let now: DateTime<Utc> = Utc::now();
        now.format("%Y-%m-%d %H:%M:%S UTC").to_string()
    }

    fn log_message<W: Write + ?Sized>(
        &self,
        writer: &mut W,
        message: &str,
    ) -> Result<(), TestError> {
        let ts = Self::timestamp();
        writeln!(writer, "[{}] {}", ts, message)?;
        if self.verbose {
            println!("[{}] {}", ts, message);
        }
        Ok(())
    }

    fn allocate_memory(&self) -> Result<Vec<u64>, TestError> {
        let size_bytes = self
            .memory_size_mb
            .checked_mul(1024 * 1024)
            .ok_or_else(|| {
                TestError::MemoryAllocation("Requested memory size overflowed".into())
            })?;

        let num_elements = size_bytes / std::mem::size_of::<u64>();

        if num_elements == 0 {
            return Err(TestError::MemoryAllocation("Memory size too small".into()));
        }

        // Try to allocate. This may OOM and panic; we catch via Vec::with_capacity then resize.
        let mut memory = Vec::with_capacity(num_elements);
        // fill with zeros
        memory.resize(num_elements, 0u64);

        Ok(memory)
    }

    fn fill_pattern(&self, memory: &mut [u64], pattern: u64) {
        memory.iter_mut().for_each(|elem| *elem = pattern);
    }

    fn test_rowhammer(
        &self,
        memory: &mut [u64],
        writer: &mut dyn Write,
    ) -> Result<usize, TestError> {
        // interpret step_size as bytes -> convert to u64 elements if divisible by 8
        let elem_step = if self.step_size % std::mem::size_of::<u64>() == 0 {
            self.step_size / std::mem::size_of::<u64>()
        } else {
            1usize
        };

        let num_elements = memory.len();
        if elem_step == 0 || elem_step >= num_elements / 2 {
            return Err(TestError::InvalidConfig(
                "step_size is too large relative to allocated memory".into(),
            ));
        }

        self.log_message(
            writer,
            &format!(
                "Starting rowhammer: elem_step={}, hammer_count={}",
                elem_step, self.hammer_count
            ),
        )?;

        let mut tests = 0usize;
        for center in (elem_step..(num_elements - elem_step)).step_by(elem_step) {
            // hammer adjacent locations
            for _ in 0..self.hammer_count {
                // read-modify-write on adjacent locations
                let a_idx = center - elem_step;
                let b_idx = center + elem_step;

                let a = memory[a_idx];
                let b = memory[b_idx];

                // simple RMW
                memory[a_idx] = a.wrapping_add(1);
                memory[b_idx] = b.wrapping_add(1);

                // keep values alive for optimizer
                std::hint::black_box((a, b));
            }

            tests += 1;

            if self.verbose && tests % 1000 == 0 {
                let progress = (center as f64 / num_elements as f64) * 100.0;
                self.log_message(
                    writer,
                    &format!("Rowhammer progress: {:.2}% ({} tests)", progress, tests),
                )?;
            }
        }

        Ok(tests)
    }

    fn test_sequential(
        &self,
        memory: &mut [u64],
        writer: &mut dyn Write,
    ) -> Result<usize, TestError> {
        self.log_message(writer, "Starting sequential test")?;
        let len = memory.len();
        for i in 0..len {
            for _ in 0..self.hammer_count {
                let v = memory[i];
                memory[i] = v.wrapping_add(1);
                std::hint::black_box(v);
            }
            if self.verbose && i % 100_000 == 0 && i > 0 {
                let prog = (i as f64 / len as f64) * 100.0;
                self.log_message(writer, &format!("Sequential progress: {:.2}%", prog))?;
            }
        }
        Ok(len)
    }

    fn test_random(&self, memory: &mut [u64], writer: &mut dyn Write) -> Result<usize, TestError> {
        self.log_message(writer, "Starting random test")?;
        let mut rng = StdRng::seed_from_u64(self.rng_seed);
        let num_ops = (self.hammer_count as usize)
            .saturating_mul(10_000)
            .min(memory.len() * 2);

        for i in 0..num_ops {
            let idx = rng.gen_range(0..memory.len());
            let v = memory[idx];
            memory[idx] = v.wrapping_add(1);
            std::hint::black_box(v);

            if self.verbose && i % 10_000 == 0 && i > 0 {
                let prog = (i as f64 / num_ops as f64) * 100.0;
                self.log_message(writer, &format!("Random progress: {:.2}%", prog))?;
            }
        }

        Ok(num_ops)
    }

    fn test_checkerboard(
        &self,
        memory: &mut [u64],
        writer: &mut dyn Write,
    ) -> Result<usize, TestError> {
        self.log_message(writer, "Starting checkerboard test")?;

        for iter in 0..self.hammer_count {
            let base = if iter % 2 == 0 {
                PATTERNS[0]
            } else {
                PATTERNS[1]
            };
            for (i, e) in memory.iter_mut().enumerate() {
                *e = if i % 2 == 0 { base } else { !base };
            }
            if self.verbose && iter % 100 == 0 {
                let prog = (iter as f64 / self.hammer_count as f64) * 100.0;
                self.log_message(writer, &format!("Checkerboard progress: {:.2}%", prog))?;
            }
        }
        Ok(memory.len() * self.hammer_count)
    }

    fn check_integrity(
        &self,
        memory: &[u64],
        expected: u64,
        pattern_id: usize,
    ) -> Vec<ErrorLocation> {
        let mut errors = Vec::new();
        for (i, &v) in memory.iter().enumerate() {
            if v != expected {
                errors.push(ErrorLocation {
                    index: i,
                    expected,
                    actual: v,
                    pattern_id,
                });
                if errors.len() >= 10_000 {
                    break;
                }
            }
        }
        errors
    }

    fn default_log_path() -> PathBuf {
        if let Ok(home) = env::var("HOME") {
            let mut p = PathBuf::from(home);
            p.push(".rusty_checker");
            p
        } else {
            PathBuf::from("/tmp/rusty_checker")
        }
    }

    fn open_log(&self) -> Result<BufWriter<File>, TestError> {
        let path = if let Some(ref p) = self.log_path {
            if p.is_dir() {
                let mut file = p.clone();
                file.push("rusty_checker.log");
                file
            } else if p.ends_with(".log") {
                p.clone()
            } else if p.exists() && p.is_file() {
                p.clone()
            } else if p.parent().is_some() {
                // allow a full path that may not exist yet
                p.clone()
            } else {
                // fallback
                let mut d = Self::default_log_path();
                d.push("rusty_checker.log");
                d
            }
        } else {
            let mut d = Self::default_log_path();
            fs::create_dir_all(&d)?;
            d.push("rusty_checker.log");
            d
        };

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)?;
        Ok(BufWriter::new(file))
    }

    fn run(&self) -> Result<TestResults, TestError> {
        let mut writer = self.open_log()?;

        self.log_message(&mut writer, "Starting rusty_checker (v2) ...")?;
        self.log_message(
            &mut writer,
            &format!(
                "Mode: {:?}, Memory: {} MB, Hammer count: {}",
                self.mode, self.memory_size_mb, self.hammer_count
            ),
        )?;

        let mut memory = self.allocate_memory()?;
        self.log_message(
            &mut writer,
            &format!(
                "Allocated {} MB ({} u64 elements)",
                self.memory_size_mb,
                memory.len()
            ),
        )?;
        self.log_message(
            &mut writer,
            &format!(
                "System: {} {}",
                std::env::consts::OS,
                std::env::consts::ARCH
            ),
        )?;

        let start = Instant::now();
        let mut all_errors = Vec::new();
        let mut total_tests: usize = 0;

        for (pid, &pattern) in PATTERNS.iter().enumerate() {
            self.log_message(
                &mut writer,
                &format!(
                    "Testing pattern {}/{}: 0x{:016X}",
                    pid + 1,
                    PATTERNS.len(),
                    pattern
                ),
            )?;
            self.fill_pattern(&mut memory, pattern);

            let tests_done = match self.mode {
                Mode::Rowhammer => self.test_rowhammer(&mut memory, &mut writer)?,
                Mode::Sequential => self.test_sequential(&mut memory, &mut writer)?,
                Mode::Random => self.test_random(&mut memory, &mut writer)?,
                Mode::Checkerboard => self.test_checkerboard(&mut memory, &mut writer)?,
            };

            total_tests = total_tests.saturating_add(tests_done);

            let errors = self.check_integrity(&memory, pattern, pid);
            if !errors.is_empty() {
                self.log_message(
                    &mut writer,
                    &format!(
                        "Found {} anomalies for pattern 0x{:016X}",
                        errors.len(),
                        pattern
                    ),
                )?;
                for e in errors.iter().take(5) {
                    self.log_message(
                        &mut writer,
                        &format!(
                            "  Index {}: expected 0x{:016X}, got 0x{:016X}",
                            e.index, e.expected, e.actual
                        ),
                    )?;
                }
                if errors.len() > 5 {
                    self.log_message(&mut writer, &format!("  ... and {} more", errors.len() - 5))?;
                }
            }

            all_errors.extend(errors);
        }

        let duration = start.elapsed();
        let total_bytes = (total_tests as f64) * (std::mem::size_of::<u64>() as f64);
        let bandwidth = if duration.as_secs_f64() > 0.0 {
            (total_bytes / 1024.0 / 1024.0) / duration.as_secs_f64()
        } else {
            0.0
        };

        let results = TestResults {
            mode: self.mode,
            memory_size_mb: self.memory_size_mb,
            hammer_count: self.hammer_count,
            step_size: self.step_size,
            total_tests,
            anomalies_found: all_errors.len(),
            test_duration: duration,
            memory_bandwidth_mbps: bandwidth,
            error_locations: all_errors,
        };

        self.log_message(&mut writer, &format!("Completed in {:.2?}", duration))?;
        self.log_message(
            &mut writer,
            &format!("Anomalies: {}", results.anomalies_found),
        )?;
        self.log_message(
            &mut writer,
            &format!("Bandwidth: {:.2} MB/s", results.memory_bandwidth_mbps),
        )?;

        // flush final messages
        writer.flush()?;

        Ok(results)
    }
}

fn main() {
    let cli = Cli::parse();

    let tester = match MemoryTester::new(cli) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Configuration error: {}", e);
            std::process::exit(1);
        }
    };

    match tester.run() {
        Ok(results) => {
            println!(
                "✅ Test finished: {} anomalies in {} MB (mode: {:?})",
                results.anomalies_found, results.memory_size_mb, results.mode
            );
            if let Some(path) = tester.log_path.as_ref() {
                println!("📝 Log (user-provided): {}", path.display());
            } else {
                println!(
                    "📝 Log saved to default location (~/.rusty_checker/rusty_checker.log or /tmp)"
                );
            }
        }
        Err(e) => {
            eprintln!("❌ rusty_checker failed: {}", e);
            std::process::exit(2);
        }
    }
}
