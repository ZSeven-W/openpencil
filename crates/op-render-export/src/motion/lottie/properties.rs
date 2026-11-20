//! Bake shared motion samples into Lottie properties.
use serde_json::{json, Map, Value};

pub(super) fn property(value: impl Into<Value>) -> Value {
    json!({"a": 0, "k": value.into()})
}

pub(super) fn animated(values: &[Value]) -> Value {
    if values.iter().all(|value| value == &values[0]) {
        return property(values[0].clone());
    }
    let mut keys = Vec::new();
    for (index, value) in values.iter().enumerate() {
        if index > 0 && value == &values[index - 1] {
            continue;
        }
        let sample = if value.is_array() {
            value.clone()
        } else {
            json!([value])
        };
        keys.push(json!({"t": index, "s": sample, "h": 1}));
    }
    json!({"a": 1, "k": keys})
}

pub(super) fn transform() -> Value {
    json!({"o":property(100), "r":property(0), "p":property(json!([0,0])),
        "a":property(json!([0,0])), "s":property(json!([100,100]))})
}

/// Only properties may vary inside a stable tree. A changed topology becomes
/// separate in/out layers, rather than an invalid path morph.
fn merge(values: &[&Value]) -> Option<Value> {
    let first = values[0];
    if values.iter().all(|value| *value == first) {
        return Some(first.clone());
    }
    match first {
        Value::Object(map) if map.get("a") == Some(&json!(0)) && map.contains_key("k") => {
            let samples: Option<Vec<_>> = values.iter().map(|v| v.get("k").cloned()).collect();
            let samples = samples?;
            if let Some(vertices) = samples[0].get("v").and_then(Value::as_array) {
                let closed = &samples[0]["c"];
                if samples.iter().any(|sample| {
                    sample.get("v").and_then(Value::as_array).map(Vec::len) != Some(vertices.len())
                        || &sample["c"] != closed
                }) {
                    return None;
                }
            }
            Some(animated(&samples))
        }
        Value::Object(map) => {
            let mut result = Map::new();
            for key in map.keys() {
                let entries: Option<Vec<_>> = values.iter().map(|value| value.get(key)).collect();
                result.insert(key.clone(), merge(&entries?)?);
            }
            Some(Value::Object(result))
        }
        Value::Array(array)
            if values
                .iter()
                .all(|v| v.as_array().map(Vec::len) == Some(array.len())) =>
        {
            (0..array.len())
                .map(|index| merge(&values.iter().map(|v| &v[index]).collect::<Vec<_>>()))
                .collect::<Option<Vec<_>>>()
                .map(Value::Array)
        }
        _ => None,
    }
}

pub(super) fn timed_layers(samples: Vec<Value>, end: u64) -> Vec<Value> {
    if let Some(mut layer) = merge(&samples.iter().collect::<Vec<_>>()) {
        layer["ip"] = json!(0);
        layer["op"] = json!(end);
        return vec![layer];
    }
    let mut layers: Vec<Value> = Vec::new();
    let mut previous: Option<Value> = None;
    for (frame, sample) in samples.into_iter().enumerate() {
        if previous.as_ref() == Some(&sample) {
            layers.last_mut().unwrap()["op"] = json!(frame + 1);
        } else {
            previous = Some(sample.clone());
            let mut layer = sample;
            layer["ip"] = json!(frame);
            layer["op"] = json!(frame + 1);
            layers.push(layer);
        }
    }
    layers
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shape_layer(vertices: &[[f32; 2]], closed: bool) -> Value {
        let tangents = vec![[0.0, 0.0]; vertices.len()];
        json!({"ty":4,"shapes":[{"ty":"sh","ks":property(json!({
            "v":vertices,"i":tangents,"o":tangents,"c":closed
        }))}]})
    }

    #[test]
    fn changing_vertex_counts_become_discrete_layers_even_with_hold_keyframes() {
        let triangle = shape_layer(&[[0.0, 0.0], [20.0, 0.0], [10.0, 20.0]], true);
        let rectangle = shape_layer(&[[0.0, 0.0], [20.0, 0.0], [20.0, 20.0], [0.0, 20.0]], true);
        let layers = timed_layers(vec![triangle.clone(), rectangle, triangle], 3);
        assert_eq!(layers.len(), 3);
        for (index, layer) in layers.iter().enumerate() {
            assert_eq!(layer["ip"], index);
            assert_eq!(layer["op"], index + 1);
            assert_eq!(layer["shapes"][0]["ks"]["a"], 0);
        }
    }

    #[test]
    fn matching_topology_keeps_animated_paths_but_closure_changes_do_not() {
        let points = [[0.0, 0.0], [20.0, 0.0], [10.0, 20.0]];
        let first = shape_layer(&points, true);
        let moved = shape_layer(&points.map(|[x, y]| [x + 10.0, y]), true);
        let layers = timed_layers(vec![first.clone(), moved], 2);
        assert_eq!(layers.len(), 1);
        assert_eq!(layers[0]["shapes"][0]["ks"]["a"], 1);
        let open = shape_layer(&points, false);
        assert_eq!(timed_layers(vec![first, open], 2).len(), 2);
    }
}
