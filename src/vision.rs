//! Finding a button picture on the game screen, in plain Rust.
//!
//! This is the original's `fo3bot.matcher.find` rebuilt: OpenCV's
//! `matchTemplate(TM_CCOEFF_NORMED)` on grey pictures, a first look at
//! half size on big screens (COARSE = 2, keep COARSE_PEAKS = 3 spots) and a
//! re-check at full size around each spot (COARSE_MARGIN), see matcher.rs.

use crate::matcher::{COARSE, COARSE_MARGIN, COARSE_MIN_AREA, COARSE_PEAKS};

/// A grey picture, one f32 (0..255) per pixel, row by row.
#[derive(Clone)]
pub struct Gray {
    pub w: usize,
    pub h: usize,
    pub px: Vec<f32>,
}

/// Where a picture was found: its top-left corner and size in screen
/// pixels, and how alike it is (-1..1, TM_CCOEFF_NORMED).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
    pub score: f32,
}

impl Gray {
    /// From RGBA bytes (OpenCV's grey: 0.299 R + 0.587 G + 0.114 B).
    pub fn from_rgba(w: usize, h: usize, rgba: &[u8]) -> Self {
        let px = rgba
            .chunks_exact(4)
            .map(|p| 0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32)
            .collect();
        Self { w, h, px }
    }

    /// From BGRA bytes (what Windows screen capture gives).
    pub fn from_bgra(w: usize, h: usize, bgra: &[u8]) -> Self {
        let px = bgra
            .chunks_exact(4)
            .map(|p| 0.299 * p[2] as f32 + 0.587 * p[1] as f32 + 0.114 * p[0] as f32)
            .collect();
        Self { w, h, px }
    }

    pub fn load(path: &std::path::Path) -> Option<Self> {
        let img = image::open(path).ok()?.to_rgba8();
        let (w, h) = img.dimensions();
        Some(Self::from_rgba(w as usize, h as usize, img.as_raw()))
    }

    /// Part of the picture (clamped to it).
    pub fn crop(&self, x: usize, y: usize, w: usize, h: usize) -> Self {
        let x = x.min(self.w);
        let y = y.min(self.h);
        let w = w.min(self.w - x);
        let h = h.min(self.h - y);
        let mut px = Vec::with_capacity(w * h);
        for r in y..y + h {
            px.extend_from_slice(&self.px[r * self.w + x..r * self.w + x + w]);
        }
        Self { w, h, px }
    }

    /// Resize to (nw, nh): area averaging when shrinking, bilinear when
    /// growing (the original uses INTER_AREA / INTER_CUBIC).
    pub fn resize(&self, nw: usize, nh: usize) -> Self {
        let nw = nw.max(1);
        let nh = nh.max(1);
        if nw == self.w && nh == self.h {
            return self.clone();
        }
        let sx = self.w as f32 / nw as f32;
        let sy = self.h as f32 / nh as f32;
        let mut px = vec![0.0; nw * nh];
        if sx >= 1.0 && sy >= 1.0 {
            for y in 0..nh {
                let y0 = y as f32 * sy;
                let y1 = y0 + sy;
                for x in 0..nw {
                    let x0 = x as f32 * sx;
                    let x1 = x0 + sx;
                    let (mut sum, mut wsum) = (0.0, 0.0);
                    let mut yy = y0.floor() as usize;
                    while (yy as f32) < y1 && yy < self.h {
                        let wy = (y1.min(yy as f32 + 1.0) - y0.max(yy as f32)).max(0.0);
                        let mut xx = x0.floor() as usize;
                        while (xx as f32) < x1 && xx < self.w {
                            let wx = (x1.min(xx as f32 + 1.0) - x0.max(xx as f32)).max(0.0);
                            sum += self.px[yy * self.w + xx] * wx * wy;
                            wsum += wx * wy;
                            xx += 1;
                        }
                        yy += 1;
                    }
                    px[y * nw + x] = if wsum > 0.0 { sum / wsum } else { 0.0 };
                }
            }
        } else {
            for y in 0..nh {
                let fy = ((y as f32 + 0.5) * sy - 0.5).clamp(0.0, (self.h - 1) as f32);
                let y0 = fy.floor() as usize;
                let y1 = (y0 + 1).min(self.h - 1);
                let ty = fy - y0 as f32;
                for x in 0..nw {
                    let fx = ((x as f32 + 0.5) * sx - 0.5).clamp(0.0, (self.w - 1) as f32);
                    let x0 = fx.floor() as usize;
                    let x1 = (x0 + 1).min(self.w - 1);
                    let tx = fx - x0 as f32;
                    let a = self.px[y0 * self.w + x0] * (1.0 - tx) + self.px[y0 * self.w + x1] * tx;
                    let b = self.px[y1 * self.w + x0] * (1.0 - tx) + self.px[y1 * self.w + x1] * tx;
                    px[y * nw + x] = a * (1.0 - ty) + b * ty;
                }
            }
        }
        Self { w: nw, h: nh, px }
    }

