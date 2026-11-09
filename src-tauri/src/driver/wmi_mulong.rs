// Native In-Process COM WMI Provider for AcpiTest_MULong
// Dedicated driverless EC communication channel for legacy Tongfang & Hasee laptops (e.g. GK5CP6X / Z2 Air / Z7-CT)
// Uses an isolated Single-Threaded Worker Actor to prevent COM MTA pollution on Tauri's UI thread (STA)
// and guarantee 100% thread safety against 0xC0000005 ACCESS_VIOLATION.

use std::sync::Mutex as StdMutex;
use std::time::Instant;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::LazyLock;
use std::thread;
use std::time::Duration;
use windows::core::{w, BSTR, VARIANT};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoSetProxyBlanket, CoUninitialize, CLSCTX_INPROC_SERVER,
    COINIT_MULTITHREADED, EOAC_NONE, RPC_C_AUTHN_LEVEL_PKT_PRIVACY, RPC_C_IMP_LEVEL_IMPERSONATE,
};
use windows::Win32::System::Wmi::{
    IWbemClassObject, IWbemLocator, IWbemServices, WbemLocator,
    WBEM_FLAG_FORWARD_ONLY, WBEM_FLAG_RETURN_IMMEDIATELY, WBEM_GENERIC_FLAG_TYPE,
};

const RPC_C_AUTHN_WINNT: u32 = 10;
const RPC_C_AUTHZ_NONE: u32 = 0;

/// 探测缓存：成功结果永久有效；失败结果只缓存一小段时间（启动早期 WMI 可能还没就绪）
static MULONG_STATE: StdMutex<Option<(bool, Instant)>> = StdMutex::new(None);
const MULONG_NEGATIVE_TTL: Duration = Duration::from_secs(5);

enum MulongTask {
    Probe(Sender<bool>),
    Read(u16, Sender<Option<u8>>),
    Write(u16, u8, Sender<bool>),
    Raw(u64, Sender<bool>),
    QueryRaw(u64, Sender<Option<u64>>),
}

struct MulongWorker {
    tx: Sender<MulongTask>,
}

static WORKER: LazyLock<Option<MulongWorker>> = LazyLock::new(init_worker);

fn init_worker() -> Option<MulongWorker> {
    let (tx, rx) = channel::<MulongTask>();

    thread::Builder::new()
        .name("wmi-mulong-actor".to_string())
        .spawn(move || {
            worker_loop(rx);
        })
        .ok()?;

    Some(MulongWorker { tx })
}

