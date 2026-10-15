//! Owned map compositions. Models select a scene; geometry is deterministic.
use crate::role_defaults::Theme;
use serde_json::{json, Value};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapTemplate {
    Running,
    Delivery,
    Location,
}

impl MapTemplate {
    pub fn from_hint(hint: &str) -> Self {
        let hint = hint.to_lowercase();
        for (token, template) in [
            ("map-template:running", Self::Running),
            ("map-template:delivery", Self::Delivery),
            ("map-template:location", Self::Location),
        ] {
            if hint.contains(token) {
                return template;
            }
        }
        if [
            "配送", "外卖", "delivery", "courier", "打车", "ride", "driving",
        ]
        .iter()
        .any(|word| hint.contains(word))
        {
            Self::Delivery
        } else if [
            "跑步",
            "运动路线",
            "骑行",
            "running",
            "jogging",
            "cycling",
            "workout",
        ]
        .iter()
        .any(|word| hint.contains(word))
        {
            Self::Running
        } else {
            Self::Location
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Delivery => "delivery",
            Self::Location => "location",
        }
    }
}

/// First child paints on top in OP. Paths use their tight coordinate bounds,
/// not viewport dimensions, so native/web fitting keeps pins on endpoints.
pub fn children(id: &str, width: f64, height: f64, theme: Theme, kind: MapTemplate) -> Vec<Value> {
    let dark = theme == Theme::Dark;
    let block = if dark { "#29343C" } else { "#E5EAE6" };
    let road = if dark { "#18232B" } else { "#FAFCF8" };
    let park = if dark { "#28483E" } else { "#D4E7D6" };
    let water = if dark { "#233E50" } else { "#D8EAF0" };
    let surface = if dark { "#40535D" } else { "#FFFFFF" };
    let ink = if dark { "#E8F0ED" } else { "#284339" };
    let w = width.max(1.0);
    let h = height.max(1.0);
    let mut nodes = Vec::new();
    let mut rect = |suffix: &str, x: f64, y: f64, rw: f64, rh: f64, color: &str, radius: f64| {
        nodes.push(json!({"type":"rectangle", "id":format!("{id}-{suffix}"),"name":suffix,
            "x":x*w,"y":y*h,"width":rw*w,"height":rh*h,"cornerRadius":radius.min(rw*w/2.0).min(rh*h/2.0),
            "fill":[{"type":"solid","color":color}]}));
    };
    rect("map-ground", 0.0, 0.0, 1.0, 1.0, road, 0.0);
    // Broken street grid leaves a stable diagonal corridor for water/greenery.
    for (i, (x, y, rw, rh)) in [
        (0.03, 0.04, 0.18, 0.18),
        (0.24, 0.04, 0.23, 0.18),
        (0.50, 0.04, 0.20, 0.18),
        (0.73, 0.04, 0.24, 0.18),
        (0.03, 0.26, 0.18, 0.20),
        (0.24, 0.26, 0.23, 0.20),
        (0.50, 0.26, 0.20, 0.20),
        (0.73, 0.26, 0.24, 0.20),
        (0.03, 0.50, 0.18, 0.22),
        (0.24, 0.50, 0.23, 0.22),
        (0.50, 0.50, 0.20, 0.22),
        (0.73, 0.50, 0.24, 0.22),
        (0.03, 0.76, 0.18, 0.20),
        (0.24, 0.76, 0.23, 0.20),
        (0.50, 0.76, 0.20, 0.20),
        (0.73, 0.76, 0.24, 0.20),
    ]
    .into_iter()
    .enumerate()
    {
        rect(&format!("map-block-{}", i + 1), x, y, rw, rh, block, 5.0);
    }
    match kind {
        MapTemplate::Running => {
            rect("map-park", 0.25, 0.27, 0.44, 0.44, park, 24.0);
            rect("map-pond", 0.38, 0.38, 0.18, 0.16, water, 24.0);
        }
        MapTemplate::Delivery => rect("map-park", 0.74, 0.51, 0.22, 0.20, park, 8.0),
        MapTemplate::Location => {
            rect("map-park", 0.50, 0.26, 0.20, 0.20, park, 8.0);
            rect("map-waterfront", 0.74, 0.0, 0.10, 1.0, water, 0.0);
        }
    }
    let points: &[(f64, f64)] = match kind {
        MapTemplate::Running => &[
            (0.30, 0.66),
            (0.24, 0.58),
            (0.24, 0.34),
            (0.31, 0.25),
            (0.59, 0.25),
            (0.68, 0.34),
            (0.68, 0.60),
            (0.60, 0.70),
            (0.42, 0.70),
        ],
        MapTemplate::Delivery => &[
            (0.12, 0.74),
            (0.225, 0.74),
            (0.225, 0.48),
            (0.485, 0.48),
            (0.485, 0.24),
            (0.66, 0.24),
        ],
        MapTemplate::Location => &[],
    };
    if !points.is_empty() {
        nodes.push(route(id, "map-route-halo", points, w, h, surface, 8.0));
        nodes.push(route(id, "map-route", points, w, h, "$--primary", 4.0));
        let start = points[0];
        if kind == MapTemplate::Running {
            nodes.push(dot(id, "map-origin-halo", start, w, h, 18.0, surface));
            nodes.push(dot(id, "map-origin", start, w, h, 10.0, "$--primary"));
        } else {
            nodes.push(marker(
                id,
                "map-origin",
                "store",
                start,
                (w, h),
                surface,
                ink,
            ));
            nodes.push(marker(
                id,
                "map-courier",
                "bike",
                (0.485, 0.40),
                (w, h),
                surface,
                "$--primary",
            ));
        }
    }
    let end = points.last().copied().unwrap_or((0.485, 0.48));
    if kind == MapTemplate::Running {
        nodes.push(dot(id, "map-destination-halo", end, w, h, 24.0, surface));
        nodes.push(dot(id, "map-pin", end, w, h, 14.0, "$--primary"));
    } else {
        let icon = if kind == MapTemplate::Delivery {
            "house"
        } else {
            "map-pin"
        };
        nodes.push(marker(
            id,
            "map-pin",
            icon,
            end,
            (w, h),
            "$--primary",
            "$--primary-foreground",
        ));
    }
    // Decorative preview control: no fabricated behavior or remote map service.
    if w >= 160.0 && h >= 120.0 {
        nodes.push(
            json!({"type":"frame","id":format!("{id}-map-layers"),"name":"Map layer control",
            "x":w-52.0,"y":12.0,"width":40,"height":40,"layout":"horizontal",
            "justifyContent":"center","alignItems":"center","cornerRadius":12,
            "fill":[{"type":"solid","color":surface}],"children":[
                {"type":"icon_font","id":format!("{id}-map-layers-icon"),"name":"Layers icon",
                 "width":20,"height":20,"iconFontFamily":"lucide","iconFontName":"layers",
                 "fill":[{"type":"solid","color":ink}]}]}),
        );
    }
    nodes.reverse();
    nodes
}

