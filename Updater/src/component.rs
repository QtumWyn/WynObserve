use std::fmt;

#[derive(Debug, Clone, Copy)]
pub enum Component {
    Agent,
    Observatory,
    Updater,
}

impl Component {
    pub const ALL: [Self; 3] = [Self::Agent, Self::Observatory, Self::Updater];

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Agent => "Agent",
            Self::Observatory => "Observatory",
            Self::Updater => "Updater",
        }
    }

    pub fn package_name(self) -> &'static str {
        match self {
            Self::Agent => "wyncommand-agent",
            Self::Observatory => "wynobserve",
            Self::Updater => "wyn-updater",
        }
    }

    pub fn release_prefix(self) -> &'static str {
        match self {
            Self::Agent => "agent-v",
            Self::Observatory => "observatory-v",
            Self::Updater => "updater-v",
        }
    }
}

impl fmt::Display for Component {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.display_name())
    }
}
