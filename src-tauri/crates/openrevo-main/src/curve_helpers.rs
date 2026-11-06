pub fn apply_monotonic_fan_rule(curve: &mut [i32], idx: usize, target_duty: i32, min_duty: i32) {
    if idx >= curve.len() {
        return;
    }
    let target = target_duty.clamp(min_duty, 100);
    curve[idx] = target;

    // Monotonic Rule 1: Points after current index cannot have lower duty (push higher)
    for j in (idx + 1)..curve.len() {
        if curve[j] < target {
            curve[j] = target;
        }
    }

    // Monotonic Rule 2: Points before current index cannot have higher duty (pull lower)
    for j in 0..idx {
        if curve[j] > target {
            curve[j] = target.max(min_duty);
        }
    }
}

