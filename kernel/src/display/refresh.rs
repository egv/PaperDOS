use crate::display::ssd1677::{DISPLAY_UPDATE_CTRL2, MASTER_ACTIVATION};
use crate::display::transport::DisplayTransport;

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
    transport.write_data(&[0xF7])?;
    transport.write_command(MASTER_ACTIVATION)?;
    transport.wait_while_busy()?;
    Ok(())
}
