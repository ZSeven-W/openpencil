use std::f64::consts::PI;

#[derive(Clone, Copy, Debug)]
struct Rgba {
    r: f64,
    g: f64,
    b: f64,
    a: f64,
}

pub fn parse_css_color(value: &str) -> Option<String> {
    parse_color(&value.trim().to_ascii_lowercase(), 0).map(format_rgba)
}

fn parse_color(value: &str, depth: usize) -> Option<Rgba> {
    if depth > 32 || value == "currentcolor" {
        return None;
    }
    if let Some(hex) = value.strip_prefix('#') {
        return parse_hex(hex);
    }
    for (name, parser) in [
        ("rgb", parse_rgb as fn(&str) -> Option<Rgba>),
        ("rgba", parse_rgb),
        ("hsl", parse_hsl),
        ("hsla", parse_hsl),
        ("hwb", parse_hwb),
        ("lab", parse_lab),
        ("lch", parse_lch),
        ("oklab", parse_oklab),
        ("oklch", parse_oklch),
        ("color", parse_color_function),
    ] {
        if let Some(body) = function_body(value, name) {
            return parser(body);
        }
    }
    if let Some(body) = function_body(value, "color-mix") {
        return parse_color_mix(body, depth + 1);
    }
    parse_named(value)
}

fn function_body<'a>(value: &'a str, name: &str) -> Option<&'a str> {
    value
        .strip_prefix(name)?
        .strip_prefix('(')?
        .strip_suffix(')')
}

fn parse_hex(hex: &str) -> Option<Rgba> {
    if !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let expanded = match hex.len() {
        3 | 4 => hex.chars().flat_map(|ch| [ch, ch]).collect::<String>(),
        6 | 8 => hex.to_string(),
        _ => return None,
    };
    let channel = |offset| u8::from_str_radix(&expanded[offset..offset + 2], 16).ok();
    Some(Rgba {
        r: f64::from(channel(0)?) / 255.0,
        g: f64::from(channel(2)?) / 255.0,
        b: f64::from(channel(4)?) / 255.0,
        a: if expanded.len() == 8 {
            f64::from(channel(6)?) / 255.0
        } else {
            1.0
        },
    })
}

fn parts(body: &str) -> Vec<&str> {
    body.split(|ch: char| ch == ',' || ch == '/' || ch.is_ascii_whitespace())
        .filter(|part| !part.is_empty())
        .collect()
}

fn parse_rgb(body: &str) -> Option<Rgba> {
    let values = parts(body);
    if !(3..=4).contains(&values.len()) {
        return None;
    }
    Some(Rgba {
        r: parse_rgb_component(values[0])?,
        g: parse_rgb_component(values[1])?,
        b: parse_rgb_component(values[2])?,
        a: values
            .get(3)
            .map_or(Some(1.0), |value| parse_alpha(value))?,
    })
}

fn parse_rgb_component(value: &str) -> Option<f64> {
    if let Some(percent) = value.strip_suffix('%') {
        return Some(parse_number(percent)?.clamp(0.0, 100.0) / 100.0);
    }
    Some(parse_number(value)?.clamp(0.0, 255.0) / 255.0)
}

fn parse_alpha(value: &str) -> Option<f64> {
    let alpha = if let Some(percent) = value.strip_suffix('%') {
        parse_number(percent)? / 100.0
    } else {
        parse_number(value)?
    };
    Some(alpha.clamp(0.0, 1.0))
}

