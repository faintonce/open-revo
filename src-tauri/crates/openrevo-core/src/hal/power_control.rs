use super::vault::{bld_power, power};

// Serialize mode decisions through hardware acknowledgement and persistence.
pub static MODE_OPERATION: parking_lot::ReentrantMutex<()> = parking_lot::ReentrantMutex::new(());

#[cfg(test)]
mod ordering_tests {
    use super::*;

    #[test]
    fn regression_mode_pipeline_waits_for_the_active_mode_decision() {
        let decision = MODE_OPERATION.lock();
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (applied_tx, applied_rx) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            started_tx.send(()).unwrap();
            apply_mode_steps(
                || { applied_tx.send(()).unwrap(); Ok(()) },
                || Ok(()), || Ok(()), || Ok(()),
            ).unwrap();
        });
        started_rx.recv_timeout(std::time::Duration::from_secs(2)).unwrap();
        assert!(applied_rx.recv_timeout(std::time::Duration::from_millis(50)).is_err());
        // Nested mode entry points on the same thread must remain usable.
        let nested = MODE_OPERATION.try_lock().unwrap();
        drop(nested);
        drop(decision);
        applied_rx.recv_timeout(std::time::Duration::from_secs(2)).unwrap();
        worker.join().unwrap();
    }
}

pub fn acknowledged(success: bool, operation: &str) -> Result<(), String> {
    if success {
        Ok(())
    } else {
        Err(format!("{operation} was not acknowledged"))
    }
}

#[derive(Debug)]
pub struct ModeApplyError {
    pub message: String,
    pub hardware_confirmed: bool,
}

pub fn apply_mode_steps(
    validate: impl FnOnce() -> Result<(), String>,
    overlay: impl FnOnce() -> Result<(), String>,
    hardware: impl FnOnce() -> Result<(), String>,
    synchronize: impl FnOnce() -> Result<(), String>,
) -> Result<(), ModeApplyError> {
    let _transition = MODE_OPERATION.lock();
    validate().map_err(|message| ModeApplyError {
        message,
        hardware_confirmed: false,
    })?;
    (|| {
        overlay()?;
        hardware()
    })()
    .map_err(|error| ModeApplyError {
        message: format!("性能模式切换未完成，部分设置可能已经生效：{error}"),
        hardware_confirmed: false,
    })?;
    synchronize().map_err(|error| ModeApplyError {
        message: format!("性能模式已切换，但 OEM 配置同步失败：{error}"),
        hardware_confirmed: true,
    })
}

pub fn mode_index(current: u8, target: u8, logo_supported: bool) -> u8 {
    let support = if logo_supported {
        power::LOGO_SUPPORT_BIT
    } else {
        current & power::LOGO_SUPPORT_BIT
    };
    (current & power::MODE_BRIGHTNESS_MASK) | (target & power::MODE_BITS_MASK) | support
}

pub fn trigger(current: u8, office: bool) -> u8 {
    (current & power::TRIGGER_KEEP_MASK)
        | if office {
            power::TRIGGER_OFFICE
        } else {
            power::TRIGGER_PERFORMANCE
        }
}

