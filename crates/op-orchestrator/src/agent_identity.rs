//! Visual identities for the concurrent "agent team" — each parallel
//! design sub-agent gets a distinct colour + name so the canvas can
//! draw per-agent breathing indicators and badges.

/// Fixed 6-colour palette. Colour is assigned by agent index (cycled),
/// so a given team always paints the same colours in the same order.
pub const AGENT_COLORS: [&str; 6] = [
    "#FF6B6B", // coral red
    "#4ECDC4", // teal
    // Cobalt blue — replaced the golden yellow: the name pill renders its
    // label in white, which was unreadable on yellow (user report).
    "#5B8DEF", "#6C5CE7", // purple
    "#51C878", // emerald - replaced pale mint for the same white-label reason
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
    assign_agent_identities_seeded(count, 0)
}

/// Like [`assign_agent_identities`], but rotated by a per-run `seed` so a
/// fresh run meets a fresh face — index 0 was ALWAYS Kiki-in-coral before.
/// Names and colours rotate on co-prime strides (pool sizes 12 and 6), so
/// the same name still shows up in different colours across runs. Teams
/// stay distinct: identities within one call never collide for counts up
/// to the pool sizes.
pub fn assign_agent_identities_seeded(count: usize, seed: u64) -> Vec<AgentIdentity> {
    let name_offset = (seed % AGENT_NAMES.len() as u64) as usize;
    let color_offset = ((seed / AGENT_NAMES.len() as u64) % AGENT_COLORS.len() as u64) as usize;
    (0..count)
        .map(|i| AgentIdentity {
            color: AGENT_COLORS[(color_offset + i) % AGENT_COLORS.len()].to_string(),
            name: AGENT_NAMES[(name_offset + i) % AGENT_NAMES.len()].to_string(),
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
        assert_eq!(ids[2].color, "#5B8DEF");
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

    #[test]
    fn seed_rotates_names_and_colors_but_keeps_teams_distinct() {
        let a = assign_agent_identities_seeded(3, 0);
        let b = assign_agent_identities_seeded(3, 5);
        assert_ne!(a[0].name, b[0].name, "a fresh seed meets a fresh face");
        let c = assign_agent_identities_seeded(4, 17);
        for i in 0..c.len() {
            for j in (i + 1)..c.len() {
                assert_ne!(c[i].name, c[j].name, "teammates stay distinct");
                assert_ne!(c[i].color, c[j].color);
            }
        }
    }
}
