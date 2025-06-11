mod shape;
mod tool;
mod tools;

use core::f32;
use std::collections::HashSet;

use crate::shape::Shape;
use crate::tool::Tool;
use crate::tool::tools::CurvatureTool;
use crate::tool::tools::DirectSelectionTool;
use crate::tool::tools::DrawingTool;
use crate::tool::tools::EditingTool;
use crate::tool::tools::PanningTool;
use crate::tool::tools::SelectionTool;
use eframe::egui::{self, Context, Painter, Visuals};
use egui::emath::Vec2;
use egui::{Align, Color32, Layout, Pos2, Sense};
use kurbo::{CubicBez, ParamCurveNearest, Point as KPoint};

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum ToolKind {
    Drawing,
    Panning,
    Editing,
    Selection,
    Curvature,
    DirectSelection,
}

/// identifies what was hit at drag start.
#[derive(Clone, Debug, PartialEq)]
pub enum HitTestResult {
    ControlPoint {
        shape_idx: usize,
        bez_idx: usize,
        ctrl_idx: usize,  // 0..3
        orig_pos: KPoint, // the world-space position at drag start
    },
    CurveSegment {
        shape_idx: usize,
        bez_idx: usize,
        orig_p0: KPoint,
        orig_p1: KPoint,
        orig_p2: KPoint,
        orig_p3: KPoint,
    },
    ShapeBody {
        shape_idx: usize,
        orig_beziers: Vec<CubicBez>, // store a copy of all beziers at drag start
    },
    None,
}

