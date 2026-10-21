/// Copper ring tray icon generated at runtime — no bundled opaque binary.
pub fn tray_rgba() -> (Vec<u8>, u32, u32) {
    let size = 32u32;
    let mut px = vec![0u8; (size * size * 4) as usize];
    let cx = (size as i32 - 1) as f32 / 2.0;
    let cy = cx;
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let r = (dx * dx + dy * dy).sqrt();
            let i = ((y * size + x) * 4) as usize;
            if (r - 10.0).abs() < 2.2 || r < 3.4 {
                px[i] = 212;
                px[i + 1] = 137;
                px[i + 2] = 74;
                px[i + 3] = 255;
            } else if r < 13.0 {
                px[i] = 20;
                px[i + 1] = 22;
                px[i + 2] = 28;
                px[i + 3] = 220;
            }
        }
    }
    (px, size, size)
}
