/// Every tunable constant of the vocal separator. Defaults follow the design spec;
/// the evaluation harness overrides them by name with `set`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SeparatorParams {
    pub center_avg: f32,
    pub center_psi_exp: f32,
    pub center_rho_exp: f32,
    pub pitch_min_hz: f32,
    pub pitch_max_hz: f32,
    pub pitch_jump_penalty: f32,
    pub pitch_octave_guard: f32,
    pub voicing_lo: f32,
    pub voicing_hi: f32,
    pub voicing_release_s: f32,
    pub comb_max_hz: f32,
    pub vibrato_cents: f32,
    pub unvoiced_lo_hz: f32,
    pub unvoiced_hi_hz: f32,
    pub flatness_lo: f32,
    pub flatness_hi: f32,
    pub center_share: f32,
    pub share_lo_hz: f32,
    pub share_hi_hz: f32,
    pub transient_guard: f32,
    pub floor_gain: f32,
    pub low_protect_hz: f32,
    pub low_full_hz: f32,
    pub attack: f32,
    pub release_s: f32,
    pub wiener_exp: f32,
    pub reverb_t60_s: f32,
    pub reverb_beta: f32,
    pub side_floor: f32,
    pub reverb_lo_hz: f32,
    pub reverb_hi_hz: f32,
}

impl Default for SeparatorParams {
    fn default() -> Self {
        Self {
            center_avg: 0.6,
            center_psi_exp: 4.0,
            center_rho_exp: 2.0,
            pitch_min_hz: 80.0,
            pitch_max_hz: 1_000.0,
            pitch_jump_penalty: 0.03,
            pitch_octave_guard: 0.5,
            voicing_lo: 0.08,
            voicing_hi: 0.25,
            voicing_release_s: 0.12,
            comb_max_hz: 5_000.0,
            vibrato_cents: 50.0,
            unvoiced_lo_hz: 4_000.0,
            unvoiced_hi_hz: 16_000.0,
            flatness_lo: 0.15,
            flatness_hi: 0.45,
            center_share: 0.35,
            share_lo_hz: 200.0,
            share_hi_hz: 350.0,
            transient_guard: 0.5,
            floor_gain: 0.01,
            low_protect_hz: 60.0,
            low_full_hz: 100.0,
            attack: 0.8,
            release_s: 0.06,
            wiener_exp: 1.0,
            reverb_t60_s: 1.2,
            reverb_beta: 0.5,
            side_floor: 0.25,
            reverb_lo_hz: 200.0,
            reverb_hi_hz: 8_000.0,
        }
    }
}

impl SeparatorParams {
    /// Overrides one parameter by its field name (used by the evaluation sweep).
    pub fn set(&mut self, key: &str, value: f32) -> Result<(), String> {
        if !value.is_finite() {
            return Err(format!("{key} must be finite"));
        }
        let slot = match key {
            "center_avg" => &mut self.center_avg,
            "center_psi_exp" => &mut self.center_psi_exp,
            "center_rho_exp" => &mut self.center_rho_exp,
            "pitch_min_hz" => &mut self.pitch_min_hz,
            "pitch_max_hz" => &mut self.pitch_max_hz,
            "pitch_jump_penalty" => &mut self.pitch_jump_penalty,
            "pitch_octave_guard" => &mut self.pitch_octave_guard,
            "voicing_lo" => &mut self.voicing_lo,
            "voicing_hi" => &mut self.voicing_hi,
            "voicing_release_s" => &mut self.voicing_release_s,
            "comb_max_hz" => &mut self.comb_max_hz,
            "vibrato_cents" => &mut self.vibrato_cents,
            "unvoiced_lo_hz" => &mut self.unvoiced_lo_hz,
            "unvoiced_hi_hz" => &mut self.unvoiced_hi_hz,
            "flatness_lo" => &mut self.flatness_lo,
            "flatness_hi" => &mut self.flatness_hi,
            "center_share" => &mut self.center_share,
            "share_lo_hz" => &mut self.share_lo_hz,
            "share_hi_hz" => &mut self.share_hi_hz,
            "transient_guard" => &mut self.transient_guard,
            "floor_gain" => &mut self.floor_gain,
            "low_protect_hz" => &mut self.low_protect_hz,
            "low_full_hz" => &mut self.low_full_hz,
            "attack" => &mut self.attack,
            "release_s" => &mut self.release_s,
            "wiener_exp" => &mut self.wiener_exp,
            "reverb_t60_s" => &mut self.reverb_t60_s,
            "reverb_beta" => &mut self.reverb_beta,
            "side_floor" => &mut self.side_floor,
            "reverb_lo_hz" => &mut self.reverb_lo_hz,
            "reverb_hi_hz" => &mut self.reverb_hi_hz,
            _ => return Err(format!("unknown separator parameter: {key}")),
        };
        *slot = value;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_spec() {
        let params = SeparatorParams::default();
        assert_eq!(params.floor_gain, 0.01);
        assert_eq!(params.center_share, 0.35);
        assert_eq!(params.voicing_lo, 0.08);
        assert_eq!(params.voicing_hi, 0.25);
        assert_eq!(params.low_protect_hz, 60.0);
    }

    #[test]
    fn set_updates_known_keys_and_rejects_bad_input() {
        let mut params = SeparatorParams::default();
        params.set("wiener_exp", 1.5).unwrap();
        assert_eq!(params.wiener_exp, 1.5);
        assert!(params.set("not_a_key", 1.0).is_err());
        assert!(params.set("floor_gain", f32::NAN).is_err());
    }
}
