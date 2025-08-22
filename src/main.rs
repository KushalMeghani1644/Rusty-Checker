use std::env;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug)]
enum TestError {
    MemoryAllocation(String),
    IoError(io::Error),
    InvalidConfig(String),
}

impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TestError::MemoryAllocation(msg) => write!(f, "Memory allocation error: {}", msg),
            TestError::IoError(err) => write!(f, "I/O error: {}", err),
            TestError::InvalidConfig(msg) => write!(f, "Configuration error: {}", msg),
        }
    }
}

impl std::error::Error for TestError {}

impl From<io::Error> for TestError {
    fn from(error: io::Error) -> Self {
        TestError::IoError(error)
    }
}

#[derive(Debug, Clone, Copy)]
enum TestMode {
    RowHammer,
    Sequential,
    Random,
    Checkerboard,
}

impl TestMode {
    fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "rowhammer" | "hammer" => Ok(TestMode::RowHammer),
            "sequential" | "seq" => Ok(TestMode::Sequential),
            "random" | "rand" => Ok(TestMode::Random),
            "checkerboard" | "checker" => Ok(TestMode::Checkerboard),
            _ => Err(format!("Invalid test mode: {}", s)),
        }
    }
}

#[derive(Debug)]
struct Config {
    memory_size_mb: usize,
    hammer_count: usize,
    mode: TestMode,
    verbose: bool,
    step_size: usize,
    log_path: Option<PathBuf>,
}

impl Config {
    fn from_args() -> Result<Self, String> {
        let args: Vec<String> = env::args().collect();

        if args.len() < 3 {
            return Err(format!(
                "Usage: {} <memory_size_mb> <hammer_count> [mode] [--verbose] [--step-size N] [--log-path PATH]\n\
                 Modes: rowhammer (default), sequential, random, checkerboard\n\
                 Example: {} 1024 100000 rowhammer --verbose",
                args[0], args[0]
            ));
        }

        let memory_size_mb = args[1]
            .parse::<usize>()
            .map_err(|_| "Invalid memory size".to_string())?;

        let hammer_count = args[2]
            .parse::<usize>()
            .map_err(|_| "Invalid hammer count".to_string())?;

        let mut mode = TestMode::RowHammer;
        let mut verbose = false;
        let mut step_size = 4096;
        let mut log_path = None;
        let mut i = 3;

        // Parse optional arguments
        while i < args.len() {
            match args[i].as_str() {
                "--verbose" | "-v" => verbose = true,
                "--step-size" => {
                    i += 1;
                    if i >= args.len() {
                        return Err("--step-size requires a value".to_string());
                    }
                    step_size = args[i]
                        .parse()
                        .map_err(|_| "Invalid step size".to_string())?;
                }
                "--log-path" => {
                    i += 1;
                    if i >= args.len() {
                        return Err("--log-path requires a value".to_string());
                    }
                    log_path = Some(PathBuf::from(&args[i]));
                }
                arg => {
                    // Try to parse as mode if it's the first positional arg after required ones
                    if i == 3 {
                        mode = TestMode::from_str(arg)?;
                    } else {
                        return Err(format!("Unknown argument: {}", arg));
                    }
                }
            }
            i += 1;
        }

        if memory_size_mb == 0 {
            return Err("Memory size must be greater than 0".to_string());
        }

        if hammer_count == 0 {
            return Err("Hammer count must be greater than 0".to_string());
        }

        Ok(Config {
            memory_size_mb,
            hammer_count,
            mode,
            verbose,
            step_size,
            log_path,
        })
    }
}

#[derive(Debug)]
struct ErrorLocation {
    index: usize,
    expected: u64,
    actual: u64,
    pattern_id: usize,
}

#[derive(Debug)]
struct TestResults {
    test_mode: TestMode,
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
    config: Config,
}

impl MemoryTester {
    fn new(config: Config) -> Self {
        MemoryTester { config }
    }