fn parse_hsl(body: &str) -> Option<Rgba> {
    let values = parts(body);
    if !(3..=4).contains(&values.len()) {
        return None;
    }
    let hue = parse_angle(values[0])?;
    let saturation = parse_percentage(values[1])?.clamp(0.0, 1.0);
    let lightness = parse_percentage(values[2])?.clamp(0.0, 1.0);
    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let sector = hue.rem_euclid(360.0) / 60.0;
    let x = chroma * (1.0 - (sector.rem_euclid(2.0) - 1.0).abs());
    let (r, g, b) = match sector.floor() as u8 {
        0 => (chroma, x, 0.0),
        1 => (x, chroma, 0.0),
        2 => (0.0, chroma, x),
        3 => (0.0, x, chroma),
        4 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    let m = lightness - chroma / 2.0;
    Some(Rgba {
        r: r + m,
        g: g + m,
        b: b + m,
        a: values
            .get(3)
            .map_or(Some(1.0), |value| parse_alpha(value))?,
    })
}

fn parse_hwb(body: &str) -> Option<Rgba> {
    let values = parts(body);
    if !(3..=4).contains(&values.len()) {
        return None;
    }
    let hue = parse_angle(values[0])?;
    let white = parse_percentage(values[1])?.clamp(0.0, 1.0);
    let black = parse_percentage(values[2])?.clamp(0.0, 1.0);
    let sum = white + black;
    let (r, g, b) = if sum >= 1.0 {
        let gray = white / sum;
        (gray, gray, gray)
    } else {
        let pure = hsl_to_rgb(hue, 1.0, 0.5);
        let scale = 1.0 - sum;
        (
            pure.0 * scale + white,
            pure.1 * scale + white,
            pure.2 * scale + white,
        )
    };
    Some(Rgba {
        r,
        g,
        b,
        a: values
            .get(3)
            .map_or(Some(1.0), |value| parse_alpha(value))?,
    })
}

fn hsl_to_rgb(hue: f64, saturation: f64, lightness: f64) -> (f64, f64, f64) {
    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let sector = hue.rem_euclid(360.0) / 60.0;
    let x = chroma * (1.0 - (sector.rem_euclid(2.0) - 1.0).abs());
    let rgb = match sector.floor() as u8 {
        0 => (chroma, x, 0.0),
        1 => (x, chroma, 0.0),
        2 => (0.0, chroma, x),
        3 => (0.0, x, chroma),
        4 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    let m = lightness - chroma / 2.0;
    (rgb.0 + m, rgb.1 + m, rgb.2 + m)
}

fn parse_lab(body: &str) -> Option<Rgba> {
    let values = parts(body);
    if !(3..=4).contains(&values.len()) {
        return None;
    }
    let l = parse_lightness_100(values[0])?.clamp(0.0, 100.0);
    let a = parse_axis(values[1], 125.0)?;
    let b = parse_axis(values[2], 125.0)?;
    with_alpha(lab_to_srgb(l, a, b), values.get(3))
}

fn parse_lch(body: &str) -> Option<Rgba> {
    let values = parts(body);
    if !(3..=4).contains(&values.len()) {
        return None;
    }
    let l = parse_lightness_100(values[0])?.clamp(0.0, 100.0);
    let c = parse_axis(values[1], 150.0)?.max(0.0);
    let h = parse_angle(values[2])? * PI / 180.0;
    with_alpha(lab_to_srgb(l, c * h.cos(), c * h.sin()), values.get(3))
}

fn parse_oklab(body: &str) -> Option<Rgba> {
    let values = parts(body);
    if !(3..=4).contains(&values.len()) {
        return None;
    }
    let l = parse_lightness_one(values[0])?.clamp(0.0, 1.0);
    let a = parse_axis(values[1], 0.4)?;
    let b = parse_axis(values[2], 0.4)?;
    with_alpha(oklab_to_srgb(l, a, b), values.get(3))
}

fn parse_oklch(body: &str) -> Option<Rgba> {
    let values = parts(body);
    if !(3..=4).contains(&values.len()) {
        return None;
    }
    let l = parse_lightness_one(values[0])?.clamp(0.0, 1.0);
    let c = parse_axis(values[1], 0.4)?.max(0.0);
    let h = parse_angle(values[2])? * PI / 180.0;
    with_alpha(oklab_to_srgb(l, c * h.cos(), c * h.sin()), values.get(3))
}

fn parse_color_function(body: &str) -> Option<Rgba> {
    let values = parts(body);
    if !(4..=5).contains(&values.len()) {
        return None;
    }
    let components = [
        parse_color_component(values[1])?,
        parse_color_component(values[2])?,
        parse_color_component(values[3])?,
    ];
    let rgb = match values[0] {
        "srgb" => (components[0], components[1], components[2]),
        "display-p3" => display_p3_to_srgb(components),
        _ => return None,
    };
    with_alpha(rgb, values.get(4))
}

fn parse_color_component(value: &str) -> Option<f64> {
    if let Some(percent) = value.strip_suffix('%') {
        Some(parse_number(percent)? / 100.0)
    } else {
        parse_number(value)
    }
}

fn parse_color_mix(body: &str, depth: usize) -> Option<Rgba> {
    let args = split_top_level(body, ',')?;
    if args.len() != 3 || args[0].trim() != "in srgb" {
        return None;
    }
    let (left_text, left_weight) = split_mix_weight(args[1])?;
    let (right_text, right_weight) = split_mix_weight(args[2])?;
    let left = parse_color(left_text, depth)?;
    let right = parse_color(right_text, depth)?;
    let (weight_left, weight_right) = match (left_weight, right_weight) {
        (None, None) => (0.5, 0.5),
        (Some(left), None) if left <= 1.0 => (left, 1.0 - left),
        (None, Some(right)) if right <= 1.0 => (1.0 - right, right),
        (Some(left), Some(right)) => (left, right),
        _ => return None,
    };
    if weight_left < 0.0 || weight_right < 0.0 || weight_left + weight_right <= 0.0 {
        return None;
    }
    let sum = weight_left + weight_right;
    let multiplier = sum.min(1.0);
    let left_weight = weight_left / sum;
    let right_weight = weight_right / sum;
    let mixed_alpha = left.a * left_weight + right.a * right_weight;
    let channel = |left_channel: f64, right_channel: f64| {
        if mixed_alpha == 0.0 {
            0.0
        } else {
            (left_channel * left.a * left_weight + right_channel * right.a * right_weight)
                / mixed_alpha
        }
    };
    Some(Rgba {
        r: channel(left.r, right.r),
        g: channel(left.g, right.g),
        b: channel(left.b, right.b),
        a: mixed_alpha * multiplier,
    })
}

fn split_mix_weight(value: &str) -> Option<(&str, Option<f64>)> {
    let value = value.trim();
    let mut depth = 0_u32;
    let mut last_space = None;
    for (index, ch) in value.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.checked_sub(1)?,
            _ if depth == 0 && ch.is_ascii_whitespace() => last_space = Some(index),
            _ => {}
        }
    }
    if depth != 0 {
        return None;
    }
    if let Some(index) = last_space {
        let suffix = value[index..].trim();
        if let Some(percent) = suffix.strip_suffix('%') {
            let color = value[..index].trim();
            return (!color.is_empty()).then_some((color, Some(parse_number(percent)? / 100.0)));
        }
    }
    Some((value, None))
}

