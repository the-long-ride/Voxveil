pub fn energy(samples: &[f32]) -> f64 {
    samples.iter().map(|&v| f64::from(v) * f64::from(v)).sum()
}

pub fn error_energy(estimate: &[f32], reference: &[f32]) -> f64 {
    estimate
        .iter()
        .zip(reference)
        .map(|(&e, &r)| {
            let d = f64::from(e) - f64::from(r);
            d * d
        })
        .sum()
}

pub fn ratio_db(numerator: f64, denominator: f64) -> f64 {
    10.0 * ((numerator + 1.0e-20) / (denominator + 1.0e-20)).log10()
}

pub fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    let n = values.len();
    if n == 0 {
        return f64::NAN;
    }
    if n % 2 == 1 { values[n / 2] } else { 0.5 * (values[n / 2 - 1] + values[n / 2]) }
}

pub struct Row {
    pub song: String,
    pub variant: String,
    pub sdr_inst: f64,
    pub vocal_reduction_db: Option<f64>,
    pub inst_damage_sdr: Option<f64>,
    pub cpu_percent: f64,
}

impl Row {
    pub const CSV_HEADER: &'static str =
        "song,variant,sdr_inst_db,vocal_reduction_db,inst_damage_sdr_db,cpu_percent";

    pub fn to_csv(&self) -> String {
        let optional = |v: Option<f64>| v.map_or(String::new(), |v| format!("{v:.2}"));
        format!(
            "{},{},{:.2},{},{},{:.3}",
            self.song,
            self.variant,
            self.sdr_inst,
            optional(self.vocal_reduction_db),
            optional(self.inst_damage_sdr),
            self.cpu_percent
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ratio_db_and_error_energy() {
        assert!((ratio_db(10.0, 1.0) - 10.0).abs() < 1.0e-9);
        assert_eq!(error_energy(&[0.5, -0.5], &[0.5, -0.5]), 0.0);
        assert!((energy(&[3.0, 4.0]) - 25.0).abs() < 1.0e-9);
    }

    #[test]
    fn median_handles_odd_and_even() {
        assert_eq!(median(&mut [3.0, 1.0, 2.0]), 2.0);
        assert_eq!(median(&mut [4.0, 1.0, 2.0, 3.0]), 2.5);
    }

    #[test]
    fn csv_row_has_one_field_per_header_column() {
        let row = Row {
            song: "a+b".into(),
            variant: "isolation".into(),
            sdr_inst: 1.0,
            vocal_reduction_db: Some(20.0),
            inst_damage_sdr: None,
            cpu_percent: 0.5,
        };
        assert_eq!(row.to_csv().split(',').count(), Row::CSV_HEADER.split(',').count());
    }
}
