extern crate skia_safe;

mod canvas;
use canvas::Canvas;

use std::fs::File;
use std::io::Write;

fn main() {
    let mut canvas = Canvas::new(2560, 1280);
    canvas.scale(1.2, 1.2);
    canvas.move_to(36.0, 48.0);
    canvas.quad_to(660.0, 880.0, 1200.0, 360.0);
    canvas.translate(10.0, 10.0);
    canvas.set_line_width(20.0);
    canvas.stroke();
    canvas.save();
    canvas.move_to(30.0, 90.0);
    canvas.line_to(110.0, 20.0);
    canvas.line_to(240.0, 130.0);
    canvas.line_to(60.0, 130.0);
    canvas.line_to(190.0, 20.0);
    canvas.line_to(270.0, 90.0);
    canvas.fill();
    let d = canvas.data();
    let mut file = File::create("test.png").unwrap();
    let bytes = d.as_bytes();
    file.write_all(bytes).unwrap();
}
