//! Read-only probe of the same Intel iGPU telemetry used by the dashboard.
//! cargo run -p openrevo-core --example intel_gpu -- --samples 3
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let count = args
        .windows(2)
        .find(|v| v[0] == "--samples")
        .and_then(|v| v[1].parse::<u32>().ok())
        .unwrap_or(1)
        .clamp(1, 60);
    for index in 0..count {
        let value = openrevo_core::hal::intel_gpu::sample();
        println!("{}", serde_json::to_string(&value).unwrap());
        if value.error.is_some() {
            std::process::exit(1);
        }
        if index + 1 < count {
            std::thread::sleep(std::time::Duration::from_millis(1500));
        }
    }
}