pub fn pulse_fan_latch(mut write: impl FnMut(u8) -> bool) -> bool {
    let asserted = write(super::vault::mask::FAN_TABLE_LATCH_BIT);
    let released = write(0);
    asserted && released
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regression_mode_transition_reports_every_failure_without_running_later_steps() {
        for failed in 0..4 {
            let calls = std::cell::Cell::new(0);
            let next = || {
                let current = calls.get();
                calls.set(current + 1);
                if current == failed {
                    Err("rejected".into())
                } else {
                    Ok(())
                }
            };
            let error = apply_mode_steps(next, next, next, next).unwrap_err();
            assert!(error.message.contains("rejected"));
            assert_eq!(error.hardware_confirmed, failed == 3);
            assert_eq!(calls.get(), failed + 1);
        }
        let calls = std::cell::Cell::new(0);
        for _ in 0..2 {
            apply_mode_steps(
                || Ok(()),
                || Ok(()),
                || {
                    calls.set(calls.get() + 1);
                    Ok(())
                },
                || Ok(()),
            )
            .unwrap();
        }
        assert_eq!(calls.get(), 2);
    }

    #[test]
    fn regression_mode_bits_keep_existing_brightness_logo_and_usb_semantics() {
        for current in u8::MIN..=u8::MAX {
            for target in [power::MODE_BALANCED, power::MODE_TURBO] {
                for logo in [false, true] {
                    let value = mode_index(current, target, logo);
                    assert_eq!(
                        value & power::MODE_BRIGHTNESS_MASK,
                        current & power::MODE_BRIGHTNESS_MASK
                    );
                    assert_eq!(value & power::MODE_BITS_MASK, target);
                    assert_eq!(
                        value & power::LOGO_SUPPORT_BIT,
                        if logo {
                            power::LOGO_SUPPORT_BIT
                        } else {
                            current & power::LOGO_SUPPORT_BIT
                        }
                    );
                }
            }
            for office in [false, true] {
                let value = trigger(current, office);
                assert_eq!(
                    value & power::TRIGGER_KEEP_MASK,
                    current & power::TRIGGER_KEEP_MASK
                );
                assert_eq!(
                    value & !power::TRIGGER_KEEP_MASK,
                    if office {
                        power::TRIGGER_OFFICE
                    } else {
                        power::TRIGGER_PERFORMANCE
                    }
                );
            }
        }
        assert!(acknowledged(true, "control").is_ok());
        assert!(acknowledged(false, "control")
            .unwrap_err()
            .contains("control"));
    }

    #[test]
    fn regression_fan_latch_release_is_attempted_and_both_write_results_are_required() {
        for failed in 0..2 {
            let mut writes = Vec::new();
            assert!(!pulse_fan_latch(|value| {
                writes.push(value);
                writes.len() != failed + 1
            }));
            assert_eq!(writes, [super::super::vault::mask::FAN_TABLE_LATCH_BIT, 0]);
        }
    }
}


pub fn bld_cpu_profile(
    pl1: u16,
    pl2: u16,
    temperature: u8,
    temperature_enabled: bool,
    send: impl FnMut(&[u8]) -> Result<Vec<u8>, String>,
) -> Result<(), String> {
    bld_cpu_tweak(
        Some(pl1),
        Some(pl2),
        temperature_enabled.then_some(temperature),
        send,
    )
}

pub fn bld_cpu_tweak(
    pl1: Option<u16>,
    pl2: Option<u16>,
    temperature: Option<u8>,
    mut send: impl FnMut(&[u8]) -> Result<Vec<u8>, String>,
) -> Result<(), String> {
    for (mut frame, value) in [
        (
            bld_power::CPU_PL1_FRAME,
            pl1.map(|v| v.clamp(25, 120) as u8),
        ),
        (
            bld_power::CPU_PL2_FRAME,
            pl2.map(|v| v.clamp(25, 140) as u8),
        ),
        (
            bld_power::CPU_TEMP_FRAME,
            temperature.map(|v| v.clamp(70, 100)),
        ),
    ] {
        if let Some(value) = value {
            frame[5] = value;
            send(&frame)?;
        }
    }
    Ok(())
}


#[cfg(test)]
mod bld_tests {
    use super::*;

    #[test]
    fn regression_bld_profile_clamps_before_byte_conversion_and_propagates_each_failure() {
        let mut frames = Vec::new();
        bld_cpu_profile(u16::MAX, u16::MAX, u8::MAX, true, |bytes| {
            frames.push(bytes.to_vec());
            Ok(Vec::new())
        })
        .unwrap();
        assert_eq!(
            frames.iter().map(|frame| frame[5]).collect::<Vec<_>>(),
            [120, 140, 100]
        );
        for failed in 0..3 {
            let mut calls = 0;
            assert!(bld_cpu_profile(75, 90, 95, true, |_| {
                calls += 1;
                if calls == failed + 1 {
                    Err("mailbox failed".into())
                } else {
                    Ok(Vec::new())
                }
            })
            .is_err());
            assert_eq!(calls, failed + 1);
        }
        let mut calls = 0;
        bld_cpu_profile(75, 90, 95, false, |_| {
            calls += 1;
            Ok(Vec::new())
        })
        .unwrap();
        assert_eq!(calls, 2);
    }
}
