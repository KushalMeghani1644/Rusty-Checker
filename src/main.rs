use std::env;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::Instant;

struct Config {
    memory_size_mb: usize,
    hammer_count: usize,
}

impl Config {
    fn from_args() -> Result<Self, String> {
        let args: Vec<String> = env::args().collect();
        if args.len() != 3 {
            return Err(format!(
                "Usage: {} <memory_size_mb> <hammer_count>",
                args[0]
            ));
        }

        let memory_size_mb = args[1]
            .parse::<usize>()
            .map_err(|_| "Invalid memory size".to_string())?;

        let hammer_count = args[2]
            .parse::<usize>()
            .map_err(|_| "Invalid hammer count".to_string())?;

        Ok(Config {
            memory_size_mb,
            hammer_count,
        })
    }
}

fn log_message<W: Write + ?Sized>(writer: &mut W, message: &str) -> io::Result<()> {
    writeln!(writer, "{}", message)?;
    Ok(())
}

fn hammer_memory<W: Write + ?Sized>(
    config: &Config,
    writer: &mut W,
) -> Result<(), Box<dyn std::error::Error>> {
    let size_bytes = config.memory_size_mb * 1024 * 1024;
    let num_elements = size_bytes / std::mem::size_of::<u64>();
    let mut memory = vec![0u64; num_elements];

    log_message(
        writer,
        &format!("Allocated {} MB of memory.", config.memory_size_mb),
    )?;

    // Fill memory with a known pattern
    for i in 0..num_elements {
        memory[i] = 0xAAAAAAAAAAAAAAAA;
    }

    let mut anomaly_count = 0;
    let hammer_step = 4096; // simulate row-like behavior

    let start_time = Instant::now();

    for i in (1..num_elements - 1).step_by(hammer_step) {
        for _ in 0..config.hammer_count {
            let _ = memory[i - 1].wrapping_add(1); // Row before
            let _ = memory[i + 1].wrapping_add(1); // Row after
        }

        // Check if the hammered row has flipped
        if memory[i] != 0xAAAAAAAAAAAAAAAA {
            anomaly_count += 1;
            log_message(
                writer,
                &format!("⚠️  Anomaly detected at index {}: 0x{:X}", i, memory[i]),
            )?;
        }
    }

    let duration = start_time.elapsed();

    log_message(
        writer,
        &format!("✅ Hammering complete in {:.2?}.", duration),
    )?;
    log_message(
        writer,
        &format!("🧪 Total anomalies detected: {}", anomaly_count),
    )?;

    Ok(())
}

fn main() {
    let config = match Config::from_args() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    let mut log_path = dirs::config_dir().unwrap_or_else(|| {
        eprintln!("Could not determine config directory path.");
        std::process::exit(1);
    });
    log_path.push("rusty_checker");

    if let Err(e) = fs::create_dir_all(&log_path) {
        eprintln!("Failed to create log directory: {}", e);
        std::process::exit(1);
    }
    log_path.push("rusty_checker.log");

    let log_file = File::create(&log_path).unwrap_or_else(|e| {
        eprintln!("Failed to create log file: {}", e);
        std::process::exit(1);
    });

    let mut log_writer: Box<dyn Write> = Box::new(io::BufWriter::new(log_file));

    let _ = log_message(&mut io::stdout(), "Starting rusty_checker...");
    let _ = log_message(&mut log_writer, "Starting rusty_checker...");

    if let Err(e) = hammer_memory(&config, &mut log_writer) {
        eprintln!("rusty_checker failed: {}", e);
        std::process::exit(1);
    }

    let msg = format!("Log saved to: {}", log_path.display());
    let _ = log_message(&mut io::stdout(), &msg);
    let _ = log_message(&mut log_writer, &msg);
}
