use crate::arm9::ManualPacketEncoder;
use crate::compact::CompactReport;
use crate::mode_sound::ModeSoundPlayer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Arm9,
}

impl OutputFormat {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "arm9" => Ok(Self::Arm9),
            other => Err(format!("unsupported output format: {other}")),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Arm9 => "arm9",
        }
    }

    pub fn create_driver(self) -> Box<dyn OutputDriver> {
        match self {
            Self::Arm9 => Box::new(Arm9OutputDriver::new()),
        }
    }
}

pub trait OutputDriver {
    fn format_name(&self) -> &'static str;
    fn encode(&mut self, compact_report: &CompactReport) -> Result<Vec<u8>, String>;
}

struct Arm9OutputDriver {
    encoder: ManualPacketEncoder,
    sound_player: ModeSoundPlayer,
}

impl Arm9OutputDriver {
    fn new() -> Self {
        Self {
            encoder: ManualPacketEncoder::new(),
            sound_player: ModeSoundPlayer::new(),
        }
    }
}

impl OutputDriver for Arm9OutputDriver {
    fn format_name(&self) -> &'static str {
        OutputFormat::Arm9.as_str()
    }

    fn encode(&mut self, compact_report: &CompactReport) -> Result<Vec<u8>, String> {
        let update = self.encoder.encode_compact_report_update(compact_report);
        if update.profile_changed {
            self.sound_player.play(update.profile.as_str());
        }
        Ok(update.packet.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::OutputFormat;

    #[test]
    fn parse_supports_arm9() {
        assert_eq!(
            OutputFormat::parse("arm9").expect("should parse"),
            OutputFormat::Arm9
        );
    }
}
