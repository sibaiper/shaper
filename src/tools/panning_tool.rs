use crate::tool::Tool;
use crate::Shaper;
use eframe::egui::{Context, Painter, Pos2, Response, Vec2};

pub struct PanningTool {
    /// remember the pointer position at the start of drag
    drag_start: Option<Pos2>,
    orig_pan: Vec2,
}

impl PanningTool {
    pub fn new() -> Self {
        PanningTool {
            drag_start: None,
            orig_pan: Vec2::ZERO,
        }
    }
}

impl Tool for PanningTool {
    fn handle_input(&mut self, ctx: &Context, response: &Response, app: &mut Shaper) {
        // handle zooming with scroll wheel
        if let Some(pointer_pos) = response.hover_pos() {
            let scroll_delta = ctx.input(|i| i.smooth_scroll_delta.y);
            if scroll_delta != 0.0 {
                // convert world position before zoom
                let old_world_pos = app.screen_to_world(pointer_pos);

                // apply zoom
                let zoom_delta = (scroll_delta * 0.009).exp();
                app.zoom *= zoom_delta;
                app.zoom = app.zoom.clamp(0.1, 16.0);

                // convert world position after zoom
                let new_world_pos = app.screen_to_world(pointer_pos);

                // adjust pan offset to keep pointer position stable
                // convert Pos2 difference directly to Vec2
                let world_delta = Vec2::new(
                    new_world_pos.x - old_world_pos.x,
                    new_world_pos.y - old_world_pos.y,
                );
                app.pan_offset += world_delta * app.zoom;
            }
        }

        // when the user starts dragging, record the initial pointer and pan
        if response.drag_started() {
            if let Some(pos) = response.interact_pointer_pos() {
                self.drag_start = Some(pos);
                self.orig_pan = app.pan_offset;
            }
        }

        // while dragging, compute delta from start, and adjust pan_offset
        if response.dragged() {
            if let (Some(start), Some(current)) = (self.drag_start, response.interact_pointer_pos())
            {
                let delta = current - start;
                app.pan_offset = self.orig_pan + Vec2::new(delta.x, delta.y);
            }
        }

        // on drag end, clear the stored start position
        if response.drag_stopped() {
            self.drag_start = None;
        }
    }

    fn paint(&mut self, _painter: &Painter, _app: &Shaper) {
        // draw an overlay indicating the pan mode is active.
        // example: draw a semi‐transparent rectangle or cursor hint.
        // let rect = painter.clip_rect();
        // painter.rect_stroke(
        //     rect,
        //     0.0,
        //     Stroke::new(5.0, Color32::LIGHT_BLUE),
        //     StrokeKind::Middle,
        // );
    }

    fn tool_ui(&mut self, ctx: &Context, app: &crate::Shaper) {
        // do nothing for
    }
}
