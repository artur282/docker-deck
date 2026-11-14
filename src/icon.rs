use eframe::egui;

const DARK_BG: [u8; 4] = [21, 25, 31, 255];
const BAR_TOP: [u8; 4] = [96, 165, 250, 255];
const BAR_MIDDLE: [u8; 4] = [59, 130, 246, 255];
const BAR_BOTTOM: [u8; 4] = [37, 99, 235, 255];

pub fn app_icon() -> egui::IconData {
    const SIZE: u32 = 64;
    let mut rgba = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let [r, g, b, a] = pixel(x, y, SIZE);
            rgba.extend_from_slice(&[r, g, b, a]);
        }
    }
    egui::IconData {
        rgba,
        width: SIZE,
        height: SIZE,
    }
}

pub fn tray_icon() -> ksni::Icon {
    const SIZE: i32 = 32;
    let mut data = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let [r, g, b, a] = pixel(x as u32, y as u32, SIZE as u32);
            // StatusNotifierItem expects ARGB32 in network byte order.
            data.extend_from_slice(&[a, r, g, b]);
        }
    }
    ksni::Icon {
        width: SIZE,
        height: SIZE,
        data,
    }
}

fn pixel(x: u32, y: u32, size: u32) -> [u8; 4] {
    let nx = (x as f32 + 0.5) / size as f32;
    let ny = (y as f32 + 0.5) / size as f32;

    let inside_background = (0.04..=0.96).contains(&nx) && (0.04..=0.96).contains(&ny);
    if !inside_background {
        return [0, 0, 0, 0];
    }

    let bar_left = 0.16;
    let bar_right = 0.84;
    let bars = [
        (0.24, 0.38, BAR_TOP),
        (0.43, 0.57, BAR_MIDDLE),
        (0.62, 0.76, BAR_BOTTOM),
    ];

    if nx >= bar_left && nx <= bar_right {
        for (top, bottom, color) in bars {
            if ny >= top && ny <= bottom {
                return color;
            }
        }
    }

    DARK_BG
}
