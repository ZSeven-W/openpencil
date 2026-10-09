//! Authored horizontal viewport intent shared by preview and delivery checks.

pub fn is_horizontal_scroll_viewport(value: &serde_json::Value) -> bool {
    if value["clipContent"] != true {
        return false;
    }
    if ["onDrag", "onSwipe"].iter().any(|hook| {
        value["events"][*hook]
            .as_array()
            .is_some_and(|actions| !actions.is_empty())
    }) {
        return false;
    }
    let Some(kids) = value["children"].as_array() else {
        return false;
    };
    let lane = if kids.len() == 1 && kids[0]["layout"] == "horizontal" {
        &kids[0]
    } else {
        value
    };
    if lane["layout"] != "horizontal"
        || lane["children"]
            .as_array()
            .is_none_or(|items| items.len() < 2)
    {
        return false;
    }
    [value, lane].iter().any(|v| {
        ["name", "role"].iter().any(|key| {
            v[*key].as_str().is_some_and(|s| {
                let s = s.to_ascii_lowercase();
                [
                    "scroll", "carousel", "viewport", "横向", "滚动", "轮播", "轨道", "视口",
                ]
                .iter()
                .any(|word| s.contains(word))
            })
        })
    })
}
