pub fn validate_power_mode(mode: u8, thin_and_light: bool, dc_online: bool, unlocked: bool) -> Result<(), String> {
    if !(1..=4).contains(&mode) {
        return Err("Invalid power mode".into());
    }
    if !thin_and_light && mode >= 3 && !dc_online && !unlocked {
        return Err("在电池或 Type-C 供电下，狂暴模式与自定义模式不可用。请连接原装电源适配器，或在系统设置中解除离电限制。".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regression_all_power_entry_points_share_battery_policy() {
        for mode in 1..=4 {
            assert_eq!(validate_power_mode(mode, false, false, false).is_ok(), mode <= 2);
            assert!(validate_power_mode(mode, false, true, false).is_ok());
            assert!(validate_power_mode(mode, false, false, true).is_ok());
            assert!(validate_power_mode(mode, true, false, false).is_ok());
        }
        assert!(validate_power_mode(0, true, true, true).is_err());
        assert!(validate_power_mode(5, true, true, true).is_err());
    }
}
