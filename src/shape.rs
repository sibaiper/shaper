use eframe::egui::{
    Color32, Painter, Pos2, Stroke,
    epaint::{CubicBezierShape, PathShape},
};
use kurbo::{CubicBez, Point as KPoint, Vec2};
use simplify_rs::{Point as SrPoint, simplify};

#[derive(Clone)]
pub struct Shape {
    /// raw points collected during the current drag
    pub current_stroke: Vec<Pos2>,

    /// history of all raw strokes, for later re-fit again
    pub raw_strokes: Vec<Vec<Pos2>>,

    /// All “fitted” Bézier segments (one CubicBez per segment)
    pub beziers: Vec<CubicBez>,

    /// Minimum pixel distance before we sample a new raw point
    pub sample_tol: f32,

    pub thickness: f64,
}

impl Shape {
    pub fn new() -> Self {
        Shape {
            current_stroke: Vec::new(),
            raw_strokes: Vec::new(),
            beziers: Vec::new(),
            sample_tol: 2.0,
            thickness: 10.0,
        }
    }

    /// take a completed raw stroke (`&[Pos2]`), run `simplify-rs` on it,
    /// and append each resulting `[SrPoint;4]` as a `kurbo::CubicBez`.
    pub fn fit_curve_and_store(&mut self, raw: &[Pos2], bzr_tol: f64) {
        // Convert Pos2 → simplify_rs::Point (which is { x: f64, y: f64 })
        let sr_points: Vec<SrPoint> = raw
            .iter()
            .map(|&p| SrPoint {
                x: p.x as f64,
                y: p.y as f64,
            })
            .collect();

        if sr_points.len() < 2 {
            return; // nothing to fit
        }

        // tolerance (in screen units) for the maximum deviation
        let tol = bzr_tol;

        // → Vec<[SrPoint;4]>: each [P0,P1,P2,P3] is a cubic in simplify-rs
        let flat: Vec<SrPoint> = simplify(&sr_points, tol);

        // turn flat Vec<SrPoint> into Vec<[SrPoint; 4]>
        let beziers_rs: Vec<[SrPoint; 4]> = flat
            .chunks_exact(4)
            .map(|chunk| {
                [
                    chunk[0].clone(),
                    chunk[1].clone(),
                    chunk[2].clone(),
                    chunk[3].clone(),
                ]
            })
            .collect();

        // convert each [SrPoint;4] → kurbo::CubicBez, then store it
        for bez in beziers_rs {
            let (p0, p1, p2, p3) = (
                // cast each simplify_rs::Point back into egui::Pos2 (f32)
                Pos2::new(bez[0].x as f32, bez[0].y as f32),
                Pos2::new(bez[1].x as f32, bez[1].y as f32),
                Pos2::new(bez[2].x as f32, bez[2].y as f32),
                Pos2::new(bez[3].x as f32, bez[3].y as f32),
            );

            // build a kurbo::CubicBez (fields are (p0,p1,p2,p3), each a kurbo::Point)
            let seg = CubicBez {
                p0: KPoint::new(p0.x as f64, p0.y as f64),
                p1: KPoint::new(p1.x as f64, p1.y as f64),
                p2: KPoint::new(p2.x as f64, p2.y as f64),
                p3: KPoint::new(p3.x as f64, p3.y as f64),
            };
            self.beziers.push(seg);
        }
    }

    /// method to be called whenever `self.bezier_tolerance` changes.
    /// it throws away all old Béziers and re‐creates them from every raw stroke.
    pub fn refit_all_strokes(&mut self, bzr_tol: f64) {
        self.beziers.clear();
        let raw_strokes: Vec<Vec<Pos2>> = self.raw_strokes.iter().cloned().collect();
        for raw in &raw_strokes {
            self.fit_curve_and_store(raw, bzr_tol);
        }
    }

