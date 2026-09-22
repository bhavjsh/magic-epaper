use image::{load_from_memory_with_format, ImageFormat, RgbaImage};
use std::io::Cursor;
use std::sync::OnceLock;
use rayon::prelude::*;

pub enum DitherMethod {
    FloydSteinberg,
    FalseFloydSteinberg,
    Stucki,
    Atkinson,
    Threshold,
    Halftone,
    Bayer,
    Sierra2,
    Burkes,
}

const BAYER_8X8: [[f32; 8]; 8] = [
    [ 0.0, 32.0,  8.0, 40.0,  2.0, 34.0, 10.0, 42.0],
    [48.0, 16.0, 56.0, 24.0, 50.0, 18.0, 58.0, 26.0],
    [12.0, 44.0,  4.0, 36.0, 14.0, 46.0,  6.0, 38.0],
    [60.0, 28.0, 52.0, 20.0, 62.0, 30.0, 54.0, 22.0],
    [ 3.0, 35.0, 11.0, 43.0,  1.0, 33.0,  9.0, 41.0],
    [51.0, 19.0, 59.0, 27.0, 49.0, 17.0, 57.0, 25.0],
    [15.0, 47.0,  7.0, 39.0, 13.0, 45.0,  5.0, 37.0],
    [63.0, 31.0, 55.0, 23.0, 61.0, 29.0, 53.0, 21.0],
];

#[derive(Clone, Copy)]
struct Colorf32 {
    r: f32,
    g: f32,
    b: f32,
}

pub enum ColorMode {
    Bw,
    Bwr,
    Bwry,
}

const PALETTE_BW: [Colorf32; 2] = [
    Colorf32 { r: 0.0, g: 0.0, b: 0.0 },
    Colorf32 { r: 255.0, g: 255.0, b: 255.0 },
];

const PALETTE_BWR: [Colorf32; 3] = [
    Colorf32 { r: 0.0, g: 0.0, b: 0.0 },
    Colorf32 { r: 255.0, g: 255.0, b: 255.0 },
    Colorf32 { r: 255.0, g: 0.0, b: 0.0 },
];

const PALETTE_BWRY: [Colorf32; 4] = [
    Colorf32 { r: 0.0, g: 0.0, b: 0.0 },
    Colorf32 { r: 255.0, g: 255.0, b: 255.0 },
    Colorf32 { r: 255.0, g: 255.0, b: 0.0 },
    Colorf32 { r: 255.0, g: 0.0, b: 0.0 },
];

const DITHER_GAMMA: f32 = 1.5;

#[inline(always)]
fn apply_dither_gamma(c: f32) -> f32 {
    (c / 255.0).powf(DITHER_GAMMA) * 255.0
}

#[inline(always)]
fn srgb_to_linear(c: f32) -> f32 {
    let c = c / 255.0;
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

#[inline(always)]
fn rgb_to_oklab(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let rl = srgb_to_linear(r);
    let gl = srgb_to_linear(g);
    let bl = srgb_to_linear(b);
    let l = (0.4122214708 * rl + 0.5363325363 * gl + 0.0514459929 * bl).cbrt();
    let m = (0.2119034982 * rl + 0.6806995451 * gl + 0.1073969566 * bl).cbrt();
    let s = (0.0883024619 * rl + 0.2817188376 * gl + 0.6299787005 * bl).cbrt();
    (
        0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s,
        1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s,
        0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s,
    )
}

struct OklabEntry {
    l: f32,
    a: f32,
    b: f32,
    rgb: Colorf32,
}

fn oklab_palette_bwry() -> &'static [OklabEntry; 4] {
    static CACHE: OnceLock<[OklabEntry; 4]> = OnceLock::new();
    CACHE.get_or_init(|| {
        let mut i = 0;
        PALETTE_BWRY.map(|c| {
            let _ = i;
            let (l, a, b) = rgb_to_oklab(c.r, c.g, c.b);
            i += 1;
            OklabEntry { l, a, b, rgb: c }
        })
    })
}

#[inline(always)]
fn closest_color_oklab(pixel: Colorf32, palette: &[OklabEntry]) -> Colorf32 {
    let (pl, pa, pb) = rgb_to_oklab(pixel.r, pixel.g, pixel.b);
    let mut min_dist = f32::MAX;
    let mut best = palette[0].rgb;
    for e in palette {
        let dl = pl - e.l;
        let da = pa - e.a;
        let db = pb - e.b;
        let dist = dl * dl + da * da + db * db;
        if dist < min_dist {
            min_dist = dist;
            best = e.rgb;
        }
    }
    best
}

fn dither_gamma_lut() -> &'static [f32; 256] {
    static LUT: OnceLock<[f32; 256]> = OnceLock::new();
    LUT.get_or_init(|| {
        let mut lut = [0.0f32; 256];
        for (i, v) in lut.iter_mut().enumerate() {
            *v = apply_dither_gamma(i as f32);
        }
        lut
    })
}

