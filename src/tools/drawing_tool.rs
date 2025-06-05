use eframe::egui::{self, Event, Context, Painter, Response, Stroke, Color32};
use crate::{Shape, Shaper};

pub struct DrawingTool;

impl DrawingTool {
    pub fn new() -> Self {
        DrawingTool
    }
}

impl super::Tool for DrawingTool {
    fn handle_input(
        &mut self,
        ctx: &Context,
        response: &Response,
        app: &mut Shaper,
    ) {
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
                    Some(&last) => last.distance(world_pos) > app.curr_shape.sample_tol,
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
                app.curr_shape.fit_curve_and_store(&stroke, app.bezier_tolerance);

                // push shape and reset
                app.shapes.push(app.curr_shape.clone());
                app.curr_shape = Shape::new();
            }
        }

        // event: allow “delete last stroke” via Backspace/Delete:
        for event in &ctx.input(|i: &egui::InputState| i.events.clone()) {
            if let Event::Key { key, pressed: true, .. } = event {
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

    fn paint(&mut self, painter: &Painter, app: &Shaper) {
        // draw all finished shapes (outside of current in-progress stroke)
        for shape in &app.shapes {
            shape.draw_beziers(painter, &app);
        }

        // move this to inside the shape as a method later.. leave here for now for easier debugging and working
        // draw in-progress raw stroke
        // apply zoom & pan to each endpoint
        for window in app.curr_shape.current_stroke.windows(2) {
            let a = app.world_to_screen(window[0]);
            let b = app.world_to_screen(window[1]);
            painter.line_segment([a, b], Stroke::new(5.0, Color32::GRAY));
        }

        // draw original green stroke if enabled
        if app.draw_original_stroke {
            for shape in &app.shapes {
                shape.draw_raw(painter, &app);
            }
        }

        // draw handles if requested
        if app.show_handles {
            for shape in &app.shapes {
                shape.draw_handles(painter, &app);
            }
        }
    }
}
