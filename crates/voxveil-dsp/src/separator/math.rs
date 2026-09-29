/// Cubic smoothstep of an already-normalised value.
pub fn smoothstep(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    value * value * (3.0 - 2.0 * value)
}

/// Smooth ramp from 0 at `low` to 1 at `high`.
pub fn ramp(value: f32, low: f32, high: f32) -> f32 {
    if high <= low {
        return if value >= low { 1.0 } else { 0.0 };
    }
    smoothstep((value - low) / (high - low))
}

/// One-pole smoothing coefficient for a time constant, applied once per hop.
pub fn one_pole_coeff(hop_seconds: f32, time_seconds: f32) -> f32 {
    1.0 - (-hop_seconds / time_seconds.max(1.0e-4)).exp()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoothstep_hits_endpoints_and_midpoint() {
        assert_eq!(smoothstep(-1.0), 0.0);
        assert_eq!(smoothstep(0.0), 0.0);
        assert!((smoothstep(0.5) - 0.5).abs() < 1.0e-6);
        assert_eq!(smoothstep(1.0), 1.0);
        assert_eq!(smoothstep(2.0), 1.0);
    }

    #[test]
    fn ramp_maps_range_and_handles_degenerate_range() {
        assert_eq!(ramp(50.0, 60.0, 100.0), 0.0);
        assert_eq!(ramp(100.0, 60.0, 100.0), 1.0);
        assert!((ramp(80.0, 60.0, 100.0) - 0.5).abs() < 1.0e-6);
        assert_eq!(ramp(5.0, 5.0, 5.0), 1.0);
        assert_eq!(ramp(4.0, 5.0, 5.0), 0.0);
    }

    #[test]
    fn one_pole_coefficient_is_faster_for_shorter_times() {
        let fast = one_pole_coeff(0.02, 0.01);
        let slow = one_pole_coeff(0.02, 1.0);
        assert!(fast > slow && fast < 1.0 && slow > 0.0);
    }
}
