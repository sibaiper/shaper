mod shape;
mod tool;
mod tools {
    pub mod drawing_tool;
    pub mod editing_tool;
    pub mod panning_tool;
}
use core::f32;

use crate::shape::Shape;
use crate::tool::Tool;
use eframe::egui::{self, Context, Visuals};
use egui::emath::Vec2;
use egui::{Align, Color32, Layout, Sense};
use tools::drawing_tool::DrawingTool;
use tools::editing_tool::EditingTool;
use tools::panning_tool::PanningTool;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum ToolKind {
    Drawing,
    Panning,
    Editing,
    // for later:
    //Selection
}

#[allow(dead_code)]
/// main application state
struct Shaper {
    // render the control points or not
    pub show_handles: bool,

    // render the original line for comparison
    pub draw_original_stroke: bool,

    // list to store all the shapes the user draws:
    pub shapes: Vec<Shape>,

    //current shape to store the currently drawing shape in:
    pub curr_shape: Shape,

    // the tolernace (in screen units) for the simplify function
    pub bezier_tolerance: f64,

    // is drawing variable
    // is_drawing: bool,
    // is panning var
    // is_panning: bool,

    // transform values
    pub pan_offset: Vec2,
    pub zoom: f32,
    // zoom and pan vals:
    pub max_zoom: f32,
    pub min_zoom: f32,
    pub zoom_percent: f32,

    // which tool is currently active
    pub selected_tool: ToolKind,

    // keep each tool in a `Box<dyn Tool>`, so they can be swapped at runtime.
    drawing_tool: Option<Box<dyn Tool>>,
    panning_tool: Option<Box<dyn Tool>>,
    editing_tool: Option<Box<dyn Tool>>,

    // will be probably moved to drawing tool once selection tool is
    // implemented. currently thickness is being used to change the width
    // of all shapes, but once selectiob tool is implemented, each shape
    // can have its own width, and the drawing tool will have a thickness
    // variable to dictate the new shape thickness.
    thickness: f64,

    // variable to track the index
    // of the selected point/s
    // should maybe be moved to
    // the editing-tool
    selected_p: i32,

    // settings variables
    handle_radius: f32,
    handle_arm_thicknes: f32,
    overlay_beziers_thickness: f32,
    p_color: Color32,
    cp_color: Color32,
    p_border_color: Color32,
    selected_p_color: Color32,
    handle_arm_color: Color32,
}

impl Default for Shaper {
    fn default() -> Self {
        let min_zoom_val = 0.1f32;
        let max_zoom_val = 16.0f32;
        let default_zoom_val = 1.0f32;

        // calc zoom_percent based on the default zoom
        let zoom_percent_val =
            (default_zoom_val - min_zoom_val) / (max_zoom_val - min_zoom_val) * 100.0;

        Shaper {
            shapes: Vec::new(),
            curr_shape: Shape::new(10.0, Color32::BLACK),
            bezier_tolerance: 10.0,
            show_handles: false,
            draw_original_stroke: false,

            pan_offset: Vec2::ZERO,
            zoom: 1.0,
            max_zoom: max_zoom_val,
            min_zoom: min_zoom_val,
            zoom_percent: zoom_percent_val,

            selected_tool: ToolKind::Drawing,
            drawing_tool: Some(Box::new(DrawingTool::new())),
            panning_tool: Some(Box::new(PanningTool::new())),
            editing_tool: Some(Box::new(EditingTool::new())),
            thickness: 10.0,

            selected_p: -1, //

            // sizes
            handle_radius: 4.0,
            handle_arm_thicknes: 1.5,
            overlay_beziers_thickness: 1.0,
            // colors
            p_color: Color32::WHITE,
            cp_color: Color32::WHITE,
            p_border_color: Color32::from_rgb(10, 118, 241),
            selected_p_color: Color32::from_rgb(10, 118, 241),
            handle_arm_color: Color32::from_rgb(10, 118, 241),
        }
    }
}
fn main() {
    let native_options = eframe::NativeOptions::default();
    let _ = eframe::run_native(
        "Shaper",
        native_options,
        Box::new(|cc| Ok(Box::new(Shaper::new(cc)))),
    );
}

impl Shaper {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }

    /// given a point in the drawing’s logical coordinate system,
    /// return the point in screen‐space after applying zoom and pan.
    pub fn world_to_screen(&self, p: egui::Pos2) -> egui::Pos2 {
        egui::Pos2::new(
            p.x * self.zoom + self.pan_offset.x,
            p.y * self.zoom + self.pan_offset.y,
        )
    }

    pub fn screen_to_world(&self, p: egui::Pos2) -> egui::Pos2 {
        egui::Pos2::new(
            (p.x - self.pan_offset.x) / self.zoom,
            (p.y - self.pan_offset.y) / self.zoom,
        )
    }

    // func to update the zoom_level variable internatlly
    // based on the also internally stored zoom variable.
    pub fn calc_zoom_level(&mut self) {
        // calc zoom_percent based on the default zoom
        self.zoom_percent = (self.zoom - self.min_zoom) / (self.max_zoom - self.min_zoom) * 100.0;
    }
}

