use crate::tool::Tool;
use crate::{Shape, Shaper};
use eframe::egui::{self, Align, Color32, Context, Event, Layout, Painter, Response, SliderOrientation};
use egui::emath::Vec2;

pub struct DrawingTool {
    bezier_tolerance: f64,
    thickness: f32,

    /// Minimum pixel distance before we sample a new raw point
    sample_tol: f32,


    drawing_color: Color32,
}

impl DrawingTool {
    pub fn new() -> Self {
        DrawingTool {
            bezier_tolerance: 10.0,
            thickness: 10.0,
            sample_tol: 2.0,
            drawing_color: Color32::BLACK,
        }
    }
}

impl Tool for DrawingTool {
    fn handle_input(&mut self, ctx: &Context, response: &Response, app: &mut Shaper) {
        // handle zooming  in and out first
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

        // begin raw stroke
        if response.drag_started() {
            app.curr_shape.current_stroke.clear();
            if let Some(pos) = response.interact_pointer_pos() {
                // app.curr_shape is reset on drag end every time. No need to reset it on drag start.
                let world_pos = app.screen_to_world(pos);
                app.curr_shape.current_stroke.push(world_pos);
            }
        }

        if response.dragged() {
            if let Some(pos) = response.interact_pointer_pos() {
                let world_pos = app.screen_to_world(pos);
                let should_add = match app.curr_shape.current_stroke.last() {
                    Some(&last) => last.distance(world_pos) > (self.sample_tol / app.zoom), // make sample_tol take into account the zoom level
                    None => true,
                };
                if should_add {
                    app.curr_shape.current_stroke.push(world_pos);
                }
            }
        }

        if response.drag_stopped() {
            if !app.curr_shape.current_stroke.is_empty() {
                // store raw stroke
                app.curr_shape
                    .raw_strokes
                    .push(app.curr_shape.current_stroke.clone());

                // fit to Bézier chain
                let stroke = app.curr_shape.current_stroke.clone();
                app.curr_shape
                    .fit_curve_and_store(&stroke, self.bezier_tolerance);

                // push shape and reset
                app.shapes.push(app.curr_shape.clone());
                app.curr_shape = Shape::new(self.thickness, self.drawing_color);
            }
        }

        // event: allow “delete last stroke” via Backspace/Delete:
        for event in &ctx.input(|i: &egui::InputState| i.events.clone()) {
            if let Event::Key {
                key, pressed: true, ..
            } = event
            {
                match key {
                    egui::Key::Delete | egui::Key::Backspace => {
                        if let Some(_) = app.shapes.pop() {
                            //nothing to do actually
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn paint(&mut self, ctx: &Context, painter: &Painter, app: &Shaper) {
        // draw a small circle to indicate the cursor position (pen size)
        if let Some(mouse_pos) = ctx.input(|i| i.pointer.hover_pos()) {
            painter.circle_filled(mouse_pos, (self.thickness * app.zoom) / 2.0, self.drawing_color);
        }
    }

    // slider for the value of the
    fn tool_ui(&mut self, ctx: &Context, app: &mut Shaper) {
        egui::TopBottomPanel::top("drawing settings")
            .resizable(false)
            .show(ctx, |ui| {
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    // slider for the tolerance of the drawing tool
                    let tol = egui::Slider::new(&mut self.bezier_tolerance, 1.0..=100.0)
                        .text("Tolerance")
                        .orientation(SliderOrientation::Horizontal);
                    ui.add(tol);

                    // slider for thickness of curves
                    let width = egui::Slider::new(&mut self.thickness, 1.0..=100.0)
                        .text("Thickness")
                        .orientation(SliderOrientation::Horizontal);
                    if ui.add(width).changed() {
                        app.curr_shape.thickness = self.thickness;
                    }
                });
            });
    }
}
