use super::*;

fn sample(at_ms: u64, load: f64, power: f64, temperature: f64) -> CpuSample {
    CpuSample {
        at_ms,
        load: Some(load),
        power: Some(power),
        temperature: Some(temperature),
        source: PowerSource::Measured,
        fan_rpm: Some(3000.0),
        gpu_power: Some(20.0),
    }
}

fn history(load: f64, power: f64, temperature: f64) -> PowerHistory {
    let mut h = PowerHistory::default();
    for i in 0..=10 {
        h.record(sample(35_000 + i * 1500, load, power, temperature));
    }
    h
}

fn request(pl1: u16) -> PowerChangeRequest {
    PowerChangeRequest {
        confirmed: false,
        context: "profile:4:ac".into(),
        current: CpuPowerLimits {
            pl1: 67,
            pl2: 91,
            pl4: 150,
        },
        proposed: CpuPowerLimits {
            pl1,
            pl2: pl1.max(91),
            pl4: (pl1 + 20).max(150),
        },
        factory: CpuPowerLimits {
            pl1: 115,
            pl2: 125,
            pl4: 190,
        },
        factory_verified: true,
        office_pl1: 45,
        minimum_pl1: 15,
        margin: 20,
        temperature_limit_c: 95.0,
    }
}

fn assess(r: &PowerChangeRequest, h: &PowerHistory) -> PowerAssessment {
    PowerSafetySession::default()
        .assess(r, h.snapshot(50_000), 50_000)
        .unwrap()
}

#[test]
fn empty_statistics_do_not_invent_sensor_defaults() {
    let s = PowerHistory::default().snapshot(50_000);
    assert!(!s.ready);
    assert_eq!(s.sample_count, 0);
    assert_eq!(s.temperature_c, None);
    assert_eq!(s.power_source, PowerSource::Unavailable);
}

#[test]
fn recent_statistics_have_freshness_and_measured_provenance() {
    let s = history(80.0, 60.0, 70.0).snapshot(50_000);
    assert!(s.ready);
    assert_eq!(s.sample_count, 11);
    assert_eq!(s.span_ms, 15_000);
    assert_eq!(s.power_p95_w, Some(60.0));
    assert_eq!(s.power_source, PowerSource::Measured);
    assert_eq!(s.temperature_slope_c_s, Some(0.0));
}

#[test]
fn stale_samples_cannot_pass_as_current_readings() {
    let s = history(80.0, 60.0, 70.0).snapshot(56_000);
    assert!(!s.ready);
    assert_eq!(s.power_source, PowerSource::Unavailable);
    assert_eq!(s.power_p95_w, None);
    assert_eq!(s.temperature_c, None);
}

#[test]
fn resume_gap_and_backwards_clock_reset_the_window() {
    let mut h = history(80.0, 60.0, 70.0);
    h.record(sample(56_000, 20.0, 15.0, 50.0));
    assert_eq!(h.snapshot(56_000).sample_count, 1);
    h.record(sample(55_000, 20.0, 15.0, 50.0));
    assert_eq!(h.snapshot(55_000).sample_count, 1);
}

#[test]
fn invalid_sensor_values_are_not_zero_or_real_measurements() {
    let mut h = history(80.0, 60.0, 70.0);
    h.record(sample(51_500, f64::NAN, f64::INFINITY, 0.0));
    let s = h.snapshot(51_500);
    assert!(!s.ready);
    assert_eq!(s.power_source, PowerSource::Unavailable);
}

#[test]
fn estimated_series_never_becomes_measured() {
    let mut h = PowerHistory::default();
    for i in 0..=10 {
        let mut s = sample(35_000 + i * 1500, 85.0, 60.0, 70.0);
        s.source = PowerSource::Estimated;
        h.record(s);
    }
    assert_eq!(h.snapshot(50_000).power_source, PowerSource::Estimated);
    assert!(!assess(&request(50), &h).requires_confirmation);
    assert!(assess(&request(40), &h).requires_confirmation);
}

#[test]
fn short_window_is_bounded_and_old_history_expires() {
    let mut h = PowerHistory::default();
    for i in 0..500 {
        h.record(sample(i * 1500, 30.0, 30.0, 60.0));
    }
    assert!(h.samples.len() <= 81);
    assert!(h.samples.back().unwrap().at_ms - h.samples.front().unwrap().at_ms <= HISTORY_MS);
}

#[test]
fn ordinary_adjustments_do_not_prompt_even_at_high_load() {
    for power in [62, 65, 66, 68, 70, 72] {
        assert!(
            !assess(&request(power), &history(90.0, 62.0, 70.0)).requires_confirmation,
            "{power}W"
        );
    }
}

#[test]
fn same_reduction_is_safe_at_idle_and_risky_under_load() {
    assert!(!assess(&request(40), &history(5.0, 8.0, 45.0)).requires_confirmation);
    assert!(assess(&request(40), &history(85.0, 62.0, 75.0)).requires_confirmation);
}

#[test]
fn responsiveness_floor_still_protects_idle_machine() {
    assert!(assess(&request(15), &history(5.0, 8.0, 45.0)).requires_confirmation);
}

#[test]
fn increasing_from_an_existing_low_limit_is_not_blocked() {
    let mut r = request(20);
    r.current.pl1 = 15;
    assert!(!assess(&r, &history(5.0, 8.0, 45.0)).requires_confirmation);
}

#[test]
fn increasing_power_only_warns_when_thermal_prediction_is_near_limit() {
    assert!(!assess(&request(75), &history(85.0, 62.0, 65.0)).requires_confirmation);
    let hot = assess(&request(75), &history(85.0, 62.0, 92.0));
    assert!(hot.requires_confirmation);
    assert!(hot.predicted_temperature_c.unwrap() >= 92.0);
}