fn bayer_offset_lut() -> &'static [[f32; 8]; 8] {
    static LUT: OnceLock<[[f32; 8]; 8]> = OnceLock::new();
    LUT.get_or_init(|| {
        let mut t = [[0.0f32; 8]; 8];
        for y in 0..8 {
            for x in 0..8 {
                t[y][x] = (BAYER_8X8[y][x] + 0.5) / 64.0 * 255.0 - 127.5;
            }
        }
        t
    })
}

#[inline(always)]
fn closest_color(pixel: Colorf32, palette: &[Colorf32]) -> Colorf32 {
    let mut min_dist = f32::MAX;
    let mut best_color = palette[0];
    for c in palette {
        let dr = pixel.r - c.r;
        let dg = pixel.g - c.g;
        let db = pixel.b - c.b;
        let dist = dr * dr + dg * dg + db * db;
        if dist < min_dist {
            min_dist = dist;
            best_color = *c;
        }
    }
    best_color
}

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}

pub fn process_image_rust(
    image_bytes: Vec<u8>,
    target_width: u32,
    target_height: u32,
    method: DitherMethod,
    color_mode: ColorMode,
) -> Vec<u8> {
    let dynamic_img = load_from_memory_with_format(&image_bytes, ImageFormat::Png)
        .expect("Failed to decode image")
        .resize_exact(target_width, target_height, image::imageops::FilterType::Nearest);

    let img = dynamic_img.to_rgba8();
    let (width, height) = img.dimensions();
    let w = width as usize;
    let h = height as usize;

    let mut buffer: Vec<Colorf32> = img.pixels()
        .map(|p| Colorf32 { r: p[0] as f32, g: p[1] as f32, b: p[2] as f32 })
        .collect();

    if !matches!(method, DitherMethod::Threshold) && !matches!(color_mode, ColorMode::Bwry) {
        let gamma_lut = dither_gamma_lut();
        buffer.par_iter_mut().for_each(|px| {
            px.r = gamma_lut[px.r.clamp(0.0, 255.0) as usize];
            px.g = gamma_lut[px.g.clamp(0.0, 255.0) as usize];
            px.b = gamma_lut[px.b.clamp(0.0, 255.0) as usize];
        });
    }

    let palette: &[Colorf32] = match color_mode {
        ColorMode::Bw => &PALETTE_BW[..],
        ColorMode::Bwr => &PALETTE_BWR[..],
        ColorMode::Bwry => &PALETTE_BWRY[..],
    };
    let use_oklab = matches!(color_mode, ColorMode::Bwry);

    macro_rules! snap {
        ($px:expr) => {
            if use_oklab {
                closest_color_oklab($px, oklab_palette_bwry())
            } else {
                closest_color($px, palette)
            }
        };
    }

    match method {
        DitherMethod::Threshold => {
            buffer.par_iter_mut().for_each(|px| {
                *px = snap!(*px);
            });
        }
        DitherMethod::Bayer => {
            if w > 0 {
                let offsets = bayer_offset_lut();
                buffer.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
                    let brow = &offsets[y & 7];
                    for (x, px) in row.iter_mut().enumerate() {
                        let off = brow[x & 7];
                        *px = snap!(Colorf32 { r: px.r + off, g: px.g + off, b: px.b + off });
                    }
                });
            }
        }
        _ => {
            let ptr = buffer.as_mut_ptr();
            for y in 0..h {
                for x in 0..w {
                    let idx = y * w + x;
                    let old_pixel = unsafe { *ptr.add(idx) };
                    let new_pixel = snap!(old_pixel);
                    unsafe { ptr.add(idx).write(new_pixel) };
                    let er = old_pixel.r - new_pixel.r;
                    let eg = old_pixel.g - new_pixel.g;
                    let eb = old_pixel.b - new_pixel.b;

                    match method {
                        DitherMethod::FloydSteinberg | DitherMethod::Halftone => {
                            if x >= 1 && x + 1 < w && y + 1 < h {
                                unsafe {
                                    add_err(ptr, idx + 1,     er, eg, eb, 7.0 / 16.0);
                                    add_err(ptr, idx + w - 1, er, eg, eb, 3.0 / 16.0);
                                    add_err(ptr, idx + w,     er, eg, eb, 5.0 / 16.0);
                                    add_err(ptr, idx + w + 1, er, eg, eb, 1.0 / 16.0);
                                }
                            } else {
                                distribute_error(ptr, x, y, w, h,  1,  0, er, eg, eb, 7.0 / 16.0);
                                distribute_error(ptr, x, y, w, h, -1,  1, er, eg, eb, 3.0 / 16.0);
                                distribute_error(ptr, x, y, w, h,  0,  1, er, eg, eb, 5.0 / 16.0);
                                distribute_error(ptr, x, y, w, h,  1,  1, er, eg, eb, 1.0 / 16.0);
                            }
                        }
                        DitherMethod::FalseFloydSteinberg => {
                            if x + 1 < w && y + 1 < h {
                                unsafe {
                                    add_err(ptr, idx + 1,     er, eg, eb, 3.0 / 8.0);
                                    add_err(ptr, idx + w,     er, eg, eb, 3.0 / 8.0);
                                    add_err(ptr, idx + w + 1, er, eg, eb, 2.0 / 8.0);
                                }
                            } else {
                                distribute_error(ptr, x, y, w, h, 1, 0, er, eg, eb, 3.0 / 8.0);
                                distribute_error(ptr, x, y, w, h, 0, 1, er, eg, eb, 3.0 / 8.0);
                                distribute_error(ptr, x, y, w, h, 1, 1, er, eg, eb, 2.0 / 8.0);
                            }
                        }
                        DitherMethod::Atkinson => {
                            let w8 = 1.0 / 8.0;
                            if x >= 1 && x + 2 < w && y + 2 < h {
                                unsafe {
                                    add_err(ptr, idx + 1,         er, eg, eb, w8);
                                    add_err(ptr, idx + 2,         er, eg, eb, w8);
                                    add_err(ptr, idx + w - 1,     er, eg, eb, w8);
                                    add_err(ptr, idx + w,         er, eg, eb, w8);
                                    add_err(ptr, idx + w + 1,     er, eg, eb, w8);
                                    add_err(ptr, idx + 2 * w,     er, eg, eb, w8);
                                }
                            } else {
                                distribute_error(ptr, x, y, w, h,  1, 0, er, eg, eb, w8);
                                distribute_error(ptr, x, y, w, h,  2, 0, er, eg, eb, w8);
                                distribute_error(ptr, x, y, w, h, -1, 1, er, eg, eb, w8);
                                distribute_error(ptr, x, y, w, h,  0, 1, er, eg, eb, w8);
                                distribute_error(ptr, x, y, w, h,  1, 1, er, eg, eb, w8);
                                distribute_error(ptr, x, y, w, h,  0, 2, er, eg, eb, w8);
                            }
                        }
                        DitherMethod::Stucki => {
                            let w42 = 1.0 / 42.0;
                            if x >= 2 && x + 2 < w && y + 2 < h {
                                unsafe {
                                    add_err(ptr, idx + 1,         er, eg, eb, 8.0 * w42);
                                    add_err(ptr, idx + 2,         er, eg, eb, 4.0 * w42);
                                    add_err(ptr, idx + w - 2,     er, eg, eb, 2.0 * w42);
                                    add_err(ptr, idx + w - 1,     er, eg, eb, 4.0 * w42);
                                    add_err(ptr, idx + w,         er, eg, eb, 8.0 * w42);
                                    add_err(ptr, idx + w + 1,     er, eg, eb, 4.0 * w42);
                                    add_err(ptr, idx + w + 2,     er, eg, eb, 2.0 * w42);
                                    add_err(ptr, idx + 2 * w - 2, er, eg, eb, 1.0 * w42);
                                    add_err(ptr, idx + 2 * w - 1, er, eg, eb, 2.0 * w42);
                                    add_err(ptr, idx + 2 * w,     er, eg, eb, 4.0 * w42);
                                    add_err(ptr, idx + 2 * w + 1, er, eg, eb, 2.0 * w42);
                                    add_err(ptr, idx + 2 * w + 2, er, eg, eb, 1.0 * w42);
                                }
                            } else {
                                distribute_error(ptr, x, y, w, h,  1, 0, er, eg, eb, 8.0 * w42);
                                distribute_error(ptr, x, y, w, h,  2, 0, er, eg, eb, 4.0 * w42);
                                distribute_error(ptr, x, y, w, h, -2, 1, er, eg, eb, 2.0 * w42);
                                distribute_error(ptr, x, y, w, h, -1, 1, er, eg, eb, 4.0 * w42);
                                distribute_error(ptr, x, y, w, h,  0, 1, er, eg, eb, 8.0 * w42);
                                distribute_error(ptr, x, y, w, h,  1, 1, er, eg, eb, 4.0 * w42);
                                distribute_error(ptr, x, y, w, h,  2, 1, er, eg, eb, 2.0 * w42);
                                distribute_error(ptr, x, y, w, h, -2, 2, er, eg, eb, 1.0 * w42);
                                distribute_error(ptr, x, y, w, h, -1, 2, er, eg, eb, 2.0 * w42);
                                distribute_error(ptr, x, y, w, h,  0, 2, er, eg, eb, 4.0 * w42);
                                distribute_error(ptr, x, y, w, h,  1, 2, er, eg, eb, 2.0 * w42);
                                distribute_error(ptr, x, y, w, h,  2, 2, er, eg, eb, 1.0 * w42);
                            }
                        }
                        DitherMethod::Sierra2 => {
                            let w16 = 1.0 / 16.0;
                            if x >= 2 && x + 2 < w && y + 1 < h {
                                unsafe {
                                    add_err(ptr, idx + 1,     er, eg, eb, 4.0 * w16);
                                    add_err(ptr, idx + 2,     er, eg, eb, 3.0 * w16);
                                    add_err(ptr, idx + w - 2, er, eg, eb, 1.0 * w16);
                                    add_err(ptr, idx + w - 1, er, eg, eb, 2.0 * w16);
                                    add_err(ptr, idx + w,     er, eg, eb, 3.0 * w16);
                                    add_err(ptr, idx + w + 1, er, eg, eb, 2.0 * w16);
                                    add_err(ptr, idx + w + 2, er, eg, eb, 1.0 * w16);
                                }
                            } else {
                                distribute_error(ptr, x, y, w, h,  1, 0, er, eg, eb, 4.0 * w16);
                                distribute_error(ptr, x, y, w, h,  2, 0, er, eg, eb, 3.0 * w16);
                                distribute_error(ptr, x, y, w, h, -2, 1, er, eg, eb, 1.0 * w16);
                                distribute_error(ptr, x, y, w, h, -1, 1, er, eg, eb, 2.0 * w16);
                                distribute_error(ptr, x, y, w, h,  0, 1, er, eg, eb, 3.0 * w16);
                                distribute_error(ptr, x, y, w, h,  1, 1, er, eg, eb, 2.0 * w16);
                                distribute_error(ptr, x, y, w, h,  2, 1, er, eg, eb, 1.0 * w16);
                            }
                        }
                        DitherMethod::Burkes => {
                            let w32 = 1.0 / 32.0;
                            if x >= 2 && x + 2 < w && y + 1 < h {
                                unsafe {
                                    add_err(ptr, idx + 1,     er, eg, eb, 8.0 * w32);
                                    add_err(ptr, idx + 2,     er, eg, eb, 4.0 * w32);
                                    add_err(ptr, idx + w - 2, er, eg, eb, 2.0 * w32);
                                    add_err(ptr, idx + w - 1, er, eg, eb, 4.0 * w32);
                                    add_err(ptr, idx + w,     er, eg, eb, 8.0 * w32);
                                    add_err(ptr, idx + w + 1, er, eg, eb, 4.0 * w32);
                                    add_err(ptr, idx + w + 2, er, eg, eb, 2.0 * w32);
                                }
                            } else {
                                distribute_error(ptr, x, y, w, h,  1, 0, er, eg, eb, 8.0 * w32);
                                distribute_error(ptr, x, y, w, h,  2, 0, er, eg, eb, 4.0 * w32);
                                distribute_error(ptr, x, y, w, h, -2, 1, er, eg, eb, 2.0 * w32);
                                distribute_error(ptr, x, y, w, h, -1, 1, er, eg, eb, 4.0 * w32);
                                distribute_error(ptr, x, y, w, h,  0, 1, er, eg, eb, 8.0 * w32);
                                distribute_error(ptr, x, y, w, h,  1, 1, er, eg, eb, 4.0 * w32);
                                distribute_error(ptr, x, y, w, h,  2, 1, er, eg, eb, 2.0 * w32);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    let raw: Vec<u8> = buffer.iter().flat_map(|c| [
        c.r.clamp(0.0, 255.0) as u8,
        c.g.clamp(0.0, 255.0) as u8,
        c.b.clamp(0.0, 255.0) as u8,
        255u8,
    ]).collect();

    let out_img = RgbaImage::from_raw(width, height, raw).expect("Buffer size mismatch");
    let mut png_bytes: Vec<u8> = Vec::new();
    out_img.write_to(&mut Cursor::new(&mut png_bytes), ImageFormat::Png).expect("Failed to encode PNG");
    png_bytes
}

#[inline(always)]
unsafe fn add_err(ptr: *mut Colorf32, idx: usize, er: f32, eg: f32, eb: f32, weight: f32) {
    let p = &mut *ptr.add(idx);
    p.r += er * weight;
    p.g += eg * weight;
    p.b += eb * weight;
}

#[inline(always)]
fn distribute_error(ptr: *mut Colorf32, x: usize, y: usize, w: usize, h: usize, dx: i32, dy: i32, er: f32, eg: f32, eb: f32, weight: f32) {
    let nx = x as i32 + dx;
    let ny = y as i32 + dy;
    if nx >= 0 && (nx as usize) < w && ny >= 0 && (ny as usize) < h {
        unsafe { add_err(ptr, ny as usize * w + nx as usize, er, eg, eb, weight) };
    }
}
