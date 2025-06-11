use eframe::egui::{Context, Response, Painter};

/// Each tool must be able to:
/// - handle input events
/// - draw tool specific UI elements
/// - draw itself onto the `painter`
/// - optionally modify the app state (e.g. current shape, shape list, camera transform, etc.)

pub trait Tool {
    /// called once per frame; let the tool inspect input, mutate app state, etc.
    fn handle_input(
        &mut self,
        ctx: &Context,
        response: &Response,
        app: &mut crate::Shaper,
    );

    /// called after input, to let the tool draw any custom UI or decorations.
    /// draw in-progress strokes, pan hints, etc.
    fn paint(&mut self,  ctx: &Context, painter: &Painter, app: &crate::Shaper);

    
    // draw specific UI elements
    fn tool_ui(&mut self, ctx: &Context, app: &mut crate::Shaper);
}


pub mod tools {
    pub use crate::tools::selecting_tool::SelectionTool;
    pub use crate::tools::panning_tool::PanningTool;
    pub use crate::tools::drawing_tool::DrawingTool;
    pub use crate::tools::editing_tool::EditingTool;
    pub use crate::tools::curvature_tool::CurvatureTool;
    pub use crate::tools::direct_selecting_tool::DirectSelectionTool;
}