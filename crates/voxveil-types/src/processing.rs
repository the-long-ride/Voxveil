use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UnitValue(f32);

impl UnitValue {
    pub fn new(value: f32) -> Result<Self, &'static str> {
        if value.is_finite() && (0.0..=1.0).contains(&value) {
            Ok(Self(value))
        } else {
            Err("value must be finite and between 0.0 and 1.0")
        }
    }

    pub const fn get(self) -> f32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VocalLevel(UnitValue);

impl VocalLevel {
    pub fn new(value: f32) -> Result<Self, &'static str> {
        UnitValue::new(value).map(Self)
    }

    pub const fn get(self) -> f32 {
        self.0.get()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QualityPreference(UnitValue);

impl QualityPreference {
    pub fn new(value: f32) -> Result<Self, &'static str> {
        UnitValue::new(value).map(Self)
    }

    pub const fn get(self) -> f32 {
        self.0.get()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProcessingMode {
    All,
    PerApp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProcessingBackendStatus {
    Ready,
    ComponentRequired,
    RoutingRequired,
    Unsupported,
    Faulted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProcessingLoad {
    Idle,
    Low,
    Medium,
    High,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProcessingEngineKind {
    Auto,
    Dsp,
    Ai,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_values_reject_out_of_range_and_non_finite_values() {
        assert!(VocalLevel::new(-0.1).is_err());
        assert!(VocalLevel::new(1.1).is_err());
        assert!(VocalLevel::new(f32::NAN).is_err());
        assert_eq!(VocalLevel::new(0.0).unwrap().get(), 0.0);
        assert_eq!(VocalLevel::new(1.0).unwrap().get(), 1.0);
    }

    #[test]
    fn quality_uses_the_same_unit_interval() {
        assert!(QualityPreference::new(0.5).is_ok());
        assert!(QualityPreference::new(2.0).is_err());
    }
}
