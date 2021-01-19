//! `ttf-parser` utils.

/// A bounding box.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
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

    /// Returns the bbox height.
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

/// A glyph outline.
#[derive(Debug, Clone)]
pub struct Outline {
    bbox: std::cell::Cell<Option<BBox>>,
    cff: bool,
    contours: Vec<Contour>,
}

impl Outline {
    /// Returns a new outline or `None` when the glyph has no outline or on error.
    pub fn new(face: &ttf_parser::Face, glyph_id: ttf_parser::GlyphId) -> Option<Self> {
        let mut outline = Outline {
            bbox: std::cell::Cell::new(None),
            cff: face.has_table(ttf_parser::TableName::CompactFontFormat)
                || face.has_table(ttf_parser::TableName::CompactFontFormat2),
            contours: Vec::new(),
        };
        let mut outline_builder = OutlineBuilder::new(&mut outline);
        let _ = face.outline_glyph(glyph_id, &mut outline_builder)?;
        Some(outline)
    }

    /// Returns the outline bounding box.
    pub fn bbox(&self) -> BBox {
        if let Some(bbox) = self.bbox.get() {
            bbox
        } else {
            let mut bbox = BBox::default();
            for p in self.contours.iter().flat_map(|c| &c.points) {
                bbox.extend_by(p.x, p.y);
            }

            self.bbox.set(Some(bbox));
            bbox
        }
    }

    /// Embolden the outline.
    pub fn embolden(&mut self, strength: f32) {
        self.bbox.set(None);
        for c in &mut self.contours {
            let num_points = c.points.len();
            if num_points == 0 {
                continue;
            }

            let closed = num_points > 1 && c.points.last() == c.points.first();
            let last = if closed {
                num_points - 2
            } else {
                num_points - 1
            };

            let mut in_pt = Point::default();
            let mut in_len = 0f32;

            let mut anchor_pt = Point::default();
            let mut anchor_len = 0f32;

            let mut i = last;
            let mut j = 0;
            let mut k: Option<usize> = None;
            while i != j && Some(i) != k {
                let (out_pt, out_len) = if Some(j) != k {
                    let x = c.points[j].x - c.points[i].x;
                    let y = c.points[j].y - c.points[i].y;
                    let len = (x * x + y * y).sqrt();
                    if len != 0.0 {
                        (Point::new(x / len, y / len), len)
                    } else {
                        j = if j < last { j + 1 } else { 0 };
                        continue;
                    }
                } else {
                    (anchor_pt, anchor_len)
                };

                if in_len != 0.0 {
                    if k.is_none() {
                        k = Some(i);
                        anchor_pt = in_pt;
                        anchor_len = in_len;
                    }

                    let d = (in_pt.x * out_pt.x) + (in_pt.y * out_pt.y);
                    let shift_pt = if d > -0.9375 {
                        let d = d + 1.0;
                        let mut q = out_pt.x * in_pt.y - out_pt.y * in_pt.x;
                        if !self.cff {
                            q = -q;
                        }

                        let len = in_len.min(out_len);
                        let (x, y) = if self.cff {
                            (in_pt.y + out_pt.y, -(in_pt.x + out_pt.x))
                        } else {
                            (-(in_pt.y + out_pt.y), in_pt.x + out_pt.x)
                        };
                        if (strength * q) <= (len * d) {
                            Point::new(x * strength / d, y * strength / d)
                        } else {
                            Point::new(x * len / q, y * len / q)
                        }
                    } else {
                        Point::default()
                    };

                    while i != j {
                        let pt = &mut c.points[i];
                        pt.x += strength + shift_pt.x;
                        pt.y += strength + shift_pt.y;
                        i = if i < last { i + 1 } else { 0 };
                    }
                } else {
                    i = j;
                }

                in_pt = out_pt;
                in_len = out_len;
                j = if j < last { j + 1 } else { 0 };
            }

            if closed {
                let first = &c.points[0];
                c.points[num_points - 1] = *first;
            }
        }
    }

    /// Slant the outline.
    pub fn oblique(&mut self, x_skew: f32) {
        self.bbox.set(None);
        for c in &mut self.contours {
            for p in &mut c.points {
                if p.y != 0.0 {
                    p.x += p.y * x_skew;
                }
            }
        }
    }

    /// Emit the outline segments.
    pub fn emit(&self, builder: &mut dyn ttf_parser::OutlineBuilder) {
        let mut points = self.contours.iter().flat_map(|c| &c.points);
        for v in self.contours.iter().flat_map(|c| &c.verbs) {
            match v {
                PathVerb::MoveTo => {
                    let p = points.next().unwrap();
                    builder.move_to(p.x, p.y);
                }
                PathVerb::LineTo => {
                    let p = points.next().unwrap();
                    builder.line_to(p.x, p.y);
                }
                PathVerb::QuadTo => {
                    let p1 = points.next().unwrap();
                    let p = points.next().unwrap();
                    builder.quad_to(p1.x, p1.y, p.x, p.y);
                }
                PathVerb::CurveTo => {
                    let p1 = points.next().unwrap();
                    let p2 = points.next().unwrap();
                    let p = points.next().unwrap();
                    builder.curve_to(p1.x, p1.y, p2.x, p2.y, p.x, p.y);
                }
                PathVerb::Close => {
                    builder.close();
                }
            }
        }
    }
}

#[derive(Debug, Default, Clone)]
struct Contour {
    verbs: Vec<PathVerb>,
    points: Vec<Point>,
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
        if self.current_contour > self.outline.contours.len() {
            self.outline.contours.push(Contour::default());
        }

        &mut self.outline.contours[self.current_contour - 1]
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
        self.current_contour().verbs.push(PathVerb::Close);
        self.current_contour += 1;
    }
}