    /// Resize by a factor (`max(1, round(w*s))`, sub_18016bac0).
    pub fn scaled(&self, s: f32) -> Self {
        let (w, h) = crate::matcher::scaled_size(self.w as u32, self.h as u32, s, s);
        self.resize(w as usize, h as usize)
    }
}

/// TM_CCOEFF_NORMED score map of `tpl` over the part of `img` whose
/// top-left corners lie in [x0, x1) × [y0, y1). Returns (x, y, score)
/// of every position, row by row.
fn ncc_map(img: &Gray, tpl: &Gray, x0: usize, y0: usize, x1: usize, y1: usize) -> Vec<(usize, usize, f32)> {
    let (tw, th) = (tpl.w, tpl.h);
    if tw > img.w || th > img.h {
        return Vec::new();
    }
    let x1 = x1.min(img.w - tw + 1);
    let y1 = y1.min(img.h - th + 1);
    if x0 >= x1 || y0 >= y1 {
        return Vec::new();
    }
    let n = (tw * th) as f64;
    let tmean = tpl.px.iter().map(|&v| v as f64).sum::<f64>() / n;
    let tz: Vec<f32> = tpl.px.iter().map(|&v| v - tmean as f32).collect();
    let tnorm = tz.iter().map(|&v| (v as f64) * (v as f64)).sum::<f64>().sqrt();

    // Integral images of the region (sum and sum of squares).
    let rw = x1 - x0 + tw - 1;
    let rh = y1 - y0 + th - 1;
    let iw = rw + 1;
    let mut s1 = vec![0f64; iw * (rh + 1)];
    let mut s2 = vec![0f64; iw * (rh + 1)];
    for y in 0..rh {
        let (mut a, mut b) = (0f64, 0f64);
        for x in 0..rw {
            let v = img.px[(y0 + y) * img.w + x0 + x] as f64;
            a += v;
            b += v * v;
            s1[(y + 1) * iw + x + 1] = s1[y * iw + x + 1] + a;
            s2[(y + 1) * iw + x + 1] = s2[y * iw + x + 1] + b;
        }
    }
    let rect = |s: &Vec<f64>, x: usize, y: usize| {
        s[(y + th) * iw + x + tw] - s[y * iw + x + tw] - s[(y + th) * iw + x] + s[y * iw + x]
    };

    let rows: Vec<usize> = (y0..y1).collect();
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get()).min(16);
    let chunk = rows.len().div_ceil(threads).max(1);
    let mut out = Vec::with_capacity((x1 - x0) * (y1 - y0));
    std::thread::scope(|sc| {
        let jobs: Vec<_> = rows
            .chunks(chunk)
            .map(|part| {
                let (tz, s1, s2) = (&tz, &s1, &s2);
                sc.spawn(move || {
                    let mut res = Vec::with_capacity(part.len() * (x1 - x0));
                    for &y in part {
                        for x in x0..x1 {
                            let mut dot = 0f32;
                            for ty in 0..th {
                                let row = &img.px[(y + ty) * img.w + x..(y + ty) * img.w + x + tw];
                                let trow = &tz[ty * tw..ty * tw + tw];
                                dot += row.iter().zip(trow).map(|(a, b)| a * b).sum::<f32>();
                            }
                            let (rx, ry) = (x - x0, y - y0);
                            let sum = rect(s1, rx, ry);
                            let sq = rect(s2, rx, ry);
                            let var = (sq - sum * sum / n).max(0.0);
                            let den = var.sqrt() * tnorm;
                            // NaN / infinity count as 0, like the original's best_match.
                            // flat spots (no texture) can't match anything
                            let score = if var > n * 0.25 && den > 1e-6 { ((dot as f64 / den) as f32).clamp(-1.0, 1.0) } else { 0.0 };
                            res.push((x, y, if score.is_finite() { score } else { 0.0 }));
                        }
                    }
                    res
                })
            })
            .collect();
        for j in jobs {
            out.extend(j.join().unwrap_or_default());
        }
    });
    out
}

