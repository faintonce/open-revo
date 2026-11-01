use super::*;
use std::cell::RefCell;

#[test]
fn regression_invalid_battery_limit_cannot_change_cache_or_call_hardware() {
    let bridge = EcBridge::dummy();
    let cooldown = bridge.cache.lock().battery_limit_cooldown_until;
    for limit in 0..=u8::MAX {
        if (60..=100).contains(&limit) {
            continue;
        }
        assert!(bridge
            .apply_battery_control(
                limit,
                || panic!("Invalid battery limit reached hardware"),
                || panic!("Invalid battery limit reached settings")
            )
            .is_err());
        assert_eq!(bridge.get_battery_limit(), 80);
        assert_eq!(bridge.cache.lock().battery_limit_cooldown_until, cooldown);
    }
}

#[test]
fn regression_failed_battery_write_keeps_the_confirmed_limit_and_skips_persistence() {
    let bridge = EcBridge::dummy();
    let result = bridge.apply_battery_control(
        60,
        || {
            assert_eq!(bridge.get_battery_limit(), 80);
            assert!(bridge.cache.lock().battery_limit_cooldown_until > Instant::now());
            bridge.cache.lock().battery_limit_cooldown_until =
                Instant::now() - Duration::from_secs(1);
            Err("write acknowledgement failed".into())
        },
        || panic!("A failed hardware write must not save a successful setting"),
    );
    assert_eq!(result.unwrap_err(), "write acknowledgement failed");
    assert_eq!(bridge.get_battery_limit(), 80);
    assert!(bridge.cache.lock().battery_limit_cooldown_until > Instant::now());
}

#[test]
fn regression_battery_settings_commit_after_hardware_and_report_save_failures() {
    for saved in [false, true] {
        let bridge = EcBridge::dummy();
        let calls = RefCell::new(Vec::new());
        let result = bridge.apply_battery_control(
            60,
            || {
                assert_eq!(bridge.get_battery_limit(), 80);
                calls.borrow_mut().push("hardware");
                Ok(())
            },
            || {
                assert_eq!(bridge.get_battery_limit(), 60);
                calls.borrow_mut().push("settings");
                if saved {
                    Ok(())
                } else {
                    Err("settings failed".into())
                }
            },
        );
        assert_eq!(*calls.borrow(), ["hardware", "settings"]);
        assert_eq!(bridge.get_battery_limit(), 60);
        assert_eq!(result.is_ok(), saved);
        if !saved {
            assert_eq!(result.unwrap_err(), "settings failed");
        }
    }
}