/// Symbols distinguish business roles without inventing addresses or map labels.
fn marker(
    id: &str,
    name: &str,
    icon: &str,
    p: (f64, f64),
    viewport: (f64, f64),
    surface: &str,
    ink: &str,
) -> Value {
    let (w, h) = viewport;
    let size = 32.0_f64.min(w * 0.16).min(h * 0.16);
    json!({"type":"frame","id":format!("{id}-{name}"),"name":name,
        "x":p.0*w-size/2.0,"y":p.1*h-size/2.0,"width":size,"height":size,
        "layout":"horizontal","justifyContent":"center","alignItems":"center",
        "cornerRadius":size/2.0,"fill":[{"type":"solid","color":surface}],"children":[
            {"type":"icon_font","id":format!("{id}-{name}-icon"),"name":format!("Map {icon} marker"),
             "width":size*0.5625,"height":size*0.5625,"iconFontFamily":"lucide","iconFontName":icon,
             "fill":[{"type":"solid","color":ink}]}]})
}

fn dot(id: &str, name: &str, p: (f64, f64), w: f64, h: f64, size: f64, color: &str) -> Value {
    let size = size.min(w * 0.16).min(h * 0.16);
    json!({"type":"ellipse","id":format!("{id}-{name}"),"name":name,
        "x":p.0*w-size/2.0,"y":p.1*h-size/2.0,"width":size,"height":size,
        "fill":[{"type":"solid","color":color}]})
}

fn route(
    id: &str,
    name: &str,
    points: &[(f64, f64)],
    w: f64,
    h: f64,
    color: &str,
    thickness: f64,
) -> Value {
    let min_x = points.iter().map(|p| p.0 * w).fold(f64::INFINITY, f64::min);
    let min_y = points.iter().map(|p| p.1 * h).fold(f64::INFINITY, f64::min);
    let max_x = points
        .iter()
        .map(|p| p.0 * w)
        .fold(f64::NEG_INFINITY, f64::max);
    let max_y = points
        .iter()
        .map(|p| p.1 * h)
        .fold(f64::NEG_INFINITY, f64::max);
    let d = points
        .iter()
        .enumerate()
        .map(|(i, p)| {
            format!(
                "{} {} {}",
                if i == 0 { "M" } else { "L" },
                p.0 * w - min_x,
                p.1 * h - min_y
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    json!({"type":"path","id":format!("{id}-{name}"),"name":name,
        "x":min_x,"y":min_y,"width":max_x-min_x,"height":max_y-min_y,"d":d,"fill":[],
        "stroke":{"thickness":thickness.min(w*0.02).min(h*0.02),"cap":"round","join":"round","fill":[{"type":"solid","color":color}]}})
}

#[cfg(test)]
#[path = "map_templates_tests.rs"]
mod tests;