/// Best `k` spots that aren't next to each other.
fn peaks(map: &[(usize, usize, f32)], k: usize, min_dx: usize, min_dy: usize) -> Vec<(usize, usize, f32)> {
    let mut sorted: Vec<_> = map.to_vec();
    sorted.sort_by(|a, b| b.2.total_cmp(&a.2));
    let mut out: Vec<(usize, usize, f32)> = Vec::new();
    for p in sorted {
        if out.len() >= k {
            break;
        }
        if out.iter().all(|q| p.0.abs_diff(q.0) >= min_dx || p.1.abs_diff(q.1) >= min_dy) {
            out.push(p);
        }
    }
    out
}

/// Every place `tpl` appears with at least `min_score`, best first,
/// up to `max_hits`. Big screens get the half-size first look.
pub fn find_all(img: &Gray, tpl: &Gray, min_score: f32, max_hits: usize) -> Vec<Hit> {
    if tpl.w > img.w || tpl.h > img.h || tpl.w < 2 || tpl.h < 2 {
        return Vec::new();
    }
    let coarse = (img.w * img.h) as u64 >= COARSE_MIN_AREA && tpl.w >= 8 && tpl.h >= 8;
    let candidates: Vec<(usize, usize)> = if coarse {
        let c = COARSE as usize;
        let small = img.resize(img.w / c, img.h / c);
        let st = tpl.resize(tpl.w / c, tpl.h / c);
        let map = ncc_map(&small, &st, 0, 0, small.w, small.h);
        let k = COARSE_PEAKS.max(max_hits * 2);
        peaks(&map, k, st.w.max(1), st.h.max(1))
            .into_iter()
            .map(|(x, y, _)| (x * c, y * c))
            .collect()
    } else {
        let map = ncc_map(img, tpl, 0, 0, img.w, img.h);
        return peaks(&map, max_hits, tpl.w, tpl.h)
            .into_iter()
            .filter(|p| p.2 >= min_score)
            .map(|(x, y, s)| Hit { x, y, w: tpl.w, h: tpl.h, score: s })
            .collect();
    };
    // Re-check each spot at full size with a border.
    let mx = ((tpl.w as f32 * COARSE_MARGIN) as usize).max(COARSE as usize * 2);
    let my = ((tpl.h as f32 * COARSE_MARGIN) as usize).max(COARSE as usize * 2);
    let mut hits: Vec<Hit> = Vec::new();
    for (cx, cy) in candidates {
        let map = ncc_map(img, tpl, cx.saturating_sub(mx), cy.saturating_sub(my), cx + mx + 1, cy + my + 1);
        if let Some(best) = map.iter().max_by(|a, b| a.2.total_cmp(&b.2)) {
            if best.2 >= min_score
                && hits.iter().all(|h| h.x.abs_diff(best.0) >= tpl.w || h.y.abs_diff(best.1) >= tpl.h)
            {
                hits.push(Hit { x: best.0, y: best.1, w: tpl.w, h: tpl.h, score: best.2 });
            }
        }
    }
    hits.sort_by(|a, b| b.score.total_cmp(&a.score));
    hits.truncate(max_hits);
    hits
}

/// The single best place of `tpl` (any score).
pub fn find_best(img: &Gray, tpl: &Gray) -> Option<Hit> {
    find_all(img, tpl, -1.0, 1).into_iter().next()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn noise(w: usize, h: usize, seed: u32) -> Gray {
        let mut s = seed;
        let px = (0..w * h)
            .map(|_| {
                s = s.wrapping_mul(1664525).wrapping_add(1013904223);
                (s >> 24) as f32
            })
            .collect();
        Gray { w, h, px }
    }

    #[test]
    fn finds_a_piece_of_itself() {
        let img = noise(700, 520, 7);
        let tpl = img.crop(333, 211, 40, 18);
        let hit = find_best(&img, &tpl).unwrap();
        assert_eq!((hit.x, hit.y), (333, 211));
        assert!(hit.score > 0.99);
        // a smooth picture still matches after resizing
        let smooth = img.resize(350, 260).resize(700, 520);
        let t2 = smooth.crop(100, 300, 60, 30).scaled(1.25);
        let big = smooth.scaled(1.25);
        let h2 = find_best(&big, &t2).unwrap();
        assert!(h2.x.abs_diff(125) <= 2 && h2.y.abs_diff(375) <= 2, "{h2:?}");
    }
}
