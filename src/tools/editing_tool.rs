use crate::Shaper;
use crate::tool::Tool;
use eframe::egui::{self, Align, Context, Layout, Painter, Pos2, Response, Vec2};
use kurbo::{Nearest, ParamCurveNearest, Point};

/// A small enum to remember what the user clicked on (and is now dragging).
/// Remember what we’re dragging: either one control handle (and its neighbors),
/// or the entire segment.

#[derive(PartialEq)]
enum MoveMode {
    MovePoint,
    MoveControlPoints,
}

enum ActiveDrag {
    ControlPoint {
        // this variable is to track the index of the
        // selected shape in the Shapes list in the
        // app struct
        // app.shapes[i]
        shape_idx: usize,
        // this variable is to track the index of
        // the selected bezier inside the selected shape.
        // shape.bezier[i]
        bez_idx: usize,
        // this variable is to track the index of the
        // point being clicked on on the bezier
        // (a point from the 4 points of a segment of a shape)
        // bezier.p0..=p3
        ctrl_idx: usize, // 0..=3
        orig_pos: Point,
    },
    CurveSegment {
        shape_idx: usize,
        bez_idx: usize,
        orig_p0: Point,
        orig_p1: Point,
        orig_p2: Point,
        orig_p3: Point,
    },
    None,
}

impl Default for ActiveDrag {
    fn default() -> Self {
        ActiveDrag::None
    }
}

pub struct EditingTool {
    /// remember the pointer position at the start of drag
    drag_start: Option<Pos2>,

    /// When dragging, remember exactly what control/segment is “active”
    active_drag: ActiveDrag,

    move_mode: MoveMode,
}

impl EditingTool {
    pub fn new() -> Self {
        EditingTool {
            drag_start: None,

            active_drag: ActiveDrag::None,
            // selected_shape_index: -1,
            // selected_bezier_index: -1,
            move_mode: MoveMode::MovePoint,
        }
    }
}

impl Tool for EditingTool {
    fn handle_input(&mut self, ctx: &Context, response: &Response, app: &mut Shaper) {
        if let Some(pointer_pos) = response.hover_pos() {
            let scroll_delta = ctx.input(|i| i.smooth_scroll_delta.y);
            if scroll_delta != 0.0 {
                // convert world position before zoom
                let old_world_pos = app.screen_to_world(pointer_pos);

                // apply zoom
                let zoom_delta = (scroll_delta * 0.009).exp();
                app.zoom *= zoom_delta;
                app.zoom = app.zoom.clamp(app.min_zoom, app.max_zoom);

                // convert world position after zoom
                let new_world_pos = app.screen_to_world(pointer_pos);

                // adjust pan offset to keep pointer position stable
                // convert Pos2 difference directly to Vec2
                let world_delta = Vec2::new(
                    new_world_pos.x - old_world_pos.x,
                    new_world_pos.y - old_world_pos.y,
                );
                app.pan_offset += world_delta * app.zoom;

                // percentage calculation:
                app.calc_zoom_level();
            }
        }

        if response.drag_started() {
            if let Some(mut pos2) = response.interact_pointer_pos() {
                pos2 = app.screen_to_world(pos2);
                self.drag_start = Some(pos2);
                let mouse = Point::new(pos2.x as f64, pos2.y as f64);

                // iterate shapes → beziers for control-point or curve hit
                let mut found = ActiveDrag::None;
                'outer: for (shape_idx, shape) in app.shapes.iter().enumerate() {
                    // tolerance for point and curve (world space)
                    let tol_point_ws: f64 = app.handle_radius as f64;
                    let tol_curve_ws: f64 = app.overlay_beziers_thickness as f64;

                    for (bez_idx, bez) in shape.beziers.iter().enumerate() {
                        // control handles (p0..p3)
                        let handles = [bez.p0, bez.p1, bez.p2, bez.p3];
                        for (ctrl_i, &pt) in handles.iter().enumerate() {
                            let dx = mouse.x - pt.x;
                            let dy = mouse.y - pt.y;
                            if (dx * dx + dy * dy).sqrt() <= tol_point_ws {
                                found = ActiveDrag::ControlPoint {
                                    shape_idx,
                                    bez_idx,
                                    ctrl_idx: ctrl_i,
                                    orig_pos: pt,
                                };
                                break 'outer;
                            }
                        }

                        // 2b) curve‐itself: use `nearest(...)` and compare distance_sq
                        // Kurbo’s `nearest(...)` returns a `Nearest { distance_sq, t }`
                        // supply a small “accuracy” (1e-6) to get a precise t, then check if
                        // dist² ≤ tol²:
                        let nearest: Nearest = bez.nearest(mouse, 1e-6);
                        if nearest.distance_sq <= tol_curve_ws * tol_curve_ws {
                            // Click is ≤ tol pixels from the curve
                            found = ActiveDrag::CurveSegment {
                                shape_idx,
                                bez_idx,
                                orig_p0: bez.p0,
                                orig_p1: bez.p1,
                                orig_p2: bez.p2,
                                orig_p3: bez.p3,
                            };
                            break 'outer;
                        }
                    }
                }

                self.active_drag = found;
            }
        }

