#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edition {
    Standard,
    ProSystem,
}

impl Edition {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::ProSystem => "pro-system",
        }
    }
}

pub fn parse_edition(value: Option<&str>) -> Edition {
    match value {
        Some("pro-system") => Edition::ProSystem,
        _ => Edition::Standard,
    }
}

pub fn current_edition() -> Edition {
    parse_edition(option_env!("VOXVEIL_EDITION"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_explicit_pro_system_selects_privileged_edition() {
        assert_eq!(parse_edition(Some("pro-system")), Edition::ProSystem);
        assert_eq!(parse_edition(Some("standard")), Edition::Standard);
        assert_eq!(parse_edition(None), Edition::Standard);
    }
}
