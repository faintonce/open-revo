use super::vault::{fan, mask};

pub fn apply_boost(
    legacy: bool,
    enable: bool,
    write_legacy: impl FnOnce(u8) -> bool,
    modify_mask: impl FnOnce(u8, bool) -> bool,
) -> bool {
    if legacy {
        write_legacy(if enable {
            fan::LEGACY_BOOST_ON
        } else {
            fan::LEGACY_BOOST_OFF
        })
    } else {
        modify_mask(mask::FAN_BOOST_CLAMP_BIT, enable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regression_legacy_boost_keeps_the_existing_control_values_and_reports_write_failure() {
        for (enable, expected) in [(true, 0x50), (false, 0x10)] {
            for success in [false, true] {
                assert_eq!(
                    apply_boost(
                        true,
                        enable,
                        |value| {
                            assert_eq!(value, expected);
                            success
                        },
                        |_, _| panic!("Legacy EC must not use the newer mask protocol")
                    ),
                    success
                );
            }
        }
    }

    #[test]
    fn regression_modern_boost_uses_the_atomic_mask_path_and_reports_transport_failure() {
        for enable in [false, true] {
            for success in [false, true] {
                assert_eq!(
                    apply_boost(
                        false,
                        enable,
                        |_| panic!("Modern EC must not overwrite the entire control byte"),
                        |bits, active| {
                            assert_eq!(bits, mask::FAN_BOOST_CLAMP_BIT);
                            assert_eq!(active, enable);
                            success
                        }
                    ),
                    success
                );
            }
        }
    }
}
