use kernel::display::render::strip_geometry;
use kernel::display::scene::{render_scene_strip, DisplayScene};
use kernel::display::ssd1677::{PANEL_HEIGHT, ROW_BYTES, STRIP_BUFFER_BYTES};
use kernel::syscall::display::{
    display_clear_to, draw_rect_in, fill_rect_in, set_pixel_in, FrameBuffer, FRAME_BYTES,
};

fn zero_buf() -> Box<FrameBuffer> {
    vec![0u8; FRAME_BYTES]
        .into_boxed_slice()
        .try_into()
        .unwrap()
}

fn render_scene_to_frame(scene: &DisplayScene) -> Box<FrameBuffer> {
    let mut frame = zero_buf();
    let geo = strip_geometry(PANEL_HEIGHT as usize);
    let mut strip = [0u8; STRIP_BUFFER_BYTES];

    for strip_idx in 0..geo.strip_count {
        let row_start = strip_idx * geo.rows_per_strip;
        let row_count = if strip_idx + 1 == geo.strip_count {
            geo.last_strip_rows
        } else {
            geo.rows_per_strip
        };
        let byte_count = row_count * ROW_BYTES;
        render_scene_strip(
            scene,
            row_start as u16,
            row_count as u16,
            &mut strip[..byte_count],
        );
        let dst_start = row_start * ROW_BYTES;
        let dst_end = dst_start + byte_count;
        frame[dst_start..dst_end].copy_from_slice(&strip[..byte_count]);
    }

    frame
}

#[test]
fn retained_scene_matches_framebuffer_reference_display_scene() {
    let mut expected = zero_buf();
    display_clear_to(&mut expected, 0xFF);
    fill_rect_in(&mut expected, 10, 12, 40, 70, 0x00);
    draw_rect_in(&mut expected, 3, 200, 100, 60, 0xFF);
    set_pixel_in(&mut expected, 479, 799, 0x00);
    set_pixel_in(&mut expected, 0, 0, 0x00);

    let mut scene = DisplayScene::new();
    scene.clear_to(0xFF);
    scene.fill_rect(10, 12, 40, 70, 0x00);
    scene.draw_rect(3, 200, 100, 60, 0xFF);
    scene.set_pixel(479, 799, 0x00);
    scene.set_pixel(0, 0, 0x00);

    let actual = render_scene_to_frame(&scene);
    assert_eq!(&*actual, &*expected);
}

#[test]
fn retained_scene_clear_drops_previous_ops_display_scene() {
    let mut expected = zero_buf();
    display_clear_to(&mut expected, 0x00);

    let mut scene = DisplayScene::new();
    scene.clear_to(0xFF);
    scene.fill_rect(0, 0, 20, 20, 0x00);
    scene.clear_to(0x00);

    let actual = render_scene_to_frame(&scene);
    assert_eq!(&*actual, &*expected);
}

#[test]
fn retained_scene_tracks_overflow_display_scene() {
    let mut scene = DisplayScene::new();
    scene.clear_to(0xFF);
    for _ in 0..300 {
        scene.set_pixel(0, 0, 0x00);
    }
    assert!(
        scene.overflowed(),
        "scene should report when the retained op list overflows"
    );
}
