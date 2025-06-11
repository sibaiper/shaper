use crate::HitTestResult;
use crate::Shaper;
use crate::tool::Tool;
use eframe::egui::{Context, Painter, Response};

pub struct CurvatureTool {
    active_hit: HitTestResult,
    drag_start_world: Option<kurbo::Point>,
}

impl CurvatureTool {
    pub fn new() -> Self {
        Self {
            active_hit: HitTestResult::None,
            drag_start_world: None,
        }
    }
}

impl Tool for CurvatureTool {
    fn handle_input(&mut self, _ctx: &Context, response: &Response, app: &mut Shaper) {
        if response.drag_started() {
            if let Some(screen_pos) = response.interact_pointer_pos() {
                let world = app.screen_to_world(screen_pos);
                let hit = app.hit_test_all(kurbo::Point::new(world.x as f64, world.y as f64));
                // only allow dragging if hit is ControlPoint with ctrl_idx 1 or 2:
                match hit {
                    HitTestResult::ControlPoint {
                        shape_idx,
                        bez_idx,
                        ctrl_idx,
                        orig_pos,
                    } if ctrl_idx == 1 || ctrl_idx == 2 => {
                        self.active_hit = HitTestResult::ControlPoint {
                            shape_idx,
                            bez_idx,
                            ctrl_idx,
                            orig_pos,
                        };
                        self.drag_start_world =
                            Some(kurbo::Point::new(world.x as f64, world.y as f64));
                    }
                    _ => {
                        // ignore drag-start if not a handle; remain in None state
                        self.active_hit = HitTestResult::None;
                        self.drag_start_world = None;
                    }
                }
            }
        }
        if response.dragged() {
            if let (Some(start_world), Some(screen_pos)) =
                (self.drag_start_world, response.interact_pointer_pos())
            {
                let world_now = app.screen_to_world(screen_pos);
                let delta = kurbo::Point::new(
                    world_now.x as f64 - start_world.x,
                    world_now.y as f64 - start_world.y,
                );
                app.apply_drag(&self.active_hit, delta);
            }
        }
        if response.drag_stopped() {
            self.active_hit = HitTestResult::None;
            self.drag_start_world = None;
        }
        // maybe on click show info or toggle smooth/sharp corner, etc.
        if response.clicked() {
            if let Some(screen_pos) = response.interact_pointer_pos() {
                let world = app.screen_to_world(screen_pos);
                if let HitTestResult::ControlPoint {
                    shape_idx,
                    bez_idx,
                    ctrl_idx,
                    ..
                } = app.hit_test_all(kurbo::Point::new(world.x as f64, world.y as f64))
                {
                    // e.g. if endpoint clicked, split/join segments; or toggle corner type
                    // app.toggle_corner_type(shape_idx, bez_idx, ctrl_idx);
                    println!("clicked endpoints");
                }
            }
        }
    }

    fn paint(&mut self, ctx: &Context, painter: &Painter, app: &Shaper) {
        // optionally highlight only handle points (p1/p2) on hover:
        if let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) {
            let world = app.screen_to_world(pos);
            if let HitTestResult::ControlPoint {
                shape_idx,
                bez_idx,
                ctrl_idx,
                ..
            } = app.hit_test_all(kurbo::Point::new(world.x as f64, world.y as f64))
            {
                if ctrl_idx == 1 || ctrl_idx == 2 {
                    // app.paint_handle_hover(shape_idx, bez_idx, ctrl_idx, painter);
                }
            }
        }
    }

    fn tool_ui(&mut self, _ctx: &Context, _app: &mut Shaper) {
        
    }
}
