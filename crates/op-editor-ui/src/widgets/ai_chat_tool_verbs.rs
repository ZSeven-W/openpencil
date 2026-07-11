//! Narrative labels for raw AI tool names.

pub(crate) fn verb_for_tool(name: &str) -> &str {
    match name {
        "batch_design" => "Designed",
        "get_guidelines" => "Checked guidelines",
        "get_screenshot" => "Reviewed visuals",
        "snapshot_layout" => "Checked layout",
        "get_variables" => "Read variables",
        "set_variables" => "Updated variables",
        "batch_get" => "Read components",
        "get_editor_state" => "Read canvas",
        "get_style_guide" | "get_style_guide_tags" => "Explored styles",
        "find_empty_space" => "Found placement",
        "export_nodes" => "Exported",
        "spawn_agents" => "Spawned agents",
        "ToolSearch" => "Loaded tools",
        _ => name,
    }
}

#[cfg(test)]
mod tests {
    use super::verb_for_tool;

    #[test]
    fn maps_every_known_tool_to_its_narrative_verb() {
        let cases = [
            ("batch_design", "Designed"),
            ("get_guidelines", "Checked guidelines"),
            ("get_screenshot", "Reviewed visuals"),
            ("snapshot_layout", "Checked layout"),
            ("get_variables", "Read variables"),
            ("set_variables", "Updated variables"),
            ("batch_get", "Read components"),
            ("get_editor_state", "Read canvas"),
            ("get_style_guide", "Explored styles"),
            ("get_style_guide_tags", "Explored styles"),
            ("find_empty_space", "Found placement"),
            ("export_nodes", "Exported"),
            ("spawn_agents", "Spawned agents"),
            ("ToolSearch", "Loaded tools"),
        ];

        for (name, expected) in cases {
            assert_eq!(verb_for_tool(name), expected, "mapping for {name}");
        }
    }

    #[test]
    fn preserves_unknown_tool_name() {
        assert_eq!(verb_for_tool("custom_tool"), "custom_tool");
    }
}
