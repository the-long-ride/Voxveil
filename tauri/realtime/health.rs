#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AudioHealth {
    pub underruns: u64,
    pub overruns: u64,
}

impl AudioHealth {
    pub fn record_underrun(&mut self) {
        self.underruns = self.underruns.saturating_add(1);
    }

    pub fn record_overrun(&mut self) {
        self.overruns = self.overruns.saturating_add(1);
    }

    pub const fn degraded(self) -> bool {
        self.underruns > 0 || self.overruns > 0
    }
}
