use super::vault::{ec, mask};

pub fn apply_ec_option(
    option: &str,
    enable: bool,
    modify: impl FnOnce(u16, u8, bool) -> Option<u8>,
) -> Result<(), String> {
    let (address, bits) = match option {
        "usb_charge" => (ec::ADDR_TRIGGER_BYTE, mask::TRIGGER_USB_CHARGE),
        "ac_recovery" => (ec::ADDR_AC_RECOVERY, mask::AC_RECOVERY_BITMASK),
        _ => return Err("Invalid EC power option".into()),
    };
    modify(address, bits, enable)
        .map(|_| ())
        .ok_or_else(|| format!("Failed to confirm {} hardware state", option))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regression_power_options_use_atomic_bits_and_propagate_transport_failure() {
        for (option, address, bits) in [
            (
                "usb_charge",
                ec::ADDR_TRIGGER_BYTE,
                mask::TRIGGER_USB_CHARGE,
            ),
            (
                "ac_recovery",
                ec::ADDR_AC_RECOVERY,
                mask::AC_RECOVERY_BITMASK,
            ),
        ] {
            for enable in [false, true] {
                for result in [None, Some(0), Some(u8::MAX)] {
                    let applied = apply_ec_option(option, enable, |actual, mask, active| {
                        assert_eq!((actual, mask, active), (address, bits, enable));
                        result
                    });
                    assert_eq!(applied.is_ok(), result.is_some());
                }
            }
        }
    }

    #[test]
    fn regression_unknown_power_options_cannot_reach_ec_io() {
        for option in ["", "USB_CHARGE", "high_perf", "unknown"] {
            assert!(apply_ec_option(option, true, |_, _, _| panic!(
                "Unknown option reached EC I/O"
            ))
            .is_err());
        }
    }
}
