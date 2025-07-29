use std::time::{Duration, Instant};

const MEMORY_SIZE_MB: usize = 128;
const HAMMER_ROUNDS: usize = 1_000_000;

fn main() {
    println!("Rusty-Checker: RowHammer vulnerability checker");
    println!("Allocation {} MB of memory", MEMORY_SIZE_MB);

    let size = MEMORY_SIZE_MB * 1024 * 1024;
    let mut memory = vec![0u8; size];

    let row_size = 8192;
    let row1_idx = size / 3;
    let row2_idx = row1_idx + row_size;

    println!(
        "Hammering memory region between index {} and {}",
        row1_idx, row2_idx
    );

    let mut dummy = 0u8;

    let start = Instant::now();

    for _ in 0..HAMMER_ROUNDS {
        dummy ^= memory[row1_idx];
        dummy ^= memory[row2_idx];
    }

    let duration = start.elapsed();
    println!("Hammering took: {:?}", duration);

    if memory[row1_idx] != 0 || memory[row2_idx] != 0 {
        println!("Possible anomaly detected: Data not as expected");
    } else {
        println!("No possible anomaly detected");
    }

    std::fs::write("/tmp/Rusty-Checker_result", format!("{}", dummy)).ok();
}
