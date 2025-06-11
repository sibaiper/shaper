use crate::HitTestResult;
use crate::Shaper;
use crate::shape;
use crate::tool::Tool;
use eframe::egui::Color32;
use eframe::egui::Stroke;
use eframe::egui::StrokeKind;
use eframe::egui::{Context, Painter, Pos2, Rect, Response};
use eframe::egui::emath::Vec2;

const DRAG_THRESHOLD: f64 = 5.0;

pub struct SelectionTool {
    active_hit: HitTestResult,
    drag_start_world: Option<kurbo::Point>,
    drag_start_screen: Option<kurbo::Point>,
    is_marquee: bool,
    is_moving_shape: bool,
    is_dragging: bool,
}

impl SelectionTool {
    pub fn new() -> Self {
        Self {
            active_hit: HitTestResult::None,
            drag_start_world: None,
            drag_start_screen: None,
            is_marquee: false,
            is_moving_shape: false,
            is_dragging: false,
        }
    }
}

impl Tool for SelectionTool {
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

        // on drag start: decide shape-drag vs marquee vs ignore control-point hits
        if response.drag_started() {
            if let Some(screen_pos) = response.interact_pointer_pos() {
                let world = app.screen_to_world(screen_pos);
                let hit = app.hit_test_all(kurbo::Point::new(world.x as f64, world.y as f64));
                let shift_held = ctx.input(|i| i.modifiers.shift);

                if shift_held {
                    // Always start marquee if shift is held
                    self.active_hit = HitTestResult::None;
                    self.is_marquee = true;
                    self.is_moving_shape = false;
                    self.drag_start_world = Some(kurbo::Point::new(world.x as f64, world.y as f64));
                } else {
                    match hit {
                        HitTestResult::CurveSegment { shape_idx, .. } => {
                            // start dragging the whole shape when clicking a curve segment
                            let orig_beziers = app.shapes[shape_idx].beziers.clone();
                            self.active_hit = HitTestResult::ShapeBody {
                                shape_idx,
                                orig_beziers,
                            };
                            self.is_marquee = false;
                            self.is_moving_shape = true;
                            self.drag_start_world =
                                Some(kurbo::Point::new(world.x as f64, world.y as f64));
                            self.drag_start_screen =
                                Some(kurbo::Point::new(screen_pos.x as f64, screen_pos.y as f64));

                            app.select_single_shape(shape_idx);
                        }
                        _ => {
                            // for any other hit (including ShapeBody/bounding box or None)
                            // prepare for potential marquee selection
                            self.active_hit = HitTestResult::None;
                            self.is_marquee = false; // Will be set to true after drag threshold
                            self.is_moving_shape = false;
                            self.drag_start_world =
                                Some(kurbo::Point::new(world.x as f64, world.y as f64));
                        }
                    }
                }
            }
        }

        // on drag
        if response.dragged() {
            if let (Some(start_world), Some(screen_pos)) =
                (self.drag_start_world, response.interact_pointer_pos())
            {
                let drag_curr_world: Pos2 = app.screen_to_world(screen_pos);

                // calc the distance moved
                let delta_x = drag_curr_world.x as f64 - start_world.x;
                let delta_y = drag_curr_world.y as f64 - start_world.y;
                let distance_moved = (delta_x.powi(2) + delta_y.powi(2)).sqrt();

                if distance_moved > DRAG_THRESHOLD / app.zoom as f64 {
                    self.is_dragging = true;

                    if self.is_moving_shape {
                        // shape drag
                        let delta = kurbo::Point::new(
                            drag_curr_world.x as f64 - start_world.x,
                            drag_curr_world.y as f64 - start_world.y,
                        );
                        app.apply_drag(&self.active_hit, delta);
                    } else {
                        // marquee selection
                        self.is_marquee = true;
                        let rect = Rect::from_two_pos(
                            Pos2::new(start_world.x as f32, start_world.y as f32),
                            drag_curr_world,
                        );
                        app.select_shapes_in_rect(rect);
                    }
                }
            }
        }
        // On drag end:
        if response.drag_stopped() {
            // if no drag occurred and the user did not click on the curve, clear selection
            if !self.is_dragging && self.active_hit == HitTestResult::None {
                app.selected_shapes.clear();
            }

            // if it was marquee, final selection already applied
            // above frame-by-frame or can re-apply here.
            self.active_hit = HitTestResult::None;
            self.drag_start_world = None;
            self.drag_start_screen = None;
            self.is_marquee = false;
            self.is_moving_shape = false;
            self.is_dragging = false;
        }

        // on click without drag:
        // if response.clicked() {
        //     if let Some(screen_pos) = response.interact_pointer_pos() {
        //         let world = app.screen_to_world(screen_pos);

