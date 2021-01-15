//! `ttf-parser` utils.

/// A bounding box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BBox {
    /// Minimum X coordinate.
    pub x_min: f32,
    /// Minimum Y coordinate.
    pub y_min: f32,
    /// Maximum X coordinate.
    pub x_max: f32,
    /// Maximum Y coordinate.
    pub y_max: f32,
}

impl BBox {
    /// Returns the bbox width.
    #[inline]
    pub fn width(&self) -> f32 {
        self.x_max - self.x_min
    }

    /// Returns bbox height.
    #[inline]
    pub fn height(&self) -> f32 {
        self.y_max - self.y_min
    }

    /// Extend the bbox.
    #[inline]
    pub fn extend_by(&mut self, x: f32, y: f32) {
        self.x_min = self.x_min.min(x);
        self.y_min = self.y_min.min(y);
        self.x_max = self.x_max.max(x);
        self.y_max = self.y_max.max(y);
    }
}

impl Default for BBox {
    fn default() -> Self {
        Self {
            x_min: f32::MAX,
            y_min: f32::MAX,
            x_max: f32::MIN,
            y_max: f32::MIN,
        }
    }
}

/// The embolden strength used in FreeType.
pub const FT_EMBOLDEN_STRENGTH: f32 = 20.0;

/// Emboldens a glyph outline and returns its tight bounding box.
pub fn embolden(
    face: &ttf_parser::Face,
    glyph_id: ttf_parser::GlyphId,
    builder: &mut dyn ttf_parser::OutlineBuilder,
    strength: f32,
) -> Option<BBox> {
    let mut outline = Outline::default();
    let mut outline_builder = OutlineBuilder::new(&mut outline);
    let _ = face.outline_glyph(glyph_id, &mut outline_builder)?;
    outline.embolden(strength);

    let mut bbox = BBox::default();
    let mut points = outline.0.iter().flat_map(|c| &c.points);
    for v in outline.0.iter().flat_map(|c| &c.verbs) {
        match v {
            PathVerb::MoveTo => {
                let p = points.next().unwrap();
                bbox.extend_by(p.x, p.y);
                builder.move_to(p.x, p.y);
            }
            PathVerb::LineTo => {
                let p = points.next().unwrap();
                bbox.extend_by(p.x, p.y);
                builder.line_to(p.x, p.y);
            }
            PathVerb::QuadTo => {
                let p1 = points.next().unwrap();
                let p = points.next().unwrap();
                bbox.extend_by(p1.x, p1.y);
                bbox.extend_by(p.x, p.y);
                builder.quad_to(p1.x, p1.y, p.x, p.y);
            }
            PathVerb::CurveTo => {
                let p1 = points.next().unwrap();
                let p2 = points.next().unwrap();
                let p = points.next().unwrap();
                bbox.extend_by(p1.x, p1.y);
                bbox.extend_by(p2.x, p2.y);
                bbox.extend_by(p.x, p.y);
                builder.curve_to(p1.x, p1.y, p2.x, p2.y, p.x, p.y);
            }
            PathVerb::Close => {
                builder.close();
            }
        }
    }

    Some(bbox)
}

#[derive(Debug, Clone, Copy)]
enum PathVerb {
    MoveTo,
    LineTo,
    QuadTo,
    CurveTo,
    Close,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
struct Point {
    x: f32,
    y: f32,
}

impl Point {
    #[inline]
    fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Default)]
struct Contour {
    verbs: Vec<PathVerb>,
    points: Vec<Point>,
}

#[derive(Debug, Default)]
struct Outline(Vec<Contour>);

