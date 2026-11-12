use open_revo_lib::driver::ec_bridge::EcBridge;
use open_revo_lib::driver::gpu_bridge::GpuBridge;
use std::env;

fn main() {
    println!("============================================================");
    println!("       OpenRevo Hardware Probe & Diagnostic Tool            ");
    println!("============================================================");

    // 1. Initialize EC Bridge
    println!("[*] Initializing EC Bridge...");
    let ec = EcBridge::auto_load();
    if ec.is_mock {
        println!("[!] Note: Running in SAFE SIMULATION (MOCK) mode. Zero kernel driver risks.");
    } else {
        println!("[✓] Real hardware driver DLL loaded successfully.");
    }

    // 2. Initialize GPU Bridge
    println!("[*] Initializing GPU Bridge...");
    let gpu = GpuBridge::auto_load();
    if gpu.is_mock {
        println!("[!] Note: GPU bridge is running in SIMULATION mode.");
    } else {
        println!("[✓] GPU driver loaded successfully.");
    }

    println!("------------------------------------------------------------");

    // Handle CLI arguments
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        let cmd = &args[1];
        match cmd.as_str() {
            "--set-mode" => {
                if let Some(val) = args.get(2) {
                    let mode_num: u8 = match val.to_lowercase().as_str() {
                        "office" | "1" => 1,
                        "balance" | "2" => 2,
                        "beast" | "3" => 3,
                        "custom" | "4" => 4,
                        _ => val.parse().unwrap_or(2),
                    };
                    println!("[>] Setting Power Mode to: {} (raw: {})", val, mode_num);
                    ec.set_power_mode(mode_num);
                    println!("[✓] Power mode set successfully.");
                }
            }
            "--set-fan-boost" => {
                if let Some(val) = args.get(2) {
                    let enable = val == "on" || val == "1" || val == "true";
                    println!("[>] Setting Fan Boost to: {}", enable);
                    if let Err(error) = ec.set_fan_boost(enable) { eprintln!("Fan Boost failed: {}", error); std::process::exit(1); }
                    println!("[✓] Fan boost set successfully.");
                }
            }
            "--set-battery-limit" => {
                if let Some(val) = args.get(2) {
                    let limit: u8 = val.parse().unwrap_or(80);
                    println!("[>] Setting Battery Charge Limit to: {}%", limit);
                    ec.set_battery_limit(limit);
                    println!("[✓] Battery limit set successfully.");
                }
            }
            "--help" | "-h" => {
                println!("Usage:");
                println!("  hardware_probe                         # Show current status");
                println!("  hardware_probe --set-mode <office|balance|beast>");
                println!("  hardware_probe --set-fan-boost <on|off>");
                println!("  hardware_probe --set-battery-limit <80|100>");
                return;
            }
            _ => {
                println!("[!] Unknown argument: {}", cmd);
            }
        }
        println!("------------------------------------------------------------");
    }

    // 3. Read & Display Current Hardware Status
    println!("[EC Controller Status]");
    let power_mode = ec.get_power_mode();
    let mode_str = match power_mode {
        1 => "1 (Office / 办公)",
        2 => "2 (Balance / 均衡)",
        3 => "3 (Beast / 狂暴)",
        4 => "4 (Custom / 自定义)",
        other => Box::leak(format!("{} (Unknown)", other).into_boxed_str()),
    };
    println!("  - Power Mode:          {}", mode_str);
    println!("  - Fan Boost Max:       {}", if ec.is_fan_boost() { "ON (开启)" } else { "OFF (自动)" });
    println!("  - Battery Limit:       {}%", ec.get_battery_limit());
    println!("  - CPU Temperature:     {} °C", ec.get_cpu_temp());
    println!("  - GPU Temperature:     {} °C", ec.get_gpu_temp());
    println!("  - CPU Fan RPM (Raw):   {} RPM", ec.get_cpu_fan_rpm());
    println!("  - GPU Fan RPM (Raw):   {} RPM", ec.get_gpu_fan_rpm());

    println!("\n[NVIDIA GPU Status]");
    let count = gpu.get_gpu_count();
    println!("  - Detected GPU Count:  {}", count);
    for i in 0..count {
        println!("  - GPU #{}:", i);
        println!("    * Temperature:       {} °C", gpu.get_temp(i));
        println!("    * Usage:             {}%", gpu.get_usage(i));
        println!("    * Core Frequency:    {} MHz", gpu.get_core_freq(i));
        println!("    * Memory Frequency:  {} MHz", gpu.get_mem_freq(i));
        println!("    * Physical VRAM:     {} MB", gpu.get_vram_mb(i));
        let (def_pwr, max_pwr) = gpu.get_power_limits();
        println!("    * VBIOS Base TGP:    {} W", def_pwr);
        println!("    * VBIOS Max TGP(DB): {} W", max_pwr);
    }

    println!("\n[Model Profile & Factory Power Presets]");
    let model_match = open_revo_lib::system::model_engine::detect_model_profile();
    println!("  - Matched Model:       {}", model_match.matched_profile.display_name);
    println!("  - Confidence:          {}", model_match.confidence_reason);
    println!("  - Models Config File:  {:?}", open_revo_lib::system::model_engine::get_models_file_path());
    let fp = open_revo_lib::system::power_mode::get_factory_power_presets(count > 0);
    println!("  - Factory Office PL1:  {} W", fp.office_pl1);
    println!("  - Factory Balance PL1: {} W", fp.balance_pl1);
    println!("  - Factory Turbo PL1:   {} W", fp.turbo_pl1);
    println!("  - Factory Base TGP:    {} W", fp.base_tgp);
    println!("  - Factory Max DB:      +{} W (Total: {} W)", fp.max_db, fp.base_tgp + fp.max_db);
    println!("  - Hardware Read Flag:  {}", fp.hardware_read);

    println!("============================================================");
}

