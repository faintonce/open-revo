use super::FanTableData;
use crate::hal::power_control::acknowledged;
use crate::hal::vault::{ec, fan, power};

pub(super) fn apply(
    table: &FanTableData,
    next_generation: bool,
    mut modify: impl FnMut(u16, u8, u8) -> Option<u8>,
    mut latch: impl FnMut(bool) -> bool,
    batch: impl FnOnce(&[(u16, u8)]) -> bool,
) -> Result<(), String> {
    modify(ec::ADDR_AP_OEM_BYTE, u8::MAX, power::AP_EXIST_BIT)
        .ok_or("Fan table APExist update failed")?;
    acknowledged(latch(true), "Fan table pre-latch")?;
    let upload = (|| {
        modify(ec::ADDR_AP_OEM_BYTE, u8::MAX, power::AP_ACTIVE_BITS)
            .ok_or("Fan table AP active update failed")?;
        let mut entries = Vec::with_capacity(fan::TABLE_SLOTS * 5 + fan::GPU_TABLE_WRITE_SLOTS + 3);
        for (base, values, duty) in [
            (ec::ADDR_RAMFAN_CPU_UPT_BASE, &table.cpu_upt, false),
            (ec::ADDR_RAMFAN_CPU_DOWNT_BASE, &table.cpu_downt, false),
            (ec::ADDR_RAMFAN_CPU_DUTY_BASE, &table.cpu_duty, true),
            (ec::ADDR_RAMFAN_GPU_UPT_BASE, &table.gpu_upt, false),
            (ec::ADDR_RAMFAN_GPU_DOWNT_BASE, &table.gpu_downt, false),
        ] {
            for (index, &value) in values.iter().enumerate() {
                entries.push((
                    base + index as u16,
                    if duty && index == 0 { 0 } else { value },
                ));
            }
        }
        for (index, &value) in table
            .gpu_duty
            .iter()
            .take(fan::GPU_TABLE_WRITE_SLOTS)
            .enumerate()
        {
            entries.push((
                ec::ADDR_RAMFAN_GPU_DUTY_BASE + index as u16,
                if index == 0 { 0 } else { value },
            ));
        }
        let maximum = table.gpu_duty[fan::TABLE_SLOTS - 1];
        entries.extend([
            (ec::ADDR_RAMFAN_GPU_MAX_THRESHOLD1, maximum),
            (ec::ADDR_RAMFAN_GPU_MAX_THRESHOLD2, maximum),
            (
                ec::ADDR_RAMFAN_TABLE_CTRL,
                maximum.max(fan::TABLE_CONTROL_MIN),
            ),
        ]);
        acknowledged(batch(&entries), "Fan table upload")
    })();
    // Keep the existing post-latch finalization after an attempted upload, including failed writes.
    let commit = acknowledged(latch(false), "Fan table commit");
    match (upload, commit) {
        (Err(upload), Err(commit)) => return Err(format!("{upload}; {commit}")),
        (Err(error), _) | (_, Err(error)) => return Err(error),
        _ => {}
    }
    if !next_generation {
        modify(
            ec::ADDR_TRIGGER_BYTE,
            power::TRIGGER_KEEP_MASK,
            power::TRIGGER_PERFORMANCE,
        )
        .ok_or("Fan table trigger update failed")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn regression_fan_table_keeps_complete_frame_layout_and_zero_rpm_entries() {
        let mut table = super::super::FAN_TABLE_OFFICE.clone();
        table.cpu_duty[0] = u8::MAX;
        table.gpu_duty[0] = u8::MAX;
        table.gpu_duty[fan::TABLE_SLOTS - 1] = 0;
        let events = RefCell::new(Vec::new());
        apply(
            &table,
            false,
            |address, preserve, value| {
                events.borrow_mut().push("bits");
                if address == ec::ADDR_TRIGGER_BYTE {
                    assert_eq!(
                        (preserve, value),
                        (power::TRIGGER_KEEP_MASK, power::TRIGGER_PERFORMANCE)
                    );
                }
                Some(value)
            },
            |pre| {
                events.borrow_mut().push(if pre { "pre" } else { "commit" });
                true
            },
            |entries| {
                events.borrow_mut().push("upload");
                assert_eq!(entries.len(), 96);
                for (base, values) in [
                    (ec::ADDR_RAMFAN_CPU_UPT_BASE, &table.cpu_upt),
                    (ec::ADDR_RAMFAN_CPU_DOWNT_BASE, &table.cpu_downt),
                    (ec::ADDR_RAMFAN_GPU_UPT_BASE, &table.gpu_upt),
                    (ec::ADDR_RAMFAN_GPU_DOWNT_BASE, &table.gpu_downt),
                ] {
                    for (index, &value) in values.iter().enumerate() {
                        assert!(entries.contains(&(base + index as u16, value)));
                    }
                }
                assert!(entries.contains(&(ec::ADDR_RAMFAN_CPU_DUTY_BASE, 0)));
                assert!(entries.contains(&(ec::ADDR_RAMFAN_GPU_DUTY_BASE, 0)));
                assert!(entries.contains(&(ec::ADDR_RAMFAN_TABLE_CTRL, fan::TABLE_CONTROL_MIN)));
                true
            },
        )
        .unwrap();
        assert_eq!(
            events.into_inner(),
            ["bits", "pre", "bits", "upload", "commit", "bits"]
        );
    }

    #[test]
    fn regression_fan_table_reports_every_failure_and_retains_post_latch_finalization() {
        for failed in 0..6 {
            let events = RefCell::new(Vec::new());
            let next = |label| {
                let mut events = events.borrow_mut();
                events.push(label);
                events.len() != failed + 1
            };
            let result = apply(
                &super::super::FAN_TABLE_OFFICE,
                false,
                |_, _, _| next("bits").then_some(0),
                |pre| next(if pre { "pre" } else { "commit" }),
                |_| next("upload"),
            );
            assert!(result.is_err());
            let events = events.into_inner();
            if failed == 2 || failed == 3 {
                assert_eq!(events.last(), Some(&"commit"));
            }
        }
    }

    #[test]
    fn regression_next_generation_fan_table_does_not_send_the_legacy_trigger() {
        apply(
            &super::super::FAN_TABLE_OFFICE,
            true,
            |address, _, value| {
                assert_ne!(address, ec::ADDR_TRIGGER_BYTE);
                Some(value)
            },
            |_| true,
            |_| true,
        )
        .unwrap();
    }
}