        // while dragging: compute delta and update either
        // the single handle (+ neighbor) or full segment
        if response.dragged() {
            if let (Some(start_pos), Some(mut curr_pos)) =
                (self.drag_start, response.interact_pointer_pos())
            {
                // start_pos is using self.drag_start which is already
                // converted to world coordinates from drag_start
                // but curr_pos needs converting of course
                curr_pos = app.screen_to_world(curr_pos);

                let delta_screen = curr_pos - start_pos;
                let dx = delta_screen.x as f64;
                let dy = delta_screen.y as f64;
                let delta = Point::new(dx, dy);

                match &self.active_drag {
                    ActiveDrag::ControlPoint {
                        shape_idx,
                        bez_idx,
                        ctrl_idx,
                        orig_pos,
                    } => {
                        let shape = &mut app.shapes[*shape_idx];
                        // mutable reference to the segment we clicked
                        let bez = &mut shape.beziers[*bez_idx];
                        let new_pt = Point::new(orig_pos.x + delta.x, orig_pos.y + delta.y);

                        // move the chosen control handle
                        match ctrl_idx {
                            0 => {
                                // move this start‐point
                                bez.p0 = new_pt;
                                // also update the previous segment’s p3, if it exists
                                if *bez_idx > 0 {
                                    let prev = &mut shape.beziers[*bez_idx - 1];
                                    prev.p3 = new_pt;
                                }
                            }
                            1 => {
                                // move this first handle
                                bez.p1 = new_pt;
                            }
                            2 => {
                                // move this second handle
                                bez.p2 = new_pt;
                            }
                            3 => {
                                // move this end‐point
                                bez.p3 = new_pt;
                                // also update the next segment’s p0, if it exists
                                if *bez_idx + 1 < shape.beziers.len() {
                                    let next = &mut shape.beziers[*bez_idx + 1];
                                    next.p0 = new_pt;
                                }
                            }
                            _ => unreachable!(),
                        }
                    }

                    ActiveDrag::CurveSegment {
                        shape_idx,
                        bez_idx,
                        orig_p0,
                        orig_p1,
                        orig_p2,
                        orig_p3,
                    } => {
                        let shape = &mut app.shapes[*shape_idx];

                        // compute the new positions first:
                        let new_p0 = Point::new(orig_p0.x + delta.x, orig_p0.y + delta.y);
                        let new_p1 = Point::new(orig_p1.x + delta.x, orig_p1.y + delta.y);
                        let new_p2 = Point::new(orig_p2.x + delta.x, orig_p2.y + delta.y);
                        let new_p3 = Point::new(orig_p3.x + delta.x, orig_p3.y + delta.y);

                        // mutably borrow the “current” segment, write all
                        // four points, then drop it immediately.
                        {
                            let bez = &mut shape.beziers[*bez_idx];
                            bez.p0 = new_p0;
                            bez.p1 = new_p1;
                            bez.p2 = new_p2;
                            bez.p3 = new_p3;
                        } // <-- `bez` goes out of scope/dropped here

                        // now that `bez` is dropped, it's safe to borrow neighbors:
                        if *bez_idx > 0 {
                            let prev = &mut shape.beziers[*bez_idx - 1];
                            prev.p3 = new_p0;
                        }
                        if *bez_idx + 1 < shape.beziers.len() {
                            let next = &mut shape.beziers[*bez_idx + 1];
                            next.p0 = new_p3;
                        }
                    }

                    ActiveDrag::None => {
                        // clicking/dragging empty space—do nothing
                    }
                }
            }
        }

        // on drag end, clear state
        if response.drag_stopped() {
            self.drag_start = None;
            self.active_drag = ActiveDrag::None;
        }
    }

    fn paint(&mut self,  _ctx: &Context, _painter: &Painter, _app: &Shaper) {}

    fn tool_ui(&mut self, ctx: &Context, _app: &mut Shaper) {
        egui::TopBottomPanel::top("edit settings")
            .resizable(false)
            .show(ctx, |ui| {
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    // let move_segment_checkbox = egui::Checkbox::new(move_segment, "Move Segment").indeterminate(false);

                    ui.radio_value(&mut self.move_mode, MoveMode::MovePoint, "Move Point");
                    ui.radio_value(&mut self.move_mode, MoveMode::MoveControlPoints, "Move Control Points");
                });
            });
    }
}