impl eframe::App for Shaper {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // set bgc/other visuals if needed
        ctx.set_visuals(Visuals {
            window_fill: Color32::WHITE,
            ..egui::Visuals::light() // base style
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let canvas_height = ctx.available_rect().height();
            let (response, painter) = ui.allocate_painter(
                egui::Vec2::new(ctx.available_rect().width(), canvas_height),
                Sense::drag(),
            );

            // handle input based on selected tool
            // this requires a couple extra steps to make it work:
            // copy the enum value out of self:
            let current_tool = self.selected_tool;
            match current_tool {
                ToolKind::Drawing => {
                    // 1) take() the DrawingTool out of the Option<Box<dyn Tool>>
                    let mut tool: Box<dyn Tool + 'static> = self
                        .drawing_tool
                        .take()
                        .expect("drawing_tool was None when it shouldn`t be");

                    // 2) call handle_input, giving it mutable access to both tool and app
                    tool.handle_input(ctx, &response, self);

                    // 3) put the Box<dyn Tool> back into self
                    self.drawing_tool = Some(tool);
                }
                ToolKind::Panning => {
                    let mut tool = self
                        .panning_tool
                        .take()
                        .expect("panning_tool was None when it shouldn`t be");

                    tool.handle_input(ctx, &response, self);

                    self.panning_tool = Some(tool);
                }

                ToolKind::Editing => {
                    let mut tool = self
                        .editing_tool
                        .take()
                        .expect("editing_tool was None when it shouldn`t be");

                    tool.handle_input(ctx, &response, self);

                    self.editing_tool = Some(tool);
                }
            }

            // draw all finished shapes (Béziers, raw, handles) by using world_to_screen() internally —
            for shape in &self.shapes {
                shape.draw_beziers(&painter, self);
            }

            // draw in-progress stroke in gray:
            // TODO
            // update this to use this method:
            // https://github.com/emilk/egui/blob/main/crates/egui_demo_lib/src/demo/painting.rs
            for window in self.curr_shape.current_stroke.windows(2) {
                let a = self.world_to_screen(window[0]);
                let b = self.world_to_screen(window[1]);
                painter.line_segment([a, b], egui::Stroke::new(self.curr_shape.thickness * self.zoom, self.curr_shape.stroke_color));
            }
            // optionally draw raw strokes in green:
            if self.draw_original_stroke {
                for shape in &self.shapes {
                    shape.draw_raw(&painter, self);
                }
            }
            // optionally draw handles in panning/drawing interactive mode:
            if self.show_handles {
                for shape in &self.shapes {
                    // draw the overlay beziers first
                    shape.draw_overlay_beziers(&painter, self);
                    shape.draw_handles(&painter, self);
                }
            }

            // — let the active tool paint any overlays (e.g. pan‐mode highlight) —
            match current_tool {
                ToolKind::Drawing => {
                    let mut tool = self
                        .drawing_tool
                        .take()
                        .expect("drawing_tool was None when it shouldn’t be");
                    tool.paint(ctx ,&painter, self);
                    self.drawing_tool = Some(tool);
                }
                ToolKind::Panning => {
                    let mut tool = self
                        .panning_tool
                        .take()
                        .expect("panning_tool was None when it shouldn’t be");
                    tool.paint(ctx ,&painter, self);
                    self.panning_tool = Some(tool);
                }

                ToolKind::Editing => {
                    let mut tool = self
                        .editing_tool
                        .take()
                        .expect("editing_tool was None when it shouldn`t be");
                    tool.paint(ctx ,&painter, self);
                    self.editing_tool = Some(tool);
                }
            }

            // draw the settings & tool‐selector windows (always at fixed screen coords)

            self.show_settings_window(ctx);
            self.show_tools_window(ctx);
            self.show_tool_specific_ui(ctx);
        });
    }
}

impl Shaper {
    // settings window
    fn show_settings_window(&mut self, ctx: &Context) {
        egui::Window::new("Settings")
            .anchor(egui::Align2::RIGHT_TOP, egui::Vec2::new(-10.0, 10.0))
            .collapsible(false)
            .title_bar(false)
            .show(ctx, |ui| {
                ui.checkbox(&mut self.show_handles, "Show handles");
                ui.checkbox(&mut self.draw_original_stroke, "Draw original stroke");
            });
    }

    // tools window
    fn show_tools_window(&mut self, ctx: &Context) {
        egui::Window::new("Tools")
            .title_bar(false)
            .resizable(false)
            .default_height(f32::NAN)
            .anchor(egui::Align2::CENTER_BOTTOM, egui::Vec2::new(0.0, -10.0))
            .show(ctx, |ui| {
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    if ui.button("Draw").clicked() {
                        self.selected_tool = ToolKind::Drawing;
                    }
                    if ui.button("Pan-Zoom").clicked() {
                        self.selected_tool = ToolKind::Panning;
                    }
                    if ui.button("Edit").clicked() {
                        self.selected_tool = ToolKind::Editing;
                    }
                });
            });
    }

    /// Displays the UI for the currently selected tool.
    ///
    /// This method handles a common Rust borrowing pattern: to avoid
    /// "multiple mutable borrows" when a tool's `tool_ui` method needs
    /// mutable access to both the tool itself and the `Shaper` instance.
    ///
    /// It achieves this by temporarily taking the active tool out of its `Option`,
    /// allowing it to be mutably borrowed and used, and then placing it back.
    ///
    /// Panics if the selected tool is unexpectedly `None`.
    fn show_tool_specific_ui(&mut self, ctx: &egui::Context) {
        let current_tool = self.selected_tool;
        match current_tool {
            ToolKind::Drawing => {
                let mut tool = self.drawing_tool.take().expect("drawing_tool was None");
                tool.tool_ui(ctx, self);
                self.drawing_tool = Some(tool);
            }
            ToolKind::Panning => {
                let mut tool = self.panning_tool.take().expect("panning_tool was None");
                tool.tool_ui(ctx, self);
                self.panning_tool = Some(tool);
            }
            ToolKind::Editing => {
                let mut tool = self.editing_tool.take().expect("editing_tool was None");
                tool.tool_ui(ctx, self);
                self.editing_tool = Some(tool);
            }
        }
    }
}
