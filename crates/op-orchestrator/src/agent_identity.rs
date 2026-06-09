//! Visual identities for the concurrent "agent team" — each parallel
//! design sub-agent gets a distinct colour + name so the canvas can
//! draw per-agent breathing indicators and badges.

/// Fixed 6-colour palette. Colour is assigned by agent index (cycled),
/// so a given team always paints the same colours in the same order.
pub const AGENT_COLORS: [&str; 6] = [
    "#FF6B6B", // coral red
    "#4ECDC4", // teal
    "#FFD93D", // golden yellow
    "#6C5CE7", // purple
    "#A8E6CF", // mint green
    "#FF8A5C", // warm orange
];

/// Name pool — distinct for the first 12 agents (a team never gets
/// anywhere near that many).
pub const AGENT_NAMES: [&str; 12] = [
    "Kiki", "Mochi", "Pixel", "Nova", "Zuri", "Cleo", "Boba", "Rune", "Fern", "Echo", "Puck",
    "Sage",
];

/// A parallel design agent's visual identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentIdentity {
    /// Hex colour string, e.g. `"#FF6B6B"`.
    pub color: String,
    /// Display name, e.g. `"Nova"`.
    pub name: String,
}

/// Assign `count` distinct identities. Colour cycles the palette by
/// index; name is taken from the pool by index (distinct for the first
/// 12 agents).
pub fn assign_agent_identities(count: usize) -> Vec<AgentIdentity> {
    (0..count)
        .map(|i| AgentIdentity {
            color: AGENT_COLORS[i % AGENT_COLORS.len()].to_string(),
            name: AGENT_NAMES[i % AGENT_NAMES.len()].to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assigns_palette_colors_in_order() {
        let ids = assign_agent_identities(3);
        assert_eq!(ids.len(), 3);
        assert_eq!(ids[0].color, "#FF6B6B");
        assert_eq!(ids[1].color, "#4ECDC4");
        assert_eq!(ids[2].color, "#FFD93D");
        assert_ne!(ids[0].name, ids[1].name);
        assert_ne!(ids[1].name, ids[2].name);
    }

    #[test]
    fn colors_cycle_past_the_palette_size() {
        let ids = assign_agent_identities(7);
        assert_eq!(ids[6].color, ids[0].color);
    }

    #[test]
    fn empty_team_yields_no_identities() {
        assert!(assign_agent_identities(0).is_empty());
    }
}