fn split_top_level(value: &str, separator: char) -> Option<Vec<&str>> {
    let mut result = Vec::new();
    let mut depth = 0_u32;
    let mut start = 0;
    for (index, ch) in value.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.checked_sub(1)?,
            _ if ch == separator && depth == 0 => {
                result.push(value[start..index].trim());
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    if depth != 0 {
        return None;
    }
    result.push(value[start..].trim());
    Some(result)
}

fn parse_number(value: &str) -> Option<f64> {
    let value = value.parse::<f64>().ok()?;
    value.is_finite().then_some(value)
}

fn parse_percentage(value: &str) -> Option<f64> {
    Some(parse_number(value.strip_suffix('%')?)? / 100.0)
}

fn parse_lightness_100(value: &str) -> Option<f64> {
    if let Some(percent) = value.strip_suffix('%') {
        parse_number(percent)
    } else {
        parse_number(value)
    }
}

fn parse_lightness_one(value: &str) -> Option<f64> {
    if let Some(percent) = value.strip_suffix('%') {
        Some(parse_number(percent)? / 100.0)
    } else {
        parse_number(value)
    }
}

fn parse_axis(value: &str, percent_scale: f64) -> Option<f64> {
    if let Some(percent) = value.strip_suffix('%') {
        Some(parse_number(percent)? * percent_scale / 100.0)
    } else {
        parse_number(value)
    }
}

fn parse_angle(value: &str) -> Option<f64> {
    let (number, scale) = if let Some(value) = value.strip_suffix("grad") {
        (value, 0.9)
    } else if let Some(value) = value.strip_suffix("turn") {
        (value, 360.0)
    } else if let Some(value) = value.strip_suffix("rad") {
        (value, 180.0 / PI)
    } else {
        (value.strip_suffix("deg").unwrap_or(value), 1.0)
    };
    Some(parse_number(number)? * scale)
}

fn with_alpha(rgb: (f64, f64, f64), alpha: Option<&&str>) -> Option<Rgba> {
    Some(Rgba {
        r: rgb.0.clamp(0.0, 1.0),
        g: rgb.1.clamp(0.0, 1.0),
        b: rgb.2.clamp(0.0, 1.0),
        a: alpha.map_or(Some(1.0), |value| parse_alpha(value))?,
    })
}

fn lab_to_srgb(l: f64, a: f64, b: f64) -> (f64, f64, f64) {
    let f1 = (l + 16.0) / 116.0;
    let f0 = a / 500.0 + f1;
    let f2 = f1 - b / 200.0;
    let epsilon = 216.0 / 24_389.0;
    let kappa = 24_389.0 / 27.0;
    let inverse = |value: f64| {
        let cube = value.powi(3);
        if cube > epsilon {
            cube
        } else {
            (116.0 * value - 16.0) / kappa
        }
    };
    let x50 = inverse(f0) * 0.96422;
    let y50 = inverse(f1);
    let z50 = inverse(f2) * 0.82521;
    let xyz65 = (
        0.955_576_6 * x50 - 0.023_039_3 * y50 + 0.063_163_6 * z50,
        -0.028_289_5 * x50 + 1.009_941_6 * y50 + 0.021_007_7 * z50,
        0.012_298_2 * x50 - 0.020_483 * y50 + 1.329_909_8 * z50,
    );
    xyz_to_srgb(xyz65)
}

fn oklab_to_srgb(l: f64, a: f64, b: f64) -> (f64, f64, f64) {
    let lms = (
        (l + 0.396_337_777_4 * a + 0.215_803_757_3 * b).powi(3),
        (l - 0.105_561_345_8 * a - 0.063_854_172_8 * b).powi(3),
        (l - 0.089_484_177_5 * a - 1.291_485_548 * b).powi(3),
    );
    linear_to_srgb((
        4.076_741_662_1 * lms.0 - 3.307_711_591_3 * lms.1 + 0.230_969_929_2 * lms.2,
        -1.268_438_004_6 * lms.0 + 2.609_757_401_1 * lms.1 - 0.341_319_396_5 * lms.2,
        -0.004_196_086_3 * lms.0 - 0.703_418_614_7 * lms.1 + 1.707_614_701 * lms.2,
    ))
}

fn display_p3_to_srgb(p3: [f64; 3]) -> (f64, f64, f64) {
    let p3 = p3.map(srgb_to_linear);
    xyz_to_srgb((
        0.486_570_948_6 * p3[0] + 0.265_667_693_2 * p3[1] + 0.198_217_285_2 * p3[2],
        0.228_974_564_1 * p3[0] + 0.691_738_521_8 * p3[1] + 0.079_286_914_1 * p3[2],
        0.045_113_381_9 * p3[1] + 1.043_944_368_9 * p3[2],
    ))
}

fn xyz_to_srgb(xyz: (f64, f64, f64)) -> (f64, f64, f64) {
    linear_to_srgb((
        3.240_6 * xyz.0 - 1.537_2 * xyz.1 - 0.498_6 * xyz.2,
        -0.968_9 * xyz.0 + 1.875_8 * xyz.1 + 0.041_5 * xyz.2,
        0.055_7 * xyz.0 - 0.204 * xyz.1 + 1.057 * xyz.2,
    ))
}

fn srgb_to_linear(value: f64) -> f64 {
    if value.abs() <= 0.04045 {
        value / 12.92
    } else {
        value.signum() * ((value.abs() + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(rgb: (f64, f64, f64)) -> (f64, f64, f64) {
    let convert = |value: f64| {
        if value.abs() <= 0.003_130_8 {
            12.92 * value
        } else {
            value.signum() * (1.055 * value.abs().powf(1.0 / 2.4) - 0.055)
        }
    };
    (convert(rgb.0), convert(rgb.1), convert(rgb.2))
}

fn format_rgba(color: Rgba) -> String {
    let channel = |value: f64| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    let (r, g, b, a) = (
        channel(color.r),
        channel(color.g),
        channel(color.b),
        channel(color.a),
    );
    if a == u8::MAX {
        format!("#{r:02x}{g:02x}{b:02x}")
    } else {
        format!("#{r:02x}{g:02x}{b:02x}{a:02x}")
    }
}

fn parse_named(value: &str) -> Option<Rgba> {
    if value == "transparent" {
        return Some(Rgba {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        });
    }
    let hex = match value {
        "aliceblue" => "f0f8ff",
        "antiquewhite" => "faebd7",
        "aqua" | "cyan" => "00ffff",
        "aquamarine" => "7fffd4",
        "azure" => "f0ffff",
        "beige" => "f5f5dc",
        "bisque" => "ffe4c4",
        "black" => "000000",
        "blanchedalmond" => "ffebcd",
        "blue" => "0000ff",
        "blueviolet" => "8a2be2",
        "brown" => "a52a2a",
        "burlywood" => "deb887",
        "cadetblue" => "5f9ea0",
        "chartreuse" => "7fff00",
        "chocolate" => "d2691e",
        "coral" => "ff7f50",
        "cornflowerblue" => "6495ed",
        "cornsilk" => "fff8dc",
        "crimson" => "dc143c",
        "darkblue" => "00008b",
        "darkcyan" => "008b8b",
        "darkgoldenrod" => "b8860b",
        "darkgray" | "darkgrey" => "a9a9a9",
        "darkgreen" => "006400",
        "darkkhaki" => "bdb76b",
        "darkmagenta" => "8b008b",
        "darkolivegreen" => "556b2f",
        "darkorange" => "ff8c00",
        "darkorchid" => "9932cc",
        "darkred" => "8b0000",
        "darksalmon" => "e9967a",
        "darkseagreen" => "8fbc8f",
        "darkslateblue" => "483d8b",
        "darkslategray" | "darkslategrey" => "2f4f4f",
        "darkturquoise" => "00ced1",
        "darkviolet" => "9400d3",
        "deeppink" => "ff1493",
        "deepskyblue" => "00bfff",
        "dimgray" | "dimgrey" => "696969",
        "dodgerblue" => "1e90ff",
        "firebrick" => "b22222",
        "floralwhite" => "fffaf0",
        "forestgreen" => "228b22",
        "fuchsia" | "magenta" => "ff00ff",
        "gainsboro" => "dcdcdc",
        "ghostwhite" => "f8f8ff",
        "gold" => "ffd700",
        "goldenrod" => "daa520",
        "gray" | "grey" => "808080",
        "green" => "008000",
        "greenyellow" => "adff2f",
        "honeydew" => "f0fff0",
        "hotpink" => "ff69b4",
        "indianred" => "cd5c5c",
        "indigo" => "4b0082",
        "ivory" => "fffff0",
        "khaki" => "f0e68c",
        "lavender" => "e6e6fa",
        "lavenderblush" => "fff0f5",
        "lawngreen" => "7cfc00",
        "lemonchiffon" => "fffacd",
        "lightblue" => "add8e6",
        "lightcoral" => "f08080",
        "lightcyan" => "e0ffff",
        "lightgoldenrodyellow" => "fafad2",
        "lightgray" | "lightgrey" => "d3d3d3",
        "lightgreen" => "90ee90",
        "lightpink" => "ffb6c1",
        "lightsalmon" => "ffa07a",
        "lightseagreen" => "20b2aa",
        "lightskyblue" => "87cefa",
        "lightslategray" | "lightslategrey" => "778899",
        "lightsteelblue" => "b0c4de",
        "lightyellow" => "ffffe0",
        "lime" => "00ff00",
        "limegreen" => "32cd32",
        "linen" => "faf0e6",
        "maroon" => "800000",
        "mediumaquamarine" => "66cdaa",
        "mediumblue" => "0000cd",
        "mediumorchid" => "ba55d3",
        "mediumpurple" => "9370db",
        "mediumseagreen" => "3cb371",
        "mediumslateblue" => "7b68ee",
        "mediumspringgreen" => "00fa9a",
        "mediumturquoise" => "48d1cc",
        "mediumvioletred" => "c71585",
        "midnightblue" => "191970",
        "mintcream" => "f5fffa",
        "mistyrose" => "ffe4e1",
        "moccasin" => "ffe4b5",
        "navajowhite" => "ffdead",
        "navy" => "000080",
        "oldlace" => "fdf5e6",
        "olive" => "808000",
        "olivedrab" => "6b8e23",
        "orange" => "ffa500",
        "orangered" => "ff4500",
        "orchid" => "da70d6",
        "palegoldenrod" => "eee8aa",
        "palegreen" => "98fb98",
        "paleturquoise" => "afeeee",
        "palevioletred" => "db7093",
        "papayawhip" => "ffefd5",
        "peachpuff" => "ffdab9",
        "peru" => "cd853f",
        "pink" => "ffc0cb",
        "plum" => "dda0dd",
        "powderblue" => "b0e0e6",
        "purple" => "800080",
        "rebeccapurple" => "663399",
        "red" => "ff0000",
        "rosybrown" => "bc8f8f",
        "royalblue" => "4169e1",
        "saddlebrown" => "8b4513",
        "salmon" => "fa8072",
        "sandybrown" => "f4a460",
        "seagreen" => "2e8b57",
        "seashell" => "fff5ee",
        "sienna" => "a0522d",
        "silver" => "c0c0c0",
        "skyblue" => "87ceeb",
        "slateblue" => "6a5acd",
        "slategray" | "slategrey" => "708090",
        "snow" => "fffafa",
        "springgreen" => "00ff7f",
        "steelblue" => "4682b4",
        "tan" => "d2b48c",
        "teal" => "008080",
        "thistle" => "d8bfd8",
        "tomato" => "ff6347",
        "turquoise" => "40e0d0",
        "violet" => "ee82ee",
        "wheat" => "f5deb3",
        "white" => "ffffff",
        "whitesmoke" => "f5f5f5",
        "yellow" => "ffff00",
        "yellowgreen" => "9acd32",
        _ => return None,
    };
    parse_hex(hex)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_and_all_named_families() {
        assert_eq!(parse_css_color("#FA3").as_deref(), Some("#ffaa33"));
        assert_eq!(parse_css_color("#0f08").as_deref(), Some("#00ff0088"));
        assert_eq!(parse_css_color("rebeccapurple").as_deref(), Some("#663399"));
        assert_eq!(parse_css_color("DarkSlateGrey").as_deref(), Some("#2f4f4f"));
        assert_eq!(parse_css_color("transparent").as_deref(), Some("#00000000"));
        assert!(parse_css_color("currentcolor").is_none());
    }

    #[test]
    fn parses_legacy_and_modern_rgb_hsl() {
        assert_eq!(
            parse_css_color("rgb(255, 0, 0)").as_deref(),
            Some("#ff0000")
        );
        assert_eq!(
            parse_css_color("rgb(100% 0% 0% / 50%)").as_deref(),
            Some("#ff000080")
        );
        assert_eq!(
            parse_css_color("rgba(0,0,255,.25)").as_deref(),
            Some("#0000ff40")
        );
        assert_eq!(
            parse_css_color("hsl(.5turn 100% 50%)").as_deref(),
            Some("#00ffff")
        );
        assert_eq!(
            parse_css_color("hsla(3.141592653589793rad,100%,50%,.5)").as_deref(),
            Some("#00ffff80")
        );
        assert_eq!(
            parse_css_color("hsl(200grad 100% 50%)").as_deref(),
            Some("#00ffff")
        );
    }

    #[test]
    fn parses_modern_color_spaces() {
        assert_eq!(
            parse_css_color("hwb(0 0% 0% / 25%)").as_deref(),
            Some("#ff000040")
        );
        assert_eq!(parse_css_color("lab(100% 0 0)").as_deref(), Some("#ffffff"));
        assert_eq!(
            parse_css_color("lch(0% 0 20deg)").as_deref(),
            Some("#000000")
        );
        assert_eq!(
            parse_css_color("oklab(100% 0 0)").as_deref(),
            Some("#ffffff")
        );
        assert_eq!(
            parse_css_color("oklch(0% 0 1turn)").as_deref(),
            Some("#000000")
        );
        assert_eq!(
            parse_css_color("color(srgb 1 0 0 / .5)").as_deref(),
            Some("#ff000080")
        );
        assert_eq!(
            parse_css_color("color(display-p3 1 0 0)").as_deref(),
            Some("#ff0000")
        );
    }

    #[test]
    fn mixes_srgb_colors_with_alpha() {
        assert_eq!(
            parse_css_color("color-mix(in srgb, red, blue)").as_deref(),
            Some("#800080")
        );
        assert_eq!(
            parse_css_color("color-mix(in srgb, red 25%, blue)").as_deref(),
            Some("#4000bf")
        );
        assert_eq!(
            parse_css_color("color-mix(in srgb, rgb(255 0 0 / 50%) 50%, transparent 50%)")
                .as_deref(),
            Some("#ff000040")
        );
    }
}