/// identifies a specific control point in a shape's bezier segment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PointId {
    pub shape_idx: usize,
    pub bezier_idx: usize,
    pub ctrl_idx: usize, // 0..3 for p0..p3
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
    selecting_tool: Option<Box<dyn Tool>>,
    direct_selecting_tool: Option<Box<dyn Tool>>,
    curvature_tool: Option<Box<dyn Tool>>,

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
    // selected_shapes: Vec<HitTestResult>,
    // selected_points: Vec<HitTestResult>, // usize for now, best be updated later.
    pub selected_shapes: HashSet<usize>,   // store shape indices
    pub selected_points: HashSet<PointId>, // store selected control points
    // optionally other selection state: e.g. selected_curve_segments, etc.

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
            selecting_tool: Some(Box::new(SelectionTool::new())),
            direct_selecting_tool: Some(Box::new(DirectSelectionTool::new())),
            curvature_tool: Some(Box::new(CurvatureTool::new())),

            thickness: 10.0,

            selected_p: -1,
            // selected_points: Vec::new(),
            // selected_shapes: Vec::new(),
            selected_points: HashSet::new(),
            selected_shapes: HashSet::new(),

            // sizes
            handle_radius: 2.0,
            handle_arm_thicknes: 1.0,
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

    /// loop shapes in reverse draw order so topmost is found first.
    pub fn hit_test_all(&self, mouse: KPoint) -> HitTestResult {
        let tol_point = self.handle_radius as f64;
        let tol_point_sq = tol_point * tol_point;

        // self.shapes: Vec<Shape>, each shape.beziers: Vec<CubicBez>
        for (shape_idx, shape) in self.shapes.iter().enumerate().rev() {
            // scale the curve tolerance by zoom and use a small base tolerance
            let base_tolerance = 2.0; // small fixed tolerance in pixels
            let tol_curve = (base_tolerance / self.zoom as f64) + (shape.thickness as f64 * 0.5);
            let tol_curve_sq = tol_curve * tol_curve;
            
            for (bez_idx, bez) in shape.beziers.iter().enumerate() {
                // 1) curve itself: use `nearest(...)` and compare distance_sq
                // Kurbo’s `nearest(...)` returns a `Nearest { distance_sq, t }`
                // supply a small “accuracy” (1e-6) to get a precise t, then check if
                // dist² ≤ tol²:
                let nearest = bez.nearest(mouse, 1e-6);
                if nearest.distance_sq <= tol_curve_sq {
                    return HitTestResult::CurveSegment {
                        shape_idx,
                        bez_idx,
                        orig_p0: bez.p0,
                        orig_p1: bez.p1,
                        orig_p2: bez.p2,
                        orig_p3: bez.p3,
                    };
                }

                // 2) control handles
                let handles = [bez.p0, bez.p1, bez.p2, bez.p3];
                for (ctrl_idx, &pt) in handles.iter().enumerate() {
                    let dx = mouse.x - pt.x;
                    let dy = mouse.y - pt.y;
                    if dx * dx + dy * dy <= tol_point_sq {
                        return HitTestResult::ControlPoint {
                            shape_idx,
                            bez_idx,
                            ctrl_idx,
                            orig_pos: pt,
                        };
                    }
                }
            }
        }

        for (shape_idx, shape) in self.shapes.iter().enumerate().rev() {
            // 3) shape body (bounding box test; implement bounding_box())
            if let Some(bounds) = shape.bounding_box() {
                // bounds is kurbo::Rect in world coords
                if bounds.contains(mouse) {
                    return HitTestResult::ShapeBody {
                        shape_idx,
                        orig_beziers: shape.beziers.clone(),
                    };
                }
            }
        }

        HitTestResult::None
    }

    /// apply a drag given the initial HitTestResult and the delta (world-space).
    pub fn apply_drag(&mut self, hit: &HitTestResult, delta: KPoint) {
        match hit {
            HitTestResult::ControlPoint {
                shape_idx,
                bez_idx,
                ctrl_idx,
                orig_pos,
            } => {
                let new_pt = KPoint::new(orig_pos.x + delta.x, orig_pos.y + delta.y);
                if let Some(shape) = self.shapes.get_mut(*shape_idx) {
                    if let Some(bez) = shape.beziers.get_mut(*bez_idx) {
                        match ctrl_idx {
                            0 => {
                                // move p0 and adjust handles/neighbors
                                let dx = new_pt.x - bez.p0.x;
                                let dy = new_pt.y - bez.p0.y;
                                bez.p0 = new_pt;
                                bez.p1 = KPoint::new(bez.p1.x + dx, bez.p1.y + dy);
                                if *bez_idx > 0 {
                                    let prev = &mut shape.beziers[*bez_idx - 1];
                                    prev.p3 = new_pt;
                                    prev.p2 = KPoint::new(prev.p2.x + dx, prev.p2.y + dy);
                                }
                            }
                            1 => {
                                bez.p1 = new_pt;
                            }
                            2 => {
                                bez.p2 = new_pt;
                            }
                            3 => {
                                // move p3 and adjust neighbor
                                let dx = new_pt.x - bez.p3.x;
                                let dy = new_pt.y - bez.p3.y;
                                bez.p3 = new_pt;
                                bez.p2 = KPoint::new(bez.p2.x + dx, bez.p2.y + dy);
                                if *bez_idx + 1 < shape.beziers.len() {
                                    let next = &mut shape.beziers[*bez_idx + 1];
                                    next.p0 = new_pt;
                                    next.p1 = KPoint::new(next.p1.x + dx, next.p1.y + dy);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            HitTestResult::CurveSegment {
                shape_idx,
                bez_idx,
                orig_p0,
                orig_p1,
                orig_p2,
                orig_p3,
            } => {
                if let Some(shape) = self.shapes.get_mut(*shape_idx) {
                    let new_p0 = KPoint::new(orig_p0.x + delta.x, orig_p0.y + delta.y);
                    let new_p1 = KPoint::new(orig_p1.x + delta.x, orig_p1.y + delta.y);
                    let new_p2 = KPoint::new(orig_p2.x + delta.x, orig_p2.y + delta.y);
                    let new_p3 = KPoint::new(orig_p3.x + delta.x, orig_p3.y + delta.y);
                    {
                        let bez = &mut shape.beziers[*bez_idx];
                        bez.p0 = new_p0;
                        bez.p1 = new_p1;
                        bez.p2 = new_p2;
                        bez.p3 = new_p3;
                    }
                    if *bez_idx > 0 {
                        let prev = &mut shape.beziers[*bez_idx - 1];
                        prev.p3 = new_p0;
                    }
                    if *bez_idx + 1 < shape.beziers.len() {
                        let next = &mut shape.beziers[*bez_idx + 1];
                        next.p0 = new_p3;
                    }
                }
            }
            HitTestResult::ShapeBody {
                shape_idx,
                orig_beziers,
            } => {
                if let Some(shape) = self.shapes.get_mut(*shape_idx) {
                    for (bez, orig_bez) in shape.beziers.iter_mut().zip(orig_beziers.iter()) {
                        bez.p0 = KPoint::new(orig_bez.p0.x + delta.x, orig_bez.p0.y + delta.y);
                        bez.p1 = KPoint::new(orig_bez.p1.x + delta.x, orig_bez.p1.y + delta.y);
                        bez.p2 = KPoint::new(orig_bez.p2.x + delta.x, orig_bez.p2.y + delta.y);
                        bez.p3 = KPoint::new(orig_bez.p3.x + delta.x, orig_bez.p3.y + delta.y);
                    }
                }
            }
            HitTestResult::None => {}
        }
    }

    /// select shapes whose bounds intersect or are contained in rect.
    /// `rect` here should be in world coordinates (so convert before calling).
    pub fn select_shapes_in_rect(&mut self, rect: egui::Rect) {
        self.selected_shapes.clear();
        for (idx, shape) in self.shapes.iter().enumerate() {
            if let Some(bounds) = shape.bounding_box() {
                // bounding_box returns kurbo::Rect in world coords
                // convert to egui::Rect or compare manually
                let bb = egui::Rect::from_min_max(
                    egui::Pos2::new(bounds.x0 as f32, bounds.y0 as f32),
                    egui::Pos2::new(bounds.x1 as f32, bounds.y1 as f32),
                );
                if rect.intersects(bb) {
                    self.selected_shapes.insert(idx);
                }
            }
        }
        // (optionally) clear selected_points || tho this should probably be called from outside this function
        self.selected_shapes.clear();
    }

    /// select control points inside the given world-space rect.
    pub fn select_points_in_rect(&mut self, rect: egui::Rect) {
        self.selected_points.clear();
        for (shape_idx, shape) in self.shapes.iter().enumerate() {
            for (bez_idx, bez) in shape.beziers.iter().enumerate() {
                let handles = [bez.p0, bez.p1, bez.p2, bez.p3];
                for (ctrl_idx, &pt) in handles.iter().enumerate() {
                    let pos2 = egui::Pos2::new(pt.x as f32, pt.y as f32);
                    if rect.contains(pos2) {
                        let pid = PointId {
                            shape_idx,
                            bezier_idx: bez_idx,
                            ctrl_idx,
                        };
                        self.selected_points.insert(pid);
                    }
                }
            }
        }
        // Optionally clear selected_shapes or leave as is, depending on UX
        self.selected_shapes.clear();
    }

    /// when selecting a shape, clear point selection:
    pub fn select_shape_clearing_points(&mut self, shape_idx: usize) {
        self.selected_shapes.clear();
        self.selected_shapes.insert(shape_idx);
        self.selected_points.clear();
    }

    /// when selecting a point, optionally also select its shape:
    pub fn select_point_and_shape(&mut self, pid: PointId) {
        self.selected_points.clear();
        self.selected_points.insert(pid);
        self.selected_shapes.insert(pid.shape_idx);
    }

    pub fn select_single_shape(&mut self, shape_idx: usize) {
        self.selected_shapes.clear();
        self.selected_shapes.insert(shape_idx);
        // optional: also clear selected_points if the UX deselects points when selecting shape
        self.selected_points.clear();
    }

    pub fn toggle_shape_selection(&mut self, shape_idx: usize) {
        if self.selected_shapes.contains(&shape_idx) {
            self.selected_shapes.remove(&shape_idx);
        } else {
            self.selected_shapes.insert(shape_idx);
        }
        // maybe also clear selected_points or adjust them
    }

    pub fn paint_hover_outline(&self, shape_idx: usize, painter: &Painter) {}

    /// return the current world-space position of the control point identified by pid.
    pub fn get_point_position(&self, pid: PointId) -> Option<kurbo::Point> {
        self.shapes.get(pid.shape_idx).and_then(|shape| {
            shape.beziers.get(pid.bezier_idx).map(|bez| {
                match pid.ctrl_idx {
                    0 => bez.p0,
                    1 => bez.p1,
                    2 => bez.p2,
                    3 => bez.p3,
                    _ => bez.p0, // shouldn't happen
                }
            })
        })
    }

    /// move a single control point to a new world-space position.
    pub fn move_point_to(&mut self, pid: PointId, new_pos: kurbo::Point) {
        if let Some(shape) = self.shapes.get_mut(pid.shape_idx) {
            if let Some(bez) = shape.beziers.get_mut(pid.bezier_idx) {
                match pid.ctrl_idx {
                    0 => {
                        // move p0 and adjust neighbors
                        let dx = new_pos.x - bez.p0.x;
                        let dy = new_pos.y - bez.p0.y;
                        bez.p0 = new_pos;
                        bez.p1 = kurbo::Point::new(bez.p1.x + dx, bez.p1.y + dy);
                        if pid.bezier_idx > 0 {
                            let prev = &mut shape.beziers[pid.bezier_idx - 1];
                            prev.p3 = new_pos;
                            prev.p2 = kurbo::Point::new(prev.p2.x + dx, prev.p2.y + dy);
                        }
                    }
                    1 => {
                        bez.p1 = new_pos;
                    }
                    2 => {
                        bez.p2 = new_pos;
                    }
                    3 => {
                        // move p3 and adjust neighbor
                        let dx = new_pos.x - bez.p3.x;
                        let dy = new_pos.y - bez.p3.y;
                        bez.p3 = new_pos;
                        bez.p2 = kurbo::Point::new(bez.p2.x + dx, bez.p2.y + dy);
                        if pid.bezier_idx + 1 < shape.beziers.len() {
                            let next = &mut shape.beziers[pid.bezier_idx + 1];
                            next.p0 = new_pos;
                            next.p1 = kurbo::Point::new(next.p1.x + dx, next.p1.y + dy);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    // ... other methods: select_single_shape, select_single_point, clear_selection, paint_hover_outline, etc.
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

                ToolKind::Selection => {
                    let mut tool = self
                        .selecting_tool
                        .take()
                        .expect("selecting_tool was None when it shouldn`t be");

                    tool.handle_input(ctx, &response, self);

                    self.selecting_tool = Some(tool);
                }

                ToolKind::Curvature => {
                    let mut tool = self
                        .curvature_tool
                        .take()
                        .expect("curvature_tool was None when it shouldn`t be");

                    tool.handle_input(ctx, &response, self);

                    self.curvature_tool = Some(tool);
                }

                ToolKind::DirectSelection => {
                    let mut tool = self
                        .direct_selecting_tool
                        .take()
                        .expect("direct_selecting_tool was None when it shouldn`t be");

                    tool.handle_input(ctx, &response, self);

                    self.direct_selecting_tool = Some(tool);
                }
            }

            // draw all finished shapes (Béziers, raw, handles) by using world_to_screen() internally —
            for shape in &self.shapes {
                shape.draw_beziers(&painter, self);
            }

            // draw in-progress stroke
            // using this method:
            // https://github.com/emilk/egui/blob/main/crates/egui_demo_lib/src/demo/painting.rs
            if self.curr_shape.current_stroke.len() >= 2 {
                // map all of the raw points into screen-space in one go:
                let pts: Vec<Pos2> = self
                    .curr_shape
                    .current_stroke
                    .iter()
                    .map(|p| self.world_to_screen(*p))
                    .collect();

                // then push one Shape::line with all of them:
                painter.add(egui::Shape::line(
                    pts,
                    egui::Stroke::new(
                        self.curr_shape.thickness * self.zoom,
                        self.curr_shape.stroke_color,
                    ),
                ));
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

            // let the active tool paint any overlays
            match current_tool {
                ToolKind::Drawing => {
                    let mut tool = self
                        .drawing_tool
                        .take()
                        .expect("drawing_tool was None when it shouldn’t be");
                    tool.paint(ctx, &painter, self);
                    self.drawing_tool = Some(tool);
                }
                ToolKind::Panning => {
                    let mut tool = self
                        .panning_tool
                        .take()
                        .expect("panning_tool was None when it shouldn’t be");
                    tool.paint(ctx, &painter, self);
                    self.panning_tool = Some(tool);
                }

                ToolKind::Editing => {
                    let mut tool = self
                        .editing_tool
                        .take()
                        .expect("editing_tool was None when it shouldn`t be");
                    tool.paint(ctx, &painter, self);
                    self.editing_tool = Some(tool);
                }

                ToolKind::Selection => {
                    let mut tool = self
                        .selecting_tool
                        .take()
                        .expect("selecting_tool was None when it shouldn`t be");
                    tool.paint(ctx, &painter, self);
                    self.selecting_tool = Some(tool);
                }

                ToolKind::DirectSelection => {
                    let mut tool = self
                        .direct_selecting_tool
                        .take()
                        .expect("direct_selecting_tool was None when it shouldn`t be");
                    tool.paint(ctx, &painter, self);
                    self.direct_selecting_tool = Some(tool);
                }

                ToolKind::Curvature => {
                    let mut tool = self
                        .curvature_tool
                        .take()
                        .expect("curvature_tool was None when it shouldn`t be");
                    tool.paint(ctx, &painter, self);
                    self.curvature_tool = Some(tool);
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
                    if ui.button("Select").clicked() {
                        self.selected_tool = ToolKind::Selection;
                    }
                    if ui.button("Direct Select").clicked() {
                        self.selected_tool = ToolKind::DirectSelection;
                    }
                    if ui.button("Mold").clicked() {
                        self.selected_tool = ToolKind::Curvature;
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

            ToolKind::Selection => {
                let mut tool = self.selecting_tool.take().expect("selecting_tool was None");
                tool.tool_ui(ctx, self);
                self.selecting_tool = Some(tool);
            }

            ToolKind::DirectSelection => {
                let mut tool = self
                    .direct_selecting_tool
                    .take()
                    .expect("direct_selecting_tool was None");
                tool.tool_ui(ctx, self);
                self.direct_selecting_tool = Some(tool);
            }

            ToolKind::Curvature => {
                let mut tool = self.curvature_tool.take().expect("curvature_tool was None");
                tool.tool_ui(ctx, self);
                self.curvature_tool = Some(tool);
            }
        }
    }
}
