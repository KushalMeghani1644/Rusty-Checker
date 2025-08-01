use std::env;
use std::time::{Duration, Instant};

fn main() {
    println!("Rusty-Checker: RowHammer vulnerability checker");
    println!("Usage: ./Rusty-Checker <MEM_MB> <HAMMER_COUNT>");
    let args: Vec<String> = env::args().collect();
    let default_mb = 128;
    let default_hammer_count = 100_000;
    let hammer_limit = 10_000_000;

    let mb: usize = if args.len() > 1 {
        args[1].parse().unwrap_or(default_mb)
    } else {
        default_mb
    };

    let mut hammer_count: usize = args
        .get(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(default_hammer_count);

    if hammer_count > hammer_limit {
        println!(
            "Hammer count capped to {} to prevent system overload.",
            hammer_limit
        );
        hammer_count = hammer_limit;
    }

    let size = mb * 1024 * 1024;
    println!("Allocation {} MB of memory", mb);

    let mut memory = vec![0xFFu8; size];

    let row_size = 8192;
    let row1_idx = size / 3;
    let row2_idx = row1_idx + row_size;

    println!(
        "Hammering memory region between index {} and {} ({} times)",
        row1_idx, row2_idx, hammer_count
    );

    let mut dummy = 0u8;

    let start = Instant::now();
    for _ in 0..hammer_count {
        memory[row1_idx] = 0x00;
        memory[row2_idx] = 0x00;
        memory[row1_idx] = 0xFF;
        memory[row2_idx] = 0xFF;
    }

    let duration = start.elapsed();
    println!("Hammering took: {:?}", duration);

    let mut anomaly_found = false;
    for (i, byte) in memory.iter().enumerate() {
        if *byte != 0xFF && i != row1_idx && i != row2_idx {
            println!(
                "Possible RowHammer anomaly at index {}: value = {}",
                i, byte
            );
            anomaly_found = true;
            break;
        }
    }
    if anomaly_found {
        println!("❌ Possible anomaly detected");
    } else {
        println!("✅ No possible anomaly detected");
    }

    std::fs::write("/tmp/Rusty-Checker_result", format!("{}", dummy)).ok();
}
