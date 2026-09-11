use std::fmt;

#[derive(Debug, Clone, Copy)]
pub enum Component {
    Agent,
    Observatory,
}

impl Component {
    pub const ALL: [Self; 2] = [Self::Agent, Self::Observatory];

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Agent => "Agent",
            Self::Observatory => "Observatory",
        }
    }

    pub fn package_name(self) -> &'static str {
        match self {
            Self::Agent => "wyncommand-agent",
            Self::Observatory => "wynobserve",
        }
    }

    pub fn release_prefix(self) -> &'static str {
        match self {
            Self::Agent => "agent-v",
            Self::Observatory => "observatory-v",
        }
    }
}

impl fmt::Display for Component {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.display_name())
    }
}
