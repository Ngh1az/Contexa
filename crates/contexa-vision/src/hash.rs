//! Perceptual hash + hamming distance — ported verbatim from the validated
//! `spikes/SP-02-capture-cpu/src/main.rs` (the algorithm behind the measured
//! CPU numbers in `benchmarks/BASELINE.md`). Docs/05 §5.3/§10.1: "downscale
//! for hashing — 1/4 resolution".

const GRID: usize = 16;

/// 256-bit average hash from BGRA rows, sampling at 1/4 resolution.
#[allow(clippy::cast_possible_truncation)]
pub fn ahash(data: &[u8], width: usize, height: usize, pitch: usize) -> [u64; 4] {
    let mut cells = [0u32; GRID * GRID];
    let mut counts = [0u32; GRID * GRID];
    let (sw, sh) = (width / 4, height / 4); // 1/4 resolution sampling
    for sy in 0..sh {
        let y = sy * 4;
        let row = &data[y * pitch..];
        let cy = sy * GRID / sh.max(1);
        for sx in 0..sw {
            let x = sx * 4;
            let px = &row[x * 4..x * 4 + 3];
            let lum = (u32::from(px[0]) + u32::from(px[1]) * 2 + u32::from(px[2])) / 4; // cheap luma
            let cx = sx * GRID / sw.max(1);
            let idx = (cy.min(GRID - 1)) * GRID + cx.min(GRID - 1);
            cells[idx] += lum;
            counts[idx] += 1;
        }
    }
    let mut avg_all = 0u64;
    let mut vals = [0u32; GRID * GRID];
    for i in 0..GRID * GRID {
        vals[i] = cells[i] / counts[i].max(1);
        avg_all += u64::from(vals[i]);
    }
    // bounded by GRID*GRID (256) samples, each a per-pixel luma <= 255 — fits u32.
    let avg = (avg_all / (GRID * GRID) as u64) as u32;
    let mut hash = [0u64; 4];
    for i in 0..GRID * GRID {
        if vals[i] > avg {
            hash[i / 64] |= 1 << (i % 64);
        }
    }
    hash
}

#[must_use]
pub fn hamming(a: &[u64; 4], b: &[u64; 4]) -> u32 {
    a.iter().zip(b).map(|(x, y)| (x ^ y).count_ones()).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid_bgra(width: usize, height: usize, b: u8, g: u8, r: u8) -> Vec<u8> {
        let pitch = width * 4;
        let mut data = vec![0u8; pitch * height];
        for px in data.chunks_mut(4) {
            px[0] = b;
            px[1] = g;
            px[2] = r;
            px[3] = 255;
        }
        data
    }

    /// Left half dark, right half light (or vice versa) — a uniform-color frame
    /// hashes to all-zero bits (every cell equals the global average), so a
    /// meaningful hash needs contrast within the frame.
    fn split_bgra(width: usize, height: usize, left_dark: bool) -> Vec<u8> {
        let pitch = width * 4;
        let mut data = vec![0u8; pitch * height];
        let (dark, light) = (10u8, 240u8);
        for y in 0..height {
            for x in 0..width {
                let is_left = x < width / 2;
                let v = if is_left == left_dark { dark } else { light };
                let px = &mut data[y * pitch + x * 4..y * pitch + x * 4 + 4];
                px[0] = v;
                px[1] = v;
                px[2] = v;
                px[3] = 255;
            }
        }
        data
    }

    #[test]
    fn identical_frames_have_zero_distance() {
        let data = solid_bgra(64, 64, 100, 150, 200);
        let h1 = ahash(&data, 64, 64, 64 * 4);
        let h2 = ahash(&data, 64, 64, 64 * 4);
        assert_eq!(hamming(&h1, &h2), 0);
    }

    #[test]
    fn flipped_split_frame_exceeds_five_percent_threshold() {
        let before = split_bgra(64, 64, true);
        let after = split_bgra(64, 64, false);
        let h1 = ahash(&before, 64, 64, 64 * 4);
        let h2 = ahash(&after, 64, 64, 64 * 4);
        let diff_pct = f64::from(hamming(&h1, &h2)) / 256.0 * 100.0;
        assert!(diff_pct > 5.0, "expected > 5% change, got {diff_pct:.1}%");
    }
}