#[test]
fn burst_increase_also_participates_in_thermal_check() {
    let mut r = request(67);
    r.proposed.pl2 = 125;
    assert!(assess(&r, &history(85.0, 62.0, 92.0)).requires_confirmation);
}

#[test]
fn rising_temperature_increases_predicted_risk() {
    let mut h = history(85.0, 62.0, 85.0);
    for (index, entry) in h.samples.iter_mut().enumerate() {
        entry.temperature = Some(75.0 + index as f64);
    }
    assert!(assess(&request(80), &h).requires_confirmation);
}

#[test]
fn unverified_above_factory_increase_is_high_risk_even_while_idle() {
    assert!(assess(&request(165), &history(5.0, 8.0, 45.0)).requires_confirmation);
    let mut unknown = request(165);
    unknown.factory_verified = false;
    assert!(assess(&unknown, &history(5.0, 8.0, 45.0)).requires_confirmation);
}

#[test]
fn cold_start_preserves_small_adjustments_and_checks_large_drops() {
    let h = PowerHistory::default();
    assert!(!assess(&request(65), &h).requires_confirmation);
    assert!(assess(&request(30), &h).requires_confirmation);
}

#[test]
fn successive_small_steps_are_compared_to_stable_baseline() {
    let s = history(85.0, 62.0, 70.0).snapshot(50_000);
    let mut session = PowerSafetySession::default();
    let mut r = request(62);
    assert!(
        !session
            .assess(&r, s.clone(), 50_000)
            .unwrap()
            .requires_confirmation
    );
    r.current.pl1 = 62;
    r.proposed.pl1 = 57;
    assert!(
        !session
            .assess(&r, s.clone(), 51_000)
            .unwrap()
            .requires_confirmation
    );
    r.current.pl1 = 57;
    r.proposed.pl1 = 52;
    assert!(
        !session
            .assess(&r, s.clone(), 52_000)
            .unwrap()
            .requires_confirmation
    );
    r.current.pl1 = 52;
    r.proposed.pl1 = 47;
    assert!(session.assess(&r, s, 53_000).unwrap().requires_confirmation);
}

#[test]
fn stable_healthy_operation_can_establish_a_new_baseline() {
    let s = history(85.0, 62.0, 70.0).snapshot(50_000);
    let mut session = PowerSafetySession::default();
    let mut r = request(40);
    session.observe(&r.context, r.current, &s, 95.0, 1_000);
    r.current.pl1 = 50;
    session.observe(&r.context, r.current, &s, 95.0, 2_000);
    assert!(
        session
            .assess(&r, s.clone(), 3_000)
            .unwrap()
            .requires_confirmation
    );
    session.observe(&r.context, r.current, &s, 95.0, 33_000);
    assert!(!session.assess(&r, s, 34_000).unwrap().requires_confirmation);
}

#[test]
fn accepted_risk_band_does_not_prompt_again_for_one_watt() {
    let s = history(5.0, 8.0, 45.0).snapshot(50_000);
    let mut session = PowerSafetySession::default();
    let mut r = request(165);
    assert!(
        session
            .assess(&r, s.clone(), 50_000)
            .unwrap()
            .requires_confirmation
    );
    r.confirmed = true;
    session.assess(&r, s.clone(), 50_100).unwrap();
    r.confirmed = false;
    r.current = r.proposed;
    r.proposed = CpuPowerLimits {
        pl1: 166,
        pl2: 166,
        pl4: 186,
    };
    assert!(!session.assess(&r, s, 50_200).unwrap().requires_confirmation);
}

#[test]
fn stale_confirmation_context_is_rejected() {
    let s = history(5.0, 8.0, 45.0).snapshot(50_000);
    let mut session = PowerSafetySession::default();
    let mut r = request(165);
    session.assess(&r, s.clone(), 50_000).unwrap();
    r.context = "other-profile".into();
    r.confirmed = true;
    assert!(session.assess(&r, s, 50_100).is_err());
}

#[test]
fn repeated_one_watt_increases_cannot_bypass_thermal_protection() {
    let s = history(85.0, 62.0, 92.0).snapshot(50_000);
    let mut session = PowerSafetySession::default();
    let mut r = request(68);
    for target in 68..72 {
        r.proposed.pl1 = target;
        assert!(
            !session
                .assess(&r, s.clone(), 50_000 + target as u64)
                .unwrap()
                .requires_confirmation
        );
        r.current.pl1 = target;
    }
    r.proposed.pl1 = 72;
    assert!(session.assess(&r, s, 50_100).unwrap().requires_confirmation);
}

#[test]
fn invalid_linked_limits_are_rejected() {
    let mut r = request(60);
    r.proposed.pl4 = 80;
    assert!(PowerSafetySession::default()
        .assess(&r, PowerSnapshot::default(), 1_000)
        .is_err());
}

#[test]
fn thermal_learning_requires_comparable_measured_conditions() {
    let mut h = PowerHistory::default();
    for i in 0..=20 {
        h.record(sample(
            i * 1500,
            80.0,
            30.0 + i as f64 * 2.0,
            45.0 + i as f64,
        ));
    }
    assert!((h.snapshot(30_000).thermal_gain_c_w.unwrap() - 0.5).abs() < 0.001);
    for s in &mut h.samples {
        s.gpu_power = None;
    }
    assert_eq!(h.snapshot(30_000).thermal_gain_c_w, None);
}
