//! Cubic paths shared by Lottie's shapes and clipping masks.
use serde_json::{json, Value};
use usvg::tiny_skia_path::{Path, PathSegment, Point};

#[derive(Default)]
struct Contour {
    vertices: Vec<[f32; 2]>,
    incoming: Vec<[f32; 2]>,
    outgoing: Vec<[f32; 2]>,
}

impl Contour {
    fn vertex(&mut self, p: Point) {
        self.vertices.push([p.x, p.y]);
        self.incoming.push([0.0, 0.0]);
        self.outgoing.push([0.0, 0.0]);
    }

    fn cubic(&mut self, c1: Point, c2: Point, end: Point) {
        let start = self.vertices.last().unwrap();
        *self.outgoing.last_mut().unwrap() = [c1.x - start[0], c1.y - start[1]];
        self.vertex(end);
        *self.incoming.last_mut().unwrap() = [c2.x - end.x, c2.y - end.y];
    }

    fn finish(mut self, closed: bool) -> Value {
        if closed && self.vertices.len() > 1 && self.vertices.first() == self.vertices.last() {
            self.vertices.pop();
            self.incoming[0] = self.incoming.pop().unwrap();
            self.outgoing.pop();
        }
        json!({"v":self.vertices, "i":self.incoming, "o":self.outgoing, "c":closed})
    }
}

pub(super) fn contours(path: &Path) -> Vec<Value> {
    let mut result = Vec::new();
    let mut contour = Contour::default();
    for segment in path.segments() {
        match segment {
            PathSegment::MoveTo(p) => {
                if !contour.vertices.is_empty() {
                    result.push(std::mem::take(&mut contour).finish(false));
                }
                contour.vertex(p);
            }
            PathSegment::LineTo(p) => contour.vertex(p),
            PathSegment::QuadTo(c, end) => {
                let start = contour.vertices.last().unwrap();
                let c1 = Point::from_xy(
                    start[0] + (c.x - start[0]) * 2.0 / 3.0,
                    start[1] + (c.y - start[1]) * 2.0 / 3.0,
                );
                let c2 = Point::from_xy(
                    end.x + (c.x - end.x) * 2.0 / 3.0,
                    end.y + (c.y - end.y) * 2.0 / 3.0,
                );
                contour.cubic(c1, c2, end);
            }
            PathSegment::CubicTo(c1, c2, end) => contour.cubic(c1, c2, end),
            PathSegment::Close => {
                if !contour.vertices.is_empty() {
                    result.push(std::mem::take(&mut contour).finish(true));
                }
            }
        }
    }
    if !contour.vertices.is_empty() {
        result.push(contour.finish(false));
    }
    result
}

pub(super) fn rounded_rect(rect: [f32; 4], radii: [f32; 4]) -> Value {
    let [x, y, w, h] = rect;
    let limit = w.min(h).max(0.0) / 2.0;
    let [tl, tr, br, bl] = radii.map(|r| r.clamp(0.0, limit));
    let k = 0.552_284_8;
    json!({"c":true,
        "v":[[x+tl,y],[x+w-tr,y],[x+w,y+tr],[x+w,y+h-br],
             [x+w-br,y+h],[x+bl,y+h],[x,y+h-bl],[x,y+tl]],
        "i":[[-tl*k,0],[0,0],[0,-tr*k],[0,0],[br*k,0],[0,0],[0,bl*k],[0,0]],
        "o":[[0,0],[tr*k,0],[0,0],[0,br*k],[0,0],[-bl*k,0],[0,0],[0,-tl*k]]})
}