        //         match app.hit_test_all(kurbo::Point::new(world.x as f64, world.y as f64)) {
        //             // HitTestResult::ShapeBody { shape_idx, .. } => {
        //             //     app.selected_shapes.clear();
        //             //     app.selected_shapes.insert(shape_idx);
        //             // }
        //             HitTestResult::CurveSegment { shape_idx, .. } => {
        //                 app.selected_shapes.clear();
        //                 app.selected_shapes.insert(shape_idx);

        //             }
        //             _ => {
        //                 app.selected_shapes.clear();
        //             }
        //         }
        //     }
        // }
    }

    fn paint(&mut self, ctx: &Context, painter: &Painter, app: &Shaper) {
        if let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) {
            let world = app.screen_to_world(pos);

            // draw marquee rectangle if active
            if self.is_marquee {
                if let Some(start_world) = self.drag_start_world {
                    // convert world positions to screen positions
                    let start_screen =
                        app.world_to_screen(Pos2::new(start_world.x as f32, start_world.y as f32));
                    let curr_screen = pos;
                    let marquee_rect = Rect::from_two_pos(start_screen, curr_screen);

                    if let (Ok(fill_color), Ok(stroke_color)) = (
                        Color32::from_hex("#9ED5F788"),
                        Color32::from_hex("#1F5FCFBB"),
                    ) {
                        let stroke = Stroke::new(1.0, stroke_color);
                        painter.rect_filled(marquee_rect, 0.0, fill_color);
                        painter.rect_stroke(marquee_rect, 0.0, stroke, StrokeKind::Middle);
                    }
                    // painter.rect_filled(
                    //     marquee_rect,
                    //     0.0,
                    //     Color32::from_rgba_unmultiplied(158, 213, 247, 140),
                    // );
                    // painter.rect_stroke(marquee_rect, 0.0, stroke, StrokeKind::Middle);
                }
            }

            if !app.selected_shapes.is_empty() {
                // first, compute the union bounding box of all selected shapes
                let mut bbox: Option<kurbo::Rect> = None;
                // TODO: fix this bug
                // when selecting an item, and then deleting an item, 
                // that causes unexptedted behvaior, or panic and crash
                // current solution: clear selected items list completely
                for &shape_idx in &app.selected_shapes {
                    if let Some(shape_bbox) = app.shapes[shape_idx].bounding_box() {
                        bbox = Some(match bbox {
                            Some(accum) => accum.union(shape_bbox),
                            None => shape_bbox,
                        });
                    }
                }
                // draw control points for all selected shapes
                for &shape_idx in &app.selected_shapes {
                    if let Some(shape) = app.shapes.get(shape_idx) {
                        shape.draw_overlay_beziers(painter, app);
                        shape.draw_handles(painter, app);
                    }
                }

                if let Some(bbox) = bbox {
                    // then, convert bbox from world to screen coordinates
                    let min = app
                        .world_to_screen(eframe::egui::Pos2::new(bbox.x0 as f32, bbox.y0 as f32));
                    let max = app
                        .world_to_screen(eframe::egui::Pos2::new(bbox.x1 as f32, bbox.y1 as f32));
                    let selected_rect = eframe::egui::Rect::from_min_max(min, max);

                    // last, draw the rectangle
                    // let stroke = Stroke::new(2.0, Color32::BLUE);
                    let stroke = Stroke::new(2.0, Color32::from_rgb(158, 213, 247));
                    painter.rect_stroke(selected_rect, 0.0, stroke, StrokeKind::Middle);
                }
            } else {
                if let HitTestResult::ShapeBody { shape_idx, .. } =
                    app.hit_test_all(kurbo::Point::new(world.x as f64, world.y as f64))
                {
                    app.paint_hover_outline(shape_idx, painter);
                }
            }
        }
    }

    fn tool_ui(&mut self, _ctx: &Context, _app: &mut Shaper) {
        // copied from drawing tool for reference for now
        // egui::TopBottomPanel::top("drawing settings")
        //     .resizable(false)
        //     .show(ctx, |ui| {
        //         ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
        //             // slider for the tolerance of the drawing tool
        //             let tol = egui::Slider::new(&mut self.bezier_tolerance, 1.0..=100.0)
        //                 .text("Tolerance")
        //                 .orientation(SliderOrientation::Horizontal);
        //             ui.add(tol);

        //             // slider for thickness of curves
        //             let width = egui::Slider::new(&mut self.thickness, 1.0..=100.0)
        //                 .text("Thickness")
        //                 .orientation(SliderOrientation::Horizontal);
        //             if ui.add(width).changed() {
        //                 app.curr_shape.thickness = self.thickness;
        //             }

        //             // color picker for the stroke using
        //             // the color edit button (most common)
        //             ui.horizontal(|ui| {
        //                 let color_response = egui::widgets::color_picker::color_edit_button_srgba(
        //                     ui,
        //                     &mut self.drawing_color,
        //                     Alpha::Opaque,
        //                 );
        //                 if color_response.changed() {
        //                     app.curr_shape.stroke_color = self.drawing_color;
        //                 }
        //                 ui.label("Stroke Color:");
        //             });
        //         });
        //     });
    }
}