impl Outline {
    fn embolden(&mut self, strength: f32) {
        for c in &mut self.0 {
            let last = c.points.len().saturating_sub(2);
            if last == 0 {
                continue;
            }

            let mut in_pt = Point::default();
            let mut in_len = 0f32;

            let mut anchor_pt = Point::default();
            let mut anchor_len = 0f32;

            let mut i = last;
            let mut j = 0;
            let mut k: Option<usize> = None;
            let advance = |x: &mut usize| {
                *x = if *x < last { *x + 1 } else { 0 };
            };

            while i != j && Some(i) != k {
                let mut out_pt = Point::default();
                let out_len = if Some(j) != k {
                    let x = c.points[j].x - c.points[i].x;
                    let y = c.points[j].y - c.points[i].y;
                    let len = (x * x + y * y).sqrt();
                    if len != 0.0 {
                        out_pt.x = x / len;
                        out_pt.y = y / len;
                        len
                    } else {
                        advance(&mut j);
                        continue;
                    }
                } else {
                    out_pt = anchor_pt;
                    anchor_len
                };

                if in_len != 0.0 {
                    if k == None {
                        k = Some(i);
                        anchor_pt = in_pt;
                        anchor_len = in_len;
                    }

                    let mut shift_pt = Point::default();
                    let d = (in_pt.x * out_pt.x) + (in_pt.y * out_pt.y);
                    if d > -0.9375 {
                        let d = d + 1.0;
                        shift_pt.x = -(in_pt.y + out_pt.y);
                        shift_pt.y = in_pt.x + out_pt.x;
                        let q = -((out_pt.x * in_pt.y) - (out_pt.y * in_pt.x));
                        let len = in_len.min(out_len);
                        if (strength * q) <= (len * d) {
                            shift_pt.x = (shift_pt.x * strength) / d;
                            shift_pt.y = (shift_pt.y * strength) / d;
                        } else {
                            shift_pt.x = (shift_pt.x * len) / q;
                            shift_pt.y = (shift_pt.y * len) / q;
                        }
                    }

                    while i != j {
                        let pt = &mut c.points[i];
                        pt.x += strength + shift_pt.x;
                        pt.y += strength + shift_pt.y;
                        advance(&mut i);
                    }
                } else {
                    i = j;
                }

                in_pt = out_pt;
                in_len = out_len;
                advance(&mut j);
            }

            let num_points = c.points.len();
            if num_points > 1 {
                let first = c.points[0];
                c.points[num_points - 1] = first;
            }
        }
    }
}

#[derive(Debug)]
struct OutlineBuilder<'a> {
    outline: &'a mut Outline,
    current_contour: usize,
}

impl<'a> OutlineBuilder<'a> {
    #[inline]
    fn new(outline: &'a mut Outline) -> Self {
        Self {
            outline,
            current_contour: 1,
        }
    }

    #[inline]
    fn current_contour(&mut self) -> &mut Contour {
        if self.current_contour > self.outline.0.len() {
            self.outline.0.push(Contour::default());
        }

        &mut self.outline.0[self.current_contour - 1]
    }
}

impl<'a> ttf_parser::OutlineBuilder for OutlineBuilder<'a> {
    fn move_to(&mut self, x: f32, y: f32) {
        let c = self.current_contour();
        c.verbs.push(PathVerb::MoveTo);
        c.points.push(Point::new(x, y));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let c = self.current_contour();
        c.verbs.push(PathVerb::LineTo);
        c.points.push(Point::new(x, y));
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let c = self.current_contour();
        c.verbs.push(PathVerb::QuadTo);
        c.points.push(Point::new(x1, y1));
        c.points.push(Point::new(x, y));
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let c = self.current_contour();
        c.verbs.push(PathVerb::CurveTo);
        c.points.push(Point::new(x1, y1));
        c.points.push(Point::new(x2, y2));
        c.points.push(Point::new(x, y));
    }

    fn close(&mut self) {
        let c = self.current_contour();
        let n = c.points.len();
        if n > 1 {
            debug_assert_eq!(c.points[0], c.points[n - 1]);
        }

        c.verbs.push(PathVerb::Close);
        self.current_contour += 1;
    }
}