    pub fn draw_beziers(&self, painter: &Painter, app: &crate::Shaper) {
        // we'll accumulate _all_ screen‐space points here:
        let mut all_points: Vec<Pos2> = Vec::new();

        // 1) loop each fitted CubicBez segment:
        for (seg_idx, bzr) in self.beziers.iter().enumerate() {
            // 1a) convert the four Kurbo control points into screen‐space Pos2:
            let (w0, w1, w2, w3) = (bzr.p0, bzr.p1, bzr.p2, bzr.p3);
            let s0 = app.world_to_screen(Pos2::new(w0.x as f32, w0.y as f32));
            let s1 = app.world_to_screen(Pos2::new(w1.x as f32, w1.y as f32));
            let s2 = app.world_to_screen(Pos2::new(w2.x as f32, w2.y as f32));
            let s3 = app.world_to_screen(Pos2::new(w3.x as f32, w3.y as f32));

            // 1b) build a temporary CubicBezierShape:
            let bez_shape = CubicBezierShape {
                points: [s0, s1, s2, s3],
                closed: false,
                stroke: Default::default(),
                fill: Color32::TRANSPARENT,
            };

            // 1c) flatten this one cubic into straight‐line PathShapes:
            //     - tol: Some(0.5) means “max error ~0.5px” (tweak for more/less fidelity)
            //     - eps:  None   means “use the default epsilon internally”
            let tol: Option<f32> = Some(0.5);
            let eps: Option<f32> = None;
            let mut sub_paths: Vec<PathShape> = bez_shape.to_path_shapes(tol, eps);

            // 1d) each `PathShape` contains a `Vec<Pos2>` in `.points`.
            //     if there are multiple PathShapes (rare—only when the curve intersects itself),
            //     we stitch them all together in order. But we must avoid duplicating the joint
            //     point between segment N and segment N+1. So:
            for path_shape in sub_paths.drain(..) {
                if seg_idx > 0 {
                    // for every segment after the first, drop the very first point to avoid duplication:
                    if let Some((_, tail)) = path_shape.points.split_first() {
                        all_points.extend_from_slice(tail);
                    }
                } else {
                    // for the first segment, take all points:
                    all_points.extend(path_shape.points.iter());
                }
            }
        }

        // now `all_points` is one continuous polyline in screen space. Stroke it once:
        let stroke_width = (self.thickness * app.zoom as f64) as f32;
        let stroke = Stroke::new(stroke_width, Color32::BLACK);
        painter.line(all_points, stroke);
    }

    /// draw the *raw* strokes in thin green
    pub fn draw_raw(&self, painter: &Painter, app: &crate::Shaper) {
        for segment in &self.raw_strokes {
            for window in segment.windows(2) {
                let a = app.world_to_screen(window[0]);
                let b = app.world_to_screen(window[1]);
                painter.line_segment([a, b], Stroke::new(1.0 * app.zoom, Color32::GREEN));
            }
        }
    }

    /// draw control‐point handles (filled circles & red connecting lines)
    pub fn draw_handles(&self, painter: &Painter, app: &crate::Shaper) {
        let handle_radius = app.handle_radius * app.zoom;
        let p_color = app.p_color;
        let cp_color = app.cp_color;
        for bez in &self.beziers {
            let k0 = bez.p0;
            let k1 = bez.p1;
            let k2 = bez.p2;
            let k3 = bez.p3;
            let p0 = app.world_to_screen(Pos2::new(k0.x as f32, k0.y as f32));
            let p1 = app.world_to_screen(Pos2::new(k1.x as f32, k1.y as f32));
            let p2 = app.world_to_screen(Pos2::new(k2.x as f32, k2.y as f32));
            let p3 = app.world_to_screen(Pos2::new(k3.x as f32, k3.y as f32));

            painter.line_segment(
                [p0, p1],
                Stroke::new(app.handle_arm_thicknes * app.zoom, Color32::RED),
            );
            // painter.line_segment([p1, p2], Stroke::new(app.handle_arm_thicknes * app.zoom, Color32::RED)); // line connecting the 2 control points to one another (off for now)
            painter.line_segment(
                [p3, p2],
                Stroke::new(app.handle_arm_thicknes * app.zoom, Color32::RED),
            );
            painter.circle_filled(p0, handle_radius, p_color);
            painter.circle_filled(p1, handle_radius, cp_color);
            painter.circle_filled(p2, handle_radius, cp_color);
            painter.circle_filled(p3, handle_radius, p_color);
        }
    }

