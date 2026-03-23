use crate::abi::{PD_SCREEN_HEIGHT, PD_SCREEN_WIDTH};
use crate::display::ssd1677::{PANEL_HEIGHT, ROW_BYTES};

pub const MAX_SCENE_OPS: usize = 256;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DisplayOp {
    Noop,
    Pixel {
        x: i32,
        y: i32,
        color: u8,
    },
    DrawRect {
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        color: u8,
    },
    FillRect {
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        color: u8,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DisplayScene {
    clear_color: u8,
    ops: [DisplayOp; MAX_SCENE_OPS],
    op_count: usize,
    overflowed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PhysicalRect {
    x0: usize,
    y0: usize,
    x1: usize,
    y1: usize,
}

impl DisplayScene {
    pub const fn new() -> Self {
        Self {
            clear_color: 0xFF,
            ops: [DisplayOp::Noop; MAX_SCENE_OPS],
            op_count: 0,
            overflowed: false,
        }
    }

    pub fn clear_to(&mut self, color: u8) {
        self.clear_color = normalize_color(color);
        self.op_count = 0;
        self.overflowed = false;
    }

    pub fn set_pixel(&mut self, x: i32, y: i32, color: u8) {
        self.push(DisplayOp::Pixel {
            x,
            y,
            color: normalize_color(color),
        });
    }

    pub fn draw_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: u8) {
        self.push(DisplayOp::DrawRect {
            x,
            y,
            w,
            h,
            color: normalize_color(color),
        });
    }

    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: u8) {
        self.push(DisplayOp::FillRect {
            x,
            y,
            w,
            h,
            color: normalize_color(color),
        });
    }

    pub fn clear_color(&self) -> u8 {
        self.clear_color
    }

    pub fn ops(&self) -> &[DisplayOp] {
        &self.ops[..self.op_count]
    }

    pub fn overflowed(&self) -> bool {
        self.overflowed
    }

    fn push(&mut self, op: DisplayOp) {
        if self.op_count < MAX_SCENE_OPS {
            self.ops[self.op_count] = op;
            self.op_count += 1;
        } else {
            self.overflowed = true;
        }
    }
}

impl Default for DisplayScene {
    fn default() -> Self {
        Self::new()
    }
}

#[inline]
pub fn logical_to_physical(x: i32, y: i32) -> Option<(usize, usize)> {
    if x < 0 || y < 0 || x >= PD_SCREEN_WIDTH || y >= PD_SCREEN_HEIGHT {
        return None;
    }

    Some((y as usize, PANEL_HEIGHT as usize - 1 - x as usize))
}

pub fn render_scene_strip(scene: &DisplayScene, row_start: u16, row_count: u16, dst: &mut [u8]) {
    let row_start = row_start as usize;
    let row_count = row_count as usize;
    let byte_count = row_count * ROW_BYTES;
    debug_assert!(dst.len() >= byte_count, "dst must hold row_count rows");

    let dst = &mut dst[..byte_count];
    dst.fill(scene.clear_color());

    for op in scene.ops() {
        match *op {
            DisplayOp::Noop => {}
            DisplayOp::Pixel { x, y, color } => {
                render_pixel(dst, row_start, row_count, x, y, color)
            }
            DisplayOp::DrawRect { x, y, w, h, color } => {
                render_draw_rect(dst, row_start, row_count, x, y, w, h, color)
            }
            DisplayOp::FillRect { x, y, w, h, color } => {
                render_fill_rect(dst, row_start, row_count, x, y, w, h, color)
            }
        }
    }
}

fn normalize_color(color: u8) -> u8 {
    if color == 0 {
        0x00
    } else {
        0xFF
    }
}

fn logical_rect_to_physical(x: i32, y: i32, w: i32, h: i32) -> Option<PhysicalRect> {
    if w <= 0 || h <= 0 {
        return None;
    }

    let x0 = x.max(0).min(PD_SCREEN_WIDTH);
    let y0 = y.max(0).min(PD_SCREEN_HEIGHT);
    let x1 = (x + w).max(0).min(PD_SCREEN_WIDTH);
    let y1 = (y + h).max(0).min(PD_SCREEN_HEIGHT);

    if x0 >= x1 || y0 >= y1 {
        return None;
    }

    Some(PhysicalRect {
        x0: y0 as usize,
        y0: PANEL_HEIGHT as usize - x1 as usize,
        x1: y1 as usize,
        y1: PANEL_HEIGHT as usize - x0 as usize,
    })
}

fn render_pixel(dst: &mut [u8], row_start: usize, row_count: usize, x: i32, y: i32, color: u8) {
    let Some((px, py)) = logical_to_physical(x, y) else {
        return;
    };
    set_strip_pixel(dst, row_start, row_count, px, py, color);
}

fn render_fill_rect(
    dst: &mut [u8],
    row_start: usize,
    row_count: usize,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    color: u8,
) {
    let Some(rect) = logical_rect_to_physical(x, y, w, h) else {
        return;
    };

    let strip_end = row_start + row_count;
    let y0 = rect.y0.max(row_start);
    let y1 = rect.y1.min(strip_end);
    if y0 >= y1 {
        return;
    }

    for py in y0..y1 {
        fill_row_span(dst, py - row_start, rect.x0, rect.x1, color);
    }
}

fn render_draw_rect(
    dst: &mut [u8],
    row_start: usize,
    row_count: usize,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    color: u8,
) {
    let Some(rect) = logical_rect_to_physical(x, y, w, h) else {
        return;
    };

    let width = rect.x1 - rect.x0;
    let height = rect.y1 - rect.y0;
    if width <= 1 || height <= 1 {
        render_fill_rect(dst, row_start, row_count, x, y, w, h, color);
        return;
    }

    let strip_end = row_start + row_count;
    if rect.y0 >= row_start && rect.y0 < strip_end {
        fill_row_span(dst, rect.y0 - row_start, rect.x0, rect.x1, color);
    }
    let bottom = rect.y1 - 1;
    if bottom >= row_start && bottom < strip_end {
        fill_row_span(dst, bottom - row_start, rect.x0, rect.x1, color);
    }

    let side_y0 = rect.y0.saturating_add(1).max(row_start);
    let side_y1 = rect.y1.saturating_sub(1).min(strip_end);
    for py in side_y0..side_y1 {
        set_strip_pixel(dst, row_start, row_count, rect.x0, py, color);
        set_strip_pixel(dst, row_start, row_count, rect.x1 - 1, py, color);
    }
}

fn set_strip_pixel(
    dst: &mut [u8],
    row_start: usize,
    row_count: usize,
    px: usize,
    py: usize,
    color: u8,
) {
    if py < row_start || py >= row_start + row_count {
        return;
    }

    let local_row = py - row_start;
    let byte_idx = local_row * ROW_BYTES + px / 8;
    let bit_mask = 0x80u8 >> (px % 8);
    if color != 0 {
        dst[byte_idx] |= bit_mask;
    } else {
        dst[byte_idx] &= !bit_mask;
    }
}

fn fill_row_span(dst: &mut [u8], local_row: usize, x0: usize, x1: usize, color: u8) {
    if x0 >= x1 {
        return;
    }

    let row = &mut dst[local_row * ROW_BYTES..(local_row + 1) * ROW_BYTES];
    let first_byte = x0 / 8;
    let last_byte = (x1 - 1) / 8;
    let first_mask = 0xFFu8 >> (x0 % 8);
    let last_mask = 0xFFu8 << (7 - ((x1 - 1) % 8));

    if first_byte == last_byte {
        apply_mask(&mut row[first_byte], first_mask & last_mask, color);
        return;
    }

    apply_mask(&mut row[first_byte], first_mask, color);
    let fill = normalize_color(color);
    for byte in &mut row[first_byte + 1..last_byte] {
        *byte = fill;
    }
    apply_mask(&mut row[last_byte], last_mask, color);
}

fn apply_mask(byte: &mut u8, mask: u8, color: u8) {
    if color != 0 {
        *byte |= mask;
    } else {
        *byte &= !mask;
    }
}