    fn get_timestamp() -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0));
        let secs = now.as_secs();
        let naive = secs;

        // Simple timestamp formatting (YYYY-MM-DD HH:MM:SS)
        let days_since_epoch = secs / 86400;
        let days_since_1970 = days_since_epoch;
        let year = 1970 + (days_since_1970 / 365) + ((days_since_1970 / 365) / 4); // Rough approximation
        let remaining_secs = secs % 86400;
        let hours = remaining_secs / 3600;
        let minutes = (remaining_secs % 3600) / 60;
        let seconds = remaining_secs % 60;

        format!(
            "{:04}-XX-XX {:02}:{:02}:{:02} UTC",
            year, hours, minutes, seconds
        )
    }

    fn log_message<W: Write + ?Sized>(
        &self,
        writer: &mut W,
        message: &str,
    ) -> Result<(), TestError> {
        let timestamp = Self::get_timestamp();
        writeln!(writer, "[{}] {}", timestamp, message)?;
        if self.config.verbose {
            println!("[{}] {}", timestamp, message);
        }
        Ok(())
    }

    fn allocate_memory(&self, size_bytes: usize) -> Result<Vec<u64>, TestError> {
        let num_elements = size_bytes / std::mem::size_of::<u64>();

        if num_elements == 0 {
            return Err(TestError::MemoryAllocation(
                "Memory size too small".to_string(),
            ));
        }

        // Try to allocate memory
        let memory = vec![0u64; num_elements];
        if memory.len() != num_elements {
            return Err(TestError::MemoryAllocation(
                "Failed to allocate requested memory size".to_string(),
            ));
        }

        Ok(memory)
    }

    fn fill_memory_pattern(&self, memory: &mut [u64], pattern: u64) {
        for element in memory.iter_mut() {
            *element = pattern;
        }
    }

    fn test_rowhammer(
        &self,
        memory: &mut [u64],
        writer: &mut dyn Write,
    ) -> Result<usize, TestError> {
        let num_elements = memory.len();
        let step = self.config.step_size.min(num_elements / 2);

        self.log_message(
            writer,
            &format!("🔨 Starting row hammer test with step size: {}", step),
        )?;

        let mut tests_performed = 0;
        for i in (step..num_elements - step).step_by(step) {
            // Hammer adjacent "rows"
            for _ in 0..self.config.hammer_count {
                // Read and write adjacent memory locations
                let temp1 = memory[i - step];
                let temp2 = memory[i + step];

                memory[i - step] = temp1.wrapping_add(1);
                memory[i + step] = temp2.wrapping_add(1);

                // Prevent compiler optimization
                std::hint::black_box((temp1, temp2));
            }

            tests_performed += 1;

            // Progress reporting
            if self.config.verbose && tests_performed % 1000 == 0 {
                let progress = (i as f64 / num_elements as f64) * 100.0;
                self.log_message(
                    writer,
                    &format!("Progress: {:.1}% ({} tests)", progress, tests_performed),
                )?;
            }
        }

        Ok(tests_performed)
    }

    fn test_sequential(
        &self,
        memory: &mut [u64],
        writer: &mut dyn Write,
    ) -> Result<usize, TestError> {
        self.log_message(writer, "🔄 Starting sequential memory test")?;

        for i in 0..memory.len() {
            for _ in 0..self.config.hammer_count {
                let temp = memory[i];
                memory[i] = temp.wrapping_add(1);
                std::hint::black_box(temp);
            }

            if self.config.verbose && i % 100000 == 0 && i > 0 {
                let progress = (i as f64 / memory.len() as f64) * 100.0;
                self.log_message(
                    writer,
                    &format!("Sequential test progress: {:.1}%", progress),
                )?;
            }
        }

        Ok(memory.len())
    }

    fn test_random(&self, memory: &mut [u64], writer: &mut dyn Write) -> Result<usize, TestError> {
        self.log_message(writer, "🎲 Starting random memory test")?;

        let num_tests = memory.len().min(self.config.hammer_count * 1000);

        for i in 0..num_tests {
            // Simple pseudo-random index generation using linear congruential generator
            let random_index = ((i.wrapping_mul(1103515245).wrapping_add(12345)) % memory.len());

            let temp = memory[random_index];
            memory[random_index] = temp.wrapping_add(1);
            std::hint::black_box(temp);

            if self.config.verbose && i % 10000 == 0 && i > 0 {
                let progress = (i as f64 / num_tests as f64) * 100.0;
                self.log_message(writer, &format!("Random test progress: {:.1}%", progress))?;
            }
        }

        Ok(num_tests)
    }

    fn test_checkerboard(
        &self,
        memory: &mut [u64],
        writer: &mut dyn Write,
    ) -> Result<usize, TestError> {
        self.log_message(writer, "🏁 Starting checkerboard memory test")?;

        // Alternate between two patterns
        for iteration in 0..self.config.hammer_count {
            let pattern = if iteration % 2 == 0 {
                0xAAAAAAAAAAAAAAAA
            } else {
                0x5555555555555555
            };

            for (i, element) in memory.iter_mut().enumerate() {
                let expected_pattern = if i % 2 == 0 { pattern } else { !pattern };
                *element = expected_pattern;
            }

            if self.config.verbose && iteration % 100 == 0 {
                let progress = (iteration as f64 / self.config.hammer_count as f64) * 100.0;
                self.log_message(
                    writer,
                    &format!("Checkerboard test progress: {:.1}%", progress),
                )?;
            }
        }

        Ok(memory.len() * self.config.hammer_count)
    }

    fn check_memory_integrity(
        &self,
        memory: &[u64],
        expected_pattern: u64,
        pattern_id: usize,
    ) -> Vec<ErrorLocation> {
        let mut errors = Vec::new();

        for (i, &value) in memory.iter().enumerate() {
            if value != expected_pattern {
                errors.push(ErrorLocation {
                    index: i,
                    expected: expected_pattern,
                    actual: value,
                    pattern_id,
                });

                // Limit error reporting to prevent excessive output
                if errors.len() >= 10000 {
                    break;
                }
            }
        }

        errors
    }

    fn run_memory_test(&self, writer: &mut dyn Write) -> Result<TestResults, TestError> {
        let size_bytes = self.config.memory_size_mb * 1024 * 1024;
        let mut memory = self.allocate_memory(size_bytes)?;

        self.log_message(
            writer,
            &format!(
                "📊 Allocated {} MB of memory ({} elements)",
                self.config.memory_size_mb,
                memory.len()
            ),
        )?;

        self.log_message(
            writer,
            &format!(
                "🖥️  System: {} {}",
                std::env::consts::OS,
                std::env::consts::ARCH
            ),
        )?;

        let start_time = Instant::now();
        let mut all_errors = Vec::new();
        let mut total_tests = 0;

        for (pattern_id, &pattern) in PATTERNS.iter().enumerate() {
            self.log_message(
                writer,
                &format!(
                    "🎯 Testing pattern {}/{}: 0x{:016X}",
                    pattern_id + 1,
                    PATTERNS.len(),
                    pattern
                ),
            )?;

            self.fill_memory_pattern(&mut memory, pattern);

            // Run the specific test based on mode
            let tests_in_pattern = match self.config.mode {
                TestMode::RowHammer => self.test_rowhammer(&mut memory, writer)?,
                TestMode::Sequential => self.test_sequential(&mut memory, writer)?,
                TestMode::Random => self.test_random(&mut memory, writer)?,
                TestMode::Checkerboard => self.test_checkerboard(&mut memory, writer)?,
            };

            // Check for bit flips after testing
            let integrity_errors = self.check_memory_integrity(&memory, pattern, pattern_id);

            if !integrity_errors.is_empty() {
                self.log_message(
                    writer,
                    &format!(
                        "⚠️  Found {} anomalies with pattern 0x{:016X}",
                        integrity_errors.len(),
                        pattern
                    ),
                )?;

                // Log first few errors
                for error in integrity_errors.iter().take(5) {
                    self.log_message(
                        writer,
                        &format!(
                            "   Index {}: expected 0x{:016X}, got 0x{:016X}",
                            error.index, error.expected, error.actual
                        ),
                    )?;
                }

                if integrity_errors.len() > 5 {
                    self.log_message(
                        writer,
                        &format!("   ... and {} more", integrity_errors.len() - 5),
                    )?;
                }
            }

            all_errors.extend(integrity_errors);
            total_tests += tests_in_pattern;
        }

        let duration = start_time.elapsed();
        let total_bytes = (total_tests * std::mem::size_of::<u64>()) as f64;
        let bandwidth_mbps = (total_bytes / 1024.0 / 1024.0) / duration.as_secs_f64();

        let results = TestResults {
            test_mode: self.config.mode,
            memory_size_mb: self.config.memory_size_mb,
            hammer_count: self.config.hammer_count,
            step_size: self.config.step_size,
            total_tests,
            anomalies_found: all_errors.len(),
            test_duration: duration,
            memory_bandwidth_mbps: bandwidth_mbps,
            error_locations: all_errors,
        };

        self.log_message(writer, &format!("✅ Testing complete in {:.2?}", duration))?;
        self.log_message(
            writer,
            &format!("🧪 Total anomalies detected: {}", results.anomalies_found),
        )?;
        self.log_message(
            writer,
            &format!("⚡ Memory bandwidth: {:.2} MB/s", bandwidth_mbps),
        )?;

        Ok(results)
    }

    fn get_default_log_path() -> PathBuf {
        let mut path = if let Some(home) = env::var("HOME").ok() {
            PathBuf::from(home)
        } else {
            PathBuf::from("/tmp")
        };
        path.push(".rusty_checker");
        path
    }

    fn run(&self) -> Result<(), TestError> {
        // Determine log path
        let log_path = if let Some(ref custom_path) = self.config.log_path {
            custom_path.clone()
        } else {
            let mut path = Self::get_default_log_path();
            if let Err(_) = fs::create_dir_all(&path) {
                // Fallback to /tmp if we can't create in home
                path = PathBuf::from("/tmp/rusty_checker");
                fs::create_dir_all(&path)?;
            }
            path.push("rusty_checker.log");
            path
        };

        // Create log file
        let log_file = File::create(&log_path)?;
        let mut log_writer = BufWriter::new(log_file);

        // Start testing
        println!("🚀 Starting rusty_checker...");
        self.log_message(&mut log_writer, "Starting rusty_checker...")?;

        let results = self.run_memory_test(&mut log_writer)?;

        // Final summary
        let summary = format!(
            "Test completed: {}/{} anomalies found in {} MB of memory",
            results.anomalies_found, results.total_tests, results.memory_size_mb
        );
        println!("🎯 {}", summary);
        println!("📝 Log saved to: {}", log_path.display());
        self.log_message(&mut log_writer, &summary)?;

        Ok(())
    }
}

fn main() {
    let config = match Config::from_args() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    let tester = MemoryTester::new(config);

    if let Err(e) = tester.run() {
        eprintln!("❌ rusty_checker failed: {}", e);
        std::process::exit(1);
    }
}
