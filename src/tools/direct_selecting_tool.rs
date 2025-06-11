use crate::HitTestResult;
use crate::PointId;
use crate::Shaper;
use crate::tool::Tool;
use eframe::egui::StrokeKind;
use eframe::egui::{Color32, Context, Painter, Pos2, Rect, Response, Stroke};

const DRAG_THRESHOLD: f64 = 5.0;

pub struct DirectSelectionTool {
    active_hit: HitTestResult,
    drag_start_world: Option<kurbo::Point>,
    is_marquee: bool,
    dragged_point_origins: Vec<(PointId, kurbo::Point)>,
}

impl DirectSelectionTool {
    pub fn new() -> Self {
        Self {
            active_hit: HitTestResult::None,
            drag_start_world: None,
            is_marquee: false,
            dragged_point_origins: Vec::new(),
        }
    }
}

impl Tool for DirectSelectionTool {
    fn handle_input(&mut self, ctx: &Context, response: &Response, app: &mut Shaper) {
        if response.drag_started() {
            if let Some(screen_pos) = response.interact_pointer_pos() {
                let world = app.screen_to_world(screen_pos);
                let hit = app.hit_test_all(kurbo::Point::new(world.x as f64, world.y as f64));
                match hit {
                    HitTestResult::ControlPoint {
                        shape_idx,
                        bez_idx,
                        ctrl_idx,
                        orig_pos,
                    } => {
                        let pid = PointId {
                            shape_idx,
                            bezier_idx: bez_idx,
                            ctrl_idx,
                        };
                        let shift = ctx.input(|i| i.modifiers.shift);
                        if shift {
                            // toggle selection membership
                            if app.selected_points.contains(&pid) {
                                app.selected_points.remove(&pid);
                                // since now deselected, we go to marquee mode
                                self.active_hit = HitTestResult::None;
                                self.is_marquee = true;
                                self.drag_start_world =
                                    Some(kurbo::Point::new(world.x as f64, world.y as f64));
                                self.dragged_point_origins.clear();
                            } else {
                                // add to selection, then drag all selected
                                app.selected_points.insert(pid);
                                self.active_hit = HitTestResult::ControlPoint {
                                    shape_idx,
                                    bez_idx,
                                    ctrl_idx,
                                    orig_pos,
                                };
                                self.is_marquee = false;
                                self.drag_start_world =
                                    Some(kurbo::Point::new(world.x as f64, world.y as f64));
                                // record origins of all selected points
                                self.dragged_point_origins.clear();
                                for &sel_pid in &app.selected_points {
                                    if let Some(orig_pt) = app.get_point_position(sel_pid) {
                                        self.dragged_point_origins.push((sel_pid, orig_pt));
                                    }
                                }
                            }
                        } else {
                            // no shift: select only this if not already sole selection
                            let already_sole = app.selected_points.len() == 1
                                && app.selected_points.contains(&pid);
                            if !already_sole {
                                app.selected_points.clear();
                                app.selected_points.insert(pid);
                            }
                            // now drag it (or drag the single selected)
                            self.active_hit = HitTestResult::ControlPoint {
                                shape_idx,
                                bez_idx,
                                ctrl_idx,
                                orig_pos,
                            };
                            self.is_marquee = false;
                            self.drag_start_world =
                                Some(kurbo::Point::new(world.x as f64, world.y as f64));
                            self.dragged_point_origins.clear();
                            for &sel_pid in &app.selected_points {
                                if let Some(orig_pt) = app.get_point_position(sel_pid) {
                                    self.dragged_point_origins.push((sel_pid, orig_pt));
                                }
                            }
                        }
                    }
                    _ => {
                        // clicked empty or non-point: marquee for points
                        let shift = ctx.input(|i| i.modifiers.shift);
                        if !shift {
                            app.selected_points.clear();
                        }
                        self.active_hit = HitTestResult::None;
                        self.is_marquee = true;
                        self.drag_start_world =
                            Some(kurbo::Point::new(world.x as f64, world.y as f64));
                        self.dragged_point_origins.clear();
                    }
                }
            }
        }

        if response.dragged() {
            if let (Some(start_world), Some(curr_screen)) =
                (self.drag_start_world, response.interact_pointer_pos())
            {
                let drag_curr_world: Pos2 = app.screen_to_world(curr_screen);

                // calc the distance moved
                let delta_x = drag_curr_world.x as f64 - start_world.x;
                let delta_y = drag_curr_world.y as f64 - start_world.y;
                let distance_moved = (delta_x.powi(2) + delta_y.powi(2)).sqrt();

                if DRAG_THRESHOLD > distance_moved {
                    let curr_world = app.screen_to_world(curr_screen);
                    if self.is_marquee {
                        let rect = Rect::from_two_pos(
                            Pos2::new(start_world.x as f32, start_world.y as f32),
                            curr_world,
                        );
                        app.select_points_in_rect(rect);
                    } else {
                        let delta = kurbo::Point::new(
                            curr_world.x as f64 - start_world.x,
                            curr_world.y as f64 - start_world.y,
                        );
                        for (pid, orig_pos) in &self.dragged_point_origins {
                            let new_pos =
                                kurbo::Point::new(orig_pos.x + delta.x, orig_pos.y + delta.y);
                            app.move_point_to(*pid, new_pos);
                        }
                    }
                }
            }
        }

        if response.drag_stopped() {
            self.active_hit = HitTestResult::None;
            self.drag_start_world = None;
            self.is_marquee = false;
            self.dragged_point_origins.clear();
        }

        if response.clicked() {
            if let Some(screen_pos) = response.interact_pointer_pos() {
                let world = app.screen_to_world(screen_pos);
                match app.hit_test_all(kurbo::Point::new(world.x as f64, world.y as f64)) {
                    HitTestResult::ControlPoint {
                        shape_idx,
                        bez_idx,
                        ctrl_idx,
                        ..
                    } => {
                        let pid = PointId {
                            shape_idx,
                            bezier_idx: bez_idx,
                            ctrl_idx,
                        };
                        let shift = ctx.input(|i| i.modifiers.shift);
                        if shift {
                            // toggle
                            if app.selected_points.contains(&pid) {
                                app.selected_points.remove(&pid);
                            } else {
                                app.selected_points.insert(pid);
                            }
                        } else {
                            app.selected_points.clear();
                            app.selected_points.insert(pid);
                        }
                    }
                    _ => {
                        let shift = ctx.input(|i| i.modifiers.shift);
                        if !shift {
                            app.selected_points.clear();
                        }
                    }
                }
            }
        }
    }

