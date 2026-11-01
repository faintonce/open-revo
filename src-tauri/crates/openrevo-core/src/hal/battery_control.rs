use super::vault::{ec, mask};

pub fn validate_limit(limit: u8) -> Result<(), String> {
    if (60..=100).contains(&limit) {
        Ok(())
    } else {
        Err("Battery charge limit must be between 60 and 100 percent".into())
    }
}

pub fn mode_name(limit: u8) -> &'static str {
    if limit >= 95 {
        "long_life"
    } else if limit <= 65 {
        "station"
    } else {
        "balance"
    }
}

pub fn mode_limit(mode: &str) -> Result<u8, String> {
    match mode {
        "long_life" => Ok(100),
        "balance" => Ok(80),
        "station" => Ok(60),
        _ => Err("Unknown battery protection mode".into()),
    }
}

pub fn write_thresholds(
    limit: u8,
    mut write: impl FnMut(u16, u8) -> bool,
    modify_mode: impl FnOnce(u8) -> Option<u8>,
) -> Result<u8, String> {
    validate_limit(limit)?;
    let mode = if limit >= 95 {
        mask::BATTERY_MODE_LONGLIFE
    } else if limit <= 65 {
        mask::BATTERY_MODE_STATION
    } else {
        mask::BATTERY_MODE_BALANCE
    };
    let upper = write(ec::ADDR_BATTERY_LIMIT_UP, limit);
    let lower = write(ec::ADDR_BATTERY_LIMIT_DOWN, limit - 5);
    let mode = modify_mode(mode);
    match mode {
        Some(mode) if upper && lower => Ok(mode),
        _ => Err(
            "Failed to confirm all battery protection writes; read back the hardware state".into(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regression_battery_limit_rejects_invalid_values_before_any_io() {
        for limit in 0..=u8::MAX {
            assert_eq!(validate_limit(limit).is_ok(), (60..=100).contains(&limit));
            if !(60..=100).contains(&limit) {
                assert!(write_thresholds(
                    limit,
                    |_, _| panic!("Invalid limits must not write hardware"),
                    |_| panic!("Invalid limits must not change protection mode")
                )
                .is_err());
            }
        }
        for (name, limit) in [("station", 60), ("balance", 80), ("long_life", 100)] {
            assert_eq!(mode_limit(name).unwrap(), limit);
            assert_eq!(mode_name(limit), name);
        }
        for name in ["", "balanced", "STATION", "unknown"] {
            assert!(mode_limit(name).is_err());
        }
    }

    #[test]
    fn regression_battery_thresholds_preserve_values_and_report_each_failed_step() {
        for (limit, expected_mode) in [(60, 0x21), (70, 0x11), (80, 0x11), (100, 0x01)] {
            for outcomes in 0..8 {
                let mut writes = Vec::new();
                let result = write_thresholds(
                    limit,
                    |address, value| {
                        writes.push((address, value));
                        outcomes & (1 << (writes.len() - 1)) != 0
                    },
                    |mode| {
                        assert_eq!(mode, expected_mode);
                        (outcomes & 4 != 0).then_some(mode | mask::TOUCHPAD_BITMASK)
                    },
                );
                assert_eq!(
                    writes,
                    [
                        (ec::ADDR_BATTERY_LIMIT_UP, limit),
                        (ec::ADDR_BATTERY_LIMIT_DOWN, limit - 5)
                    ]
                );
                assert_eq!(result.is_ok(), outcomes == 7);
                if let Ok(mode) = result {
                    assert_eq!(mode, expected_mode | mask::TOUCHPAD_BITMASK);
                }
            }
        }
    }
}