    pub fn draw_overlay_beziers(&self, painter: &Painter, app: &crate::Shaper) {
        // we'll accumulate _all_ screen‐space points here:
        let mut all_points: Vec<Pos2> = Vec::new();

        // 1) loop each fitted CubicBez segment:
        for (seg_idx, bzr) in self.beziers.iter().enumerate() {
            // 1a) convert the four Kurbo control points into screen‐space Pos2:
            let (w0, w1, w2, w3) = (bzr.p0, bzr.p1, bzr.p2, bzr.p3);
            let s0 = app.world_to_screen(Pos2::new(w0.x as f32, w0.y as f32));
            let s1 = app.world_to_screen(Pos2::new(w1.x as f32, w1.y as f32));
            let s2 = app.world_to_screen(Pos2::new(w2.x as f32, w2.y as f32));
            let s3 = app.world_to_screen(Pos2::new(w3.x as f32, w3.y as f32));

            // 1b) build a temporary CubicBezierShape:
            let bez_shape = CubicBezierShape {
                points: [s0, s1, s2, s3],
                closed: false,
                stroke: Default::default(),
                fill: Color32::TRANSPARENT,
            };

            // 1c) flatten this one cubic into straight‐line PathShapes:
            //     - tol: Some(0.5) means “max error ~0.5px” (tweak for more/less fidelity)
            //     - eps:  None   means “use the default epsilon internally”
            let tol: Option<f32> = Some(0.5);
            let eps: Option<f32> = None;
            let mut sub_paths: Vec<PathShape> = bez_shape.to_path_shapes(tol, eps);

            // 1d) each `PathShape` contains a `Vec<Pos2>` in `.points`.
            //     if there are multiple PathShapes (rare—only when the curve intersects itself),
            //     we stitch them all together in order. But we must avoid duplicating the joint
            //     point between segment N and segment N+1. So:
            for path_shape in sub_paths.drain(..) {
                if seg_idx > 0 {
                    // for every segment after the first, drop the very first point to avoid duplication:
                    if let Some((_, tail)) = path_shape.points.split_first() {
                        all_points.extend_from_slice(tail);
                    }
                } else {
                    // for the first segment, take all points:
                    all_points.extend(path_shape.points.iter());
                }
            }
        }

        // now `all_points` is one continuous polyline in screen space. Stroke it once:
        let stroke_width = app.overlay_beziers_thickness * app.zoom;
        let stroke = Stroke::new(stroke_width, Color32::WHITE);
        painter.line(all_points, stroke);
    }
}

// keeping this for maybe later use if needed. previous implementations of the
// rendering algorithm used this function to make up a quad of 2 triangles.
// but that egui rasterises triangles drawn onto the painter, so the final
// shape was not smooth at all. Opted for the built in egui cubic bezier render method
#[allow(dead_code)]
fn bezier_tangent(bzr: CubicBez, t: f64) -> Vec2 {
    let u = 1.0_f64 - t;
    let tt = t * t;
    let uu = u * u;

    let p0 = bzr.p0;
    let p1 = bzr.p1;
    let p2 = bzr.p2;
    let p3 = bzr.p3;

    let mut tangent: Vec2 = Vec2 {
        x: -3.0 * uu * p0.x + 3.0 * uu * p1.x - 6.0 * u * t * p1.x + 6.0 * u * t * p2.x
            - 3.0 * tt * p2.x
            + 3.0 * tt * p3.x,
        y: -3.0 * uu * p0.y + 3.0 * uu * p1.y - 6.0 * u * t * p1.y + 6.0 * u * t * p2.y
            - 3.0 * tt * p2.y
            + 3.0 * tt * p3.y,
    };

    // normalize the tangent vector to get the direction
    let length: f64 = (tangent.x * tangent.x + tangent.y * tangent.y).sqrt();
    tangent.x /= length;
    tangent.y /= length;

    tangent
}
