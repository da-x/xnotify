pub mod geometry;
pub mod color;
pub mod formats;

use std::sync::Arc;
use super::error::*;
pub use geometry::Geometry;
pub use color::Color;

// Get the screen from an XCB Connection
pub fn screen(conn: &Arc<xcb::Connection>) -> Result<xcb::Screen> {
    conn.get_setup()
        .roots()
        .next()
        .ok_or_else(|| ErrorKind::XcbNoScreenError(()).into())
}