    fn paint(&mut self, ctx: &Context, painter: &Painter, app: &Shaper) {
        if self.is_marquee {
            if let (Some(start_world), Some(curr_screen)) = (
                self.drag_start_world,
                ctx.input(|i| i.pointer.interact_pos()),
            ) {
                // convert world positions to screen positions
                let start_screen =
                    app.world_to_screen(Pos2::new(start_world.x as f32, start_world.y as f32));

                let marquee_rect = Rect::from_two_pos(start_screen, curr_screen);

                if let (Ok(fill_color), Ok(stroke_color)) = (
                    Color32::from_hex("#9ED5F788"),
                    Color32::from_hex("#1F5FCFBB"),
                ) {
                    let stroke = Stroke::new(1.0, stroke_color);
                    painter.rect_filled(marquee_rect, 0.0, fill_color);
                    painter.rect_stroke(marquee_rect, 0.0, stroke, StrokeKind::Middle);
                }
            }
        }

        // hover highlight
        if let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) {
            let world = app.screen_to_world(pos);
            if let HitTestResult::ControlPoint {
                shape_idx,
                bez_idx,
                ctrl_idx,
                ..
            } = app.hit_test_all(kurbo::Point::new(world.x as f64, world.y as f64))
            {
                // app.paint_point_hover_outline(shape_idx, bez_idx, ctrl_idx, painter);
            }
        }
        // selected highlights
        for &pid in &app.selected_points {
            // app.paint_point_selected_outline(pid.shape_idx, pid.bezier_idx, pid.ctrl_idx, painter);
        }
    }

    fn tool_ui(&mut self, _ctx: &Context, _app: &mut Shaper) {}
}
