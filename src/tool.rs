use eframe::egui::{Context, Response, Painter};


/// Each tool must be able to:
/// - handle input events
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
    fn paint(&mut self, painter: &Painter, app: &crate::Shaper);
}
