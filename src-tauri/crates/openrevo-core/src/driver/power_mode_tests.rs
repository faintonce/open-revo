use super::*;
use std::cell::Cell;

#[test]
fn regression_mode_failure_preserves_cache_and_does_not_save_settings() {
    let bridge = EcBridge::dummy();
    let result = bridge.apply_power_mode(
        3,
        || {
            assert_eq!(bridge.get_power_mode(), 2);
            assert!(bridge.cache.lock().power_mode_cooldown_until > Instant::now());
            bridge.cache.lock().power_mode_cooldown_until = Instant::now() - Duration::from_secs(1);
            Err("hardware failed".into())
        },
        || panic!("A failed mode must not be saved"),
    );
    assert_eq!(result.unwrap_err(), "hardware failed");
    assert_eq!(bridge.get_power_mode(), 2);
    assert!(bridge.cache.lock().power_mode_cooldown_until > Instant::now());
}

#[test]
fn regression_mode_cache_commits_after_ack_and_survives_persistence_failure() {
    let bridge = EcBridge::dummy();
    let applied = Cell::new(false);
    let result = bridge.apply_power_mode(
        4,
        || {
            assert_eq!(bridge.get_power_mode(), 2);
            applied.set(true);
            Ok(())
        },
        || {
            assert!(applied.get());
            assert_eq!(bridge.get_power_mode(), 4);
            Err("disk failed".into())
        },
    );
    assert!(result.unwrap_err().contains("disk failed"));
    assert_eq!(bridge.get_power_mode(), 4);
    assert!(bridge
        .apply_power_mode(
            0,
            || panic!("Invalid mode reached hardware"),
            || panic!("Invalid mode saved")
        )
        .is_err());
}

#[test]
fn regression_observed_hardware_mode_remains_truthful_when_os_synchronization_fails() {
    let bridge = EcBridge::dummy();
    let result = bridge.align_hardware_mode(3, || {
        assert_eq!(bridge.get_power_mode(), 3);
        Err("overlay failed".into())
    });
    assert_eq!(result.unwrap_err(), "overlay failed");
    assert_eq!(bridge.get_power_mode(), 3);
    assert!(bridge
        .align_hardware_mode(0, || panic!("Invalid observation synchronized"))
        .is_err());
}
