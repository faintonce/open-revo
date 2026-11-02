// shortcut: These are existing parameter guards, not a per-model register allowlist.
pub fn validate_read_address(addr: u16) -> Result<(), String> {
    if addr > 0x0FFF {
        return Err(format!("Address out of physical bounds: 0x{:04X} exceeds 0x0FFF", addr));
    }
    Ok(())
}

pub fn validate_write_operation(addr: u16, val: u8) -> Result<(), String> {
    validate_read_address(addr)?;
    if addr == 0x0000 && val == 0xFF {
        return Err("Operation blocked by Hardware Blast Shield: Reserved reset vector".into());
    }
    Ok(())
}

pub fn masked_write_value(addr: u16, current: u8, mask: u8, enabled: bool) -> Result<u8, String> {
    let next = if enabled { current | mask } else { current & !mask };
    validate_write_operation(addr, next)?;
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regression_masked_write_checks_the_final_value() {
        assert!(validate_write_operation(0, 0).is_ok());
        assert!(masked_write_value(0, 0, 0xFF, true).is_err());
        assert!(masked_write_value(0x1000, 0, 1, true).is_err());
        assert_eq!(masked_write_value(0, 0xFF, 0xF0, false).unwrap(), 0x0F);
    }
}