fn worker_loop(rx: Receiver<MulongTask>) {
    unsafe {
        // Initialize COM exclusively within this dedicated background worker thread
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

        let mut services_opt: Option<IWbemServices> = None;
        let mut in_param_class_opt: Option<IWbemClassObject> = None;
        let mut instance_path_opt: Option<BSTR> = None;
        let mut is_available = false;

        // Try to connect to root\WMI and discover AcpiTest_MULong
        if let Ok(locator) = CoCreateInstance::<_, IWbemLocator>(&WbemLocator, None, CLSCTX_INPROC_SERVER) {
            let namespace = BSTR::from(r"root\WMI");
            if let Ok(services) = locator.ConnectServer(
                &namespace,
                &BSTR::new(),
                &BSTR::new(),
                &BSTR::new(),
                0,
                &BSTR::new(),
                None,
            ) {
                let _ = CoSetProxyBlanket(
                    &services,
                    RPC_C_AUTHN_WINNT,
                    RPC_C_AUTHZ_NONE,
                    None,
                    RPC_C_AUTHN_LEVEL_PKT_PRIVACY,
                    RPC_C_IMP_LEVEL_IMPERSONATE,
                    None,
                    EOAC_NONE,
                );

                let query_lang = BSTR::from("WQL");
                let query_str = BSTR::from("SELECT __RELPATH, InstanceName FROM AcpiTest_MULong");
                if let Ok(enumerator) = services.ExecQuery(
                    &query_lang,
                    &query_str,
                    WBEM_FLAG_FORWARD_ONLY | WBEM_FLAG_RETURN_IMMEDIATELY,
                    None,
                ) {
                    let mut chosen_path: Option<String> = None;
                    let mut fallback_path: Option<String> = None;

                    let mut objects = [None];
                    let mut returned = 0;
                    while enumerator.Next(1000, &mut objects, &mut returned).is_ok() && returned > 0 {
                        if let Some(obj) = objects[0].take() {
                            let mut inst_val = VARIANT::default();
                            let _ = obj.Get(w!("InstanceName"), 0, &mut inst_val, None, None);
                            let inst_str = inst_val.to_string();

                            let mut rel_val = VARIANT::default();
                            let _ = obj.Get(w!("__RELPATH"), 0, &mut rel_val, None, None);
                            let rel_str = rel_val.to_string();

                            let candidate = if !rel_str.is_empty() {
                                rel_str
                            } else if !inst_str.is_empty() {
                                let escaped = inst_str.replace('\\', "\\\\");
                                format!("AcpiTest_MULong.InstanceName=\"{}\"", escaped)
                            } else {
                                String::new()
                            };

                            if !candidate.is_empty() {
                                if inst_str.contains("1_1") {
                                    chosen_path = Some(candidate);
                                    break;
                                } else if fallback_path.is_none() {
                                    fallback_path = Some(candidate);
                                }
                            }
                        }
                    }

                    let target_inst = chosen_path.or(fallback_path);
                    if let Some(target_path_str) = target_inst {
                        let class_name = BSTR::from("AcpiTest_MULong");
                        let mut class_obj_opt: Option<IWbemClassObject> = None;
                        if services.GetObject(&class_name, WBEM_GENERIC_FLAG_TYPE(0), None, Some(&mut class_obj_opt), None).is_ok() {
                            if let Some(class_obj) = class_obj_opt {
                                let mut in_sig: Option<IWbemClassObject> = None;
                                if class_obj.GetMethod(w!("GetSetULong"), 0, &mut in_sig, std::ptr::null_mut()).is_ok() {
                                    if let Some(in_sig) = in_sig {
                                        instance_path_opt = Some(BSTR::from(&target_path_str));
                                        in_param_class_opt = Some(in_sig);
                                        services_opt = Some(services);
                                        is_available = true;
                                        crate::log_info!("WMI_MULONG", "Successfully connected to native AcpiTest_MULong: {}", target_path_str);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Process message tasks strictly inside this single COM-initialized thread
        while let Ok(task) = rx.recv() {
            match task {
                MulongTask::Probe(resp) => {
                    let _ = resp.send(is_available);
                }
                MulongTask::Read(addr, resp) => {
                    if !is_available {
                        let _ = resp.send(None);
                        continue;
                    }
                    let res = (|| -> Option<u8> {
                        let services = services_opt.as_ref()?;
                        let in_param_class = in_param_class_opt.as_ref()?;
                        let instance_path = instance_path_opt.as_ref()?;

                        let in_params = in_param_class.SpawnInstance(0).ok()?;
                        let req_data: i64 = ((1i64) << 40) | (addr as i64);
                        // Standard CIM_UINT64 handling in WMI prefers BSTR string representation, with VT_I8 fallback
                        let mut var_data = VARIANT::from(BSTR::from(format!("{}", req_data)));
                        if in_params.Put(w!("Data"), 0, &mut var_data, 0).is_err() {
                            var_data = VARIANT::from(req_data);
                            in_params.Put(w!("Data"), 0, &mut var_data, 0).ok()?;
                        }

                        let method_name = BSTR::from("GetSetULong");
                        let mut out_params: Option<IWbemClassObject> = None;
                        services.ExecMethod(
                            instance_path,
                            &method_name,
                            WBEM_GENERIC_FLAG_TYPE(0),
                            None,
                            &in_params,
                            Some(&mut out_params),
                            None,
                        ).ok()?;

                        let out = out_params?;
                        let mut ret_val = VARIANT::default();
                        out.Get(w!("Return"), 0, &mut ret_val, None, None).ok()?;

                        let str_val = ret_val.to_string();
                        let raw_u64 = str_val.parse::<u64>().unwrap_or_else(|_| {
                            if let Some(stripped) = str_val.strip_prefix("0x").or_else(|| str_val.strip_prefix("0X")) {
                                u64::from_str_radix(stripped, 16).unwrap_or(0)
                            } else {
                                0
                            }
                        });
                        Some((raw_u64 & 0xFF) as u8)
                    })();
                    let _ = resp.send(res);
                }
                MulongTask::Write(addr, val, resp) => {
                    if !is_available {
                        let _ = resp.send(false);
                        continue;
                    }
                    let res = (|| -> Option<bool> {
                        let services = services_opt.as_ref()?;
                        let in_param_class = in_param_class_opt.as_ref()?;
                        let instance_path = instance_path_opt.as_ref()?;

                        let in_params = in_param_class.SpawnInstance(0).ok()?;
                        let req_data: i64 = ((val as i64) << 16) | (addr as i64);
                        // Standard CIM_UINT64 handling in WMI prefers BSTR string representation, with VT_I8 fallback
                        let mut var_data = VARIANT::from(BSTR::from(format!("{}", req_data)));
                        if in_params.Put(w!("Data"), 0, &mut var_data, 0).is_err() {
                            var_data = VARIANT::from(req_data);
                            in_params.Put(w!("Data"), 0, &mut var_data, 0).ok()?;
                        }

                        let method_name = BSTR::from("GetSetULong");
                        let hr = services.ExecMethod(
                            instance_path,
                            &method_name,
                            WBEM_GENERIC_FLAG_TYPE(0),
                            None,
                            &in_params,
                            None,
                            None,
                        );
                        Some(hr.is_ok())
                    })().unwrap_or(false);
                    let _ = resp.send(res);
                }
                MulongTask::Raw(raw_val, resp) => {
                    if !is_available {
                        crate::log_warn!("WMI_MULONG", "MulongTask::Raw: AcpiTest_MULong is not available");
                        let _ = resp.send(false);
                        continue;
                    }
                    let res = (|| -> Option<bool> {
                        let services = services_opt.as_ref()?;
                        let in_param_class = in_param_class_opt.as_ref()?;
                        let instance_path = instance_path_opt.as_ref()?;

                        let in_params = in_param_class.SpawnInstance(0).ok()?;

                        // Try string representation first (standard CIM_UINT64 handling in WMI), fallback to VT_I8
                        let mut var_data = VARIANT::from(BSTR::from(format!("{}", raw_val)));
                        if in_params.Put(w!("Data"), 0, &mut var_data, 0).is_err() {
                            var_data = VARIANT::from(raw_val as i64);
                            in_params.Put(w!("Data"), 0, &mut var_data, 0).ok()?;
                        }

                        let method_name = BSTR::from("GetSetULong");
                        let mut out_params: Option<IWbemClassObject> = None;
                        let hr = services.ExecMethod(
                            instance_path,
                            &method_name,
                            WBEM_GENERIC_FLAG_TYPE(0),
                            None,
                            &in_params,
                            Some(&mut out_params),
                            None,
                        );
                        if let Err(e) = hr {
                            crate::log_warn!("WMI_MULONG", "ExecMethod GetSetULong failed on {}: err={:?}", instance_path, e);
                            return Some(false);
                        }
                        crate::log_info!("WMI_MULONG", "ExecMethod GetSetULong succeeded for command 0x{:016X}", raw_val);
                        Some(true)
                    })().unwrap_or(false);
                    let _ = resp.send(res);
                }
                MulongTask::QueryRaw(raw_val, resp) => {
                    if !is_available {
                        crate::log_warn!("WMI_MULONG", "MulongTask::QueryRaw: AcpiTest_MULong is not available");
                        let _ = resp.send(None);
                        continue;
                    }
                    let res = (|| -> Option<u64> {
                        let services = services_opt.as_ref()?;
                        let in_param_class = in_param_class_opt.as_ref()?;
                        let instance_path = instance_path_opt.as_ref()?;

                        let in_params = in_param_class.SpawnInstance(0).ok()?;

                        let mut var_data = VARIANT::from(BSTR::from(format!("{}", raw_val)));
                        if in_params.Put(w!("Data"), 0, &mut var_data, 0).is_err() {
                            var_data = VARIANT::from(raw_val as i64);
                            in_params.Put(w!("Data"), 0, &mut var_data, 0).ok()?;
                        }

                        let method_name = BSTR::from("GetSetULong");
                        let mut out_params: Option<IWbemClassObject> = None;
                        let hr = services.ExecMethod(
                            instance_path,
                            &method_name,
                            WBEM_GENERIC_FLAG_TYPE(0),
                            None,
                            &in_params,
                            Some(&mut out_params),
                            None,
                        );
                        if let Err(e) = hr {
                            crate::log_warn!("WMI_MULONG", "ExecMethod GetSetULong query failed on {}: err={:?}", instance_path, e);
                            return None;
                        }
                        let out = out_params?;
                        let mut ret_val = VARIANT::default();
                        out.Get(w!("Return"), 0, &mut ret_val, None, None).ok()?;
                        let str_val = ret_val.to_string();
                        let raw_u64 = str_val.parse::<u64>().unwrap_or_else(|_| {
                            if let Some(stripped) = str_val.strip_prefix("0x") {
                                u64::from_str_radix(stripped, 16).unwrap_or(0)
                            } else {
                                0
                            }
                        });
                        Some(raw_u64)
                    })();
                    let _ = resp.send(res);
                }
            }
        }

        CoUninitialize();
    }
}

/// Probes whether AcpiTest_MULong is supported on this machine
pub fn probe_mulong_available() -> bool {
    {
        let guard = MULONG_STATE.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((ok, at)) = *guard {
            // 成功：永久有效；失败：只缓存 MULONG_NEGATIVE_TTL，之后允许重试
            if ok || at.elapsed() < MULONG_NEGATIVE_TTL {
                return ok;
            }
        }
    }

    let ok = if let Some(worker) = WORKER.as_ref() {
        let (tx, rx) = channel();
        if worker.tx.send(MulongTask::Probe(tx)).is_ok() {
            rx.recv_timeout(Duration::from_millis(2500)).unwrap_or(false)
        } else {
            false
        }
    } else {
        false
    };

    {
        let mut guard = MULONG_STATE.lock().unwrap_or_else(|e| e.into_inner());
        *guard = Some((ok, Instant::now()));
    }
    ok
}

/// Actively tests if AcpiTest_MULong can actually communicate with the hardware EC.
/// Modern Tongfang models leave a dummy/ghost AcpiTest_MULong WMI class, but GetSetULong fails or returns 0/0xFF.
/// A genuine legacy model (e.g. GK5CP6X / Z7-CT) responds with a valid byte when reading standard project ID.
pub fn verify_mulong_functional() -> bool {
    if !is_mulong_available() {
        return false;
    }
    // Attempt active handshake by reading ADDR_PROJECT_ID via MULong
    if let Some(val) = mulong_read_ec(crate::hal::vault::ec::ADDR_PROJECT_ID) {
        if val != 0 && val != 0xFF {
            crate::log_info!("WMI_MULONG", "Active handshake verified: ProjectID valid via MULong");
            return true;
        }
    }
    crate::log_debug!("WMI_MULONG", "Active handshake failed: AcpiTest_MULong cannot read valid EC data (ghost interface)");
    false
}

/// Returns whether AcpiTest_MULong is available（失败结果可重试，不永久缓存）
pub fn is_mulong_available() -> bool {
    let guard = MULONG_STATE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((ok, _)) = *guard {
        if ok {
            return true;
        }
    }
    drop(guard);
    probe_mulong_available()
}

/// Reads a 1-byte register from EC RAM via native WMI AcpiTest_MULong.GetSetULong
pub fn mulong_read_ec(addr: u16) -> Option<u8> {
    if !is_mulong_available() {
        return None;
    }
    let worker = WORKER.as_ref()?;
    let (tx, rx) = channel();
    worker.tx.send(MulongTask::Read(addr, tx)).ok()?;
    rx.recv_timeout(Duration::from_millis(1500)).ok().flatten()
}

/// Writes a 1-byte register to EC RAM via native WMI AcpiTest_MULong.GetSetULong
pub fn mulong_write_ec(addr: u16, val: u8) -> bool {
    if !is_mulong_available() {
        return false;
    }
    let worker = match WORKER.as_ref() {
        Some(w) => w,
        None => return false,
    };
    let (tx, rx) = channel();
    if worker.tx.send(MulongTask::Write(addr, val, tx)).is_err() {
        return false;
    }
    rx.recv_timeout(Duration::from_millis(1500)).unwrap_or(false)
}

/// Executes raw 64-bit command via native WMI AcpiTest_MULong.GetSetULong
pub fn mulong_exec_raw(raw_val: u64) -> bool {
    if !is_mulong_available() {
        crate::log_warn!("WMI_MULONG", "mulong_exec_raw: is_mulong_available is false");
        return false;
    }
    let worker = match WORKER.as_ref() {
        Some(w) => w,
        None => return false,
    };
    let (tx, rx) = channel();
    if worker.tx.send(MulongTask::Raw(raw_val, tx)).is_err() {
        return false;
    }
    rx.recv_timeout(Duration::from_millis(2500)).unwrap_or(false)
}

/// Executes raw 64-bit query and retrieves return value via native WMI AcpiTest_MULong.GetSetULong
pub fn mulong_query_raw(raw_val: u64) -> Option<u64> {
    if !is_mulong_available() {
        return None;
    }
    let worker = match WORKER.as_ref() {
        Some(w) => w,
        None => return None,
    };
    let (tx, rx) = channel();
    if worker.tx.send(MulongTask::QueryRaw(raw_val, tx)).is_err() {
        return None;
    }
    rx.recv_timeout(Duration::from_millis(2500)).ok().flatten()
}

static HOTSWITCH_SUPPORT_CACHE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

/// Probes whether this Tongfang/Mechrevo machine supports seamless runtime ACPI hot-switch (cmd=3 addr=3)
/// Return value low byte: 85 (0x55) = supported, 170 (0xAA) = unsupported
pub fn probe_hotswitch_support() -> bool {
    *HOTSWITCH_SUPPORT_CACHE.get_or_init(|| {
        // 0x30000000003 = 0x3_0000_0000_03 (WMIGetECIGPUonlySupport)
        if let Some(res) = mulong_query_raw(0x30000000003u64) {
            let low_byte = (res & 0xFF) as u8;
            crate::log_info!("WMI_MULONG", "probe_hotswitch_support: response=0x{:016X}, low_byte={}", res, low_byte);
            low_byte == 85
        } else {
            false
        }
    })
}

/// Retrieves current ACPI iGPU-only active status (cmd=3 addr=2)
/// Return value low byte: 85 (0x55) = iGPU-only active (dGPU silenced/offline), 170 (0xAA) = not active (dGPU online)
pub fn get_acpi_igpu_status() -> Option<u8> {
    // 0x30000000002 = 0x3_0000_0000_02 (WMIGetECDGPUStatusForIGPUonly)
    mulong_query_raw(0x30000000002u64).map(|v| (v & 0xFF) as u8)
}

/// Executes ACPI-WMI command to toggle iGPU-only mode (seamless hot-switch without reboot)
/// - enable=true:  0x3_0000_0000_01 (WMIWriteECIGPUonlyON)
/// - enable=false: 0x3_0000_0000_00 (WMIWriteECIGPUonlyOFF)
pub fn set_acpi_igpu_mode(enable: bool) -> bool {
    let cmd = if enable {
        0x30000000001u64
    } else {
        0x30000000000u64
    };
    mulong_exec_raw(cmd)
}



