use crate::display::ssd1677::{
    DISPLAY_UPDATE_CTRL2, FULL_UPDATE_SEQUENCE, MASTER_ACTIVATION, PANEL_HEIGHT, PANEL_WIDTH,
    PARTIAL_UPDATE_SEQUENCE,
};
use crate::display::transport::DisplayTransport;

/// Pixel-coordinate input for a partial display update.
pub struct PartialRegion {
    /// Left edge of the region in pixels (0-based).
    pub x: u16,
    /// Top edge of the region in pixels (0-based).
    pub y: u16,
    /// Width of the region in pixels.
    pub width: u16,
    /// Height of the region in pixels.
    pub height: u16,
}

/// Partial-update region expressed in SSD1677 address units.
///
/// X fields are byte-column addresses (each unit covers 8 pixels).
/// Y fields are pixel-row addresses.
pub struct NormalizedRegion {
    /// First byte-column of the region (`x / 8` after alignment).
    pub x_byte_start: u8,
    /// Last byte-column of the region (inclusive).
    pub x_byte_end: u8,
    /// First pixel row of the region.
    pub y_start: u16,
    /// Last pixel row of the region (inclusive).
    pub y_end: u16,
}

/// Normalize a partial-update region to SSD1677 address units.
///
/// Clamps the region to panel bounds, aligns X to byte-column boundaries
/// (expands outward to cover any partially-covered byte), and returns
/// `None` if the clamped region is empty.
pub fn normalize_partial_region(r: PartialRegion) -> Option<NormalizedRegion> {
    // Clamp origin to panel bounds.
    if r.x >= PANEL_WIDTH || r.y >= PANEL_HEIGHT || r.width == 0 || r.height == 0 {
        return None;
    }
    // Clamp extents.
    let x_end_px = r.x.saturating_add(r.width).min(PANEL_WIDTH);
    let y_end_row = r.y.saturating_add(r.height).min(PANEL_HEIGHT);
    // Align X to byte-column boundaries (expand outward).
    let x_byte_start = (r.x / 8) as u8;
    let x_byte_end = ((x_end_px - 1) / 8) as u8;
    Some(NormalizedRegion {
        x_byte_start,
        x_byte_end,
        y_start: r.y,
        y_end: y_end_row - 1,
    })
}

/// Tracks partial-refresh cycles and signals when a full refresh is required.
///
/// Ghosting accumulates on e-paper displays after repeated partial updates.
/// `PartialRefreshCounter` counts partial refreshes and returns `true` from
/// [`record_partial`] when the threshold is reached, indicating the caller
/// should run a full refresh instead. The counter resets automatically on
/// promotion.
pub struct PartialRefreshCounter {
    count: u32,
    threshold: u32,
}

impl PartialRefreshCounter {
    /// Create a new counter that promotes to full refresh every `threshold` partial updates.
    pub const fn new(threshold: u32) -> Self {
        Self { count: 0, threshold }
    }

    /// Record one partial refresh.
    ///
    /// Returns `true` when the running total reaches `threshold`, indicating that
    /// the caller should perform a full refresh. The counter resets to zero on promotion.
    pub fn record_partial(&mut self) -> bool {
        self.count += 1;
        if self.count >= self.threshold {
            self.count = 0;
            true
        } else {
            false
        }
    }

    /// Reset the counter without triggering promotion.
    pub fn reset(&mut self) {
        self.count = 0;
    }
}

/// Trigger a DU partial-refresh cycle.
///
/// Writes the partial-update sequence flag to `DISPLAY_UPDATE_CTRL2`, issues
/// `MASTER_ACTIVATION` to start the refresh, then waits for the controller
/// to signal completion via the BUSY line.
pub fn trigger_partial_refresh<T>(transport: &mut T) -> Result<(), T::Error>
where
    T: DisplayTransport,
{
    transport.write_command(DISPLAY_UPDATE_CTRL2)?;
    transport.write_data(&[PARTIAL_UPDATE_SEQUENCE])?;
    transport.write_command(MASTER_ACTIVATION)?;
    transport.wait_while_busy()?;
    Ok(())
}

/// Trigger a full-panel refresh cycle.
///
/// Writes the full-update sequence flag to `DISPLAY_UPDATE_CTRL2`, issues
/// `MASTER_ACTIVATION` to start the refresh, then waits for the controller
/// to signal completion via the BUSY line.
pub fn trigger_full_refresh<T>(transport: &mut T) -> Result<(), T::Error>
where
    T: DisplayTransport,
{
    transport.write_command(DISPLAY_UPDATE_CTRL2)?;
    transport.write_data(&[FULL_UPDATE_SEQUENCE])?;
    transport.write_command(MASTER_ACTIVATION)?;
    transport.wait_while_busy()?;
    Ok(())
}
