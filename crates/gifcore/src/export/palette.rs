//! 256-colour palette: port of gif-h's median-split k-d tree.
//!
//! Same algorithm as v2's vendored `gif.h` (`GifMakePalette`, `GifSplitPalette`,
//! `GifGetClosestPaletteColor`), working on RGB triplets instead of RGBA (gif-h
//! ignores alpha). Entry 0 is the transparent colour and is never matched.
//! The palette is built from the whole frame, as v2 does, and starts zeroed,
//! so entries of empty subtrees are black.

/// Pixels per palette (gif-h `bitDepth = 8`).
pub const NUM_COLORS: usize = 256;
/// Palette index used for "same as the previous frame" pixels.
pub const TRANSPARENT_INDEX: u8 = 0;

/// A palette plus the k-d tree used to search it.
///
/// Node `i` has children `2i` and `2i + 1`; nodes `256..512` are the leaves,
/// leaf `256 + e` holds colour `e`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Palette {
    /// Colours, index 0 is transparent (always black).
    pub colors: [[u8; 3]; NUM_COLORS],
    split_elt: [u8; NUM_COLORS],
    split: [u8; NUM_COLORS],
}

impl Palette {
    /// Builds a palette from RGB pixels (gif-h `GifMakePalette` with no
    /// previous frame). `for_dither` keeps the darkest and lightest colours.
    pub fn build(pixels: &[[u8; 3]], for_dither: bool) -> Self {
        let mut pal = Palette {
            colors: [[0; 3]; NUM_COLORS],
            split_elt: [0; NUM_COLORS],
            split: [0; NUM_COLORS],
        };
        // The split is destructive (it reorders pixels), so work on a copy.
        let mut work = pixels.to_vec();
        pal.split_node(&mut work, 1, 0, for_dither);
        // The bottom node for the transparency index.
        pal.split[NUM_COLORS / 2] = 0;
        pal.split_elt[NUM_COLORS / 2] = 0;
        pal.colors[usize::from(TRANSPARENT_INDEX)] = [0; 3];
        pal
    }

    /// Index of the closest non-transparent colour by L1 distance, starting
    /// from `best` (index, distance), as gif-h's tree walk does.
    pub fn closest(&self, rgb: [i32; 3], best: (u8, i32)) -> (u8, i32) {
        let mut best = best;
        self.search(rgb, &mut best, 1);
        best
    }

    fn search(&self, rgb: [i32; 3], best: &mut (u8, i32), node: usize) {
        if node >= NUM_COLORS {
            let ind = node - NUM_COLORS;
            if ind == usize::from(TRANSPARENT_INDEX) {
                return;
            }
            let diff: i32 = rgb
                .iter()
                .zip(self.colors[ind])
                .map(|(&want, have)| (want - i32::from(have)).abs())
                .sum();
            if diff < best.1 {
                *best = (ind as u8, diff);
            }
            return;
        }
        let comp = rgb[usize::from(self.split_elt[node])];
        let pos = i32::from(self.split[node]);
        if pos > comp {
            self.search(rgb, best, node * 2);
            if best.1 > pos - comp {
                self.search(rgb, best, node * 2 + 1);
            }
        } else {
            self.search(rgb, best, node * 2 + 1);
            if best.1 > comp - pos {
                self.search(rgb, best, node * 2);
            }
        }
    }

    /// gif-h `GifSplitPalette`.
    fn split_node(&mut self, px: &mut [[u8; 3]], node: usize, level: u32, for_dither: bool) {
        if px.is_empty() {
            return;
        }
        if node >= NUM_COLORS {
            let entry = node - NUM_COLORS;
            self.colors[entry] = if for_dither && entry == 1 {
                // Dithering needs a colour as dark as anything in the image...
                fold_channels(px, 255, u8::min)
            } else if for_dither && entry == NUM_COLORS - 1 {
                // ...and one as light, or error builds up.
                fold_channels(px, 0, u8::max)
            } else {
                mean_color(px)
            };
            return;
        }

        let lo = fold_channels(px, 255, u8::min);
        let hi = fold_channels(px, 0, u8::max);
        let range = |c: usize| i32::from(hi[c]) - i32::from(lo[c]);
        // Axis with the largest range; ties go to green, then blue.
        let mut com = 1;
        if range(2) > range(1) {
            com = 2;
        }
        if range(0) > range(2) && range(0) > range(1) {
            com = 0;
        }
        let (range_min, range_max) = (i32::from(lo[com]), i32::from(hi[com]));

        let n = px.len();
        let mut sub_a = n / 2;
        partition_by_median(px, 0, n, com, sub_a);
        let mut split_value = i32::from(px[sub_a][com]);

        // A very unbalanced split goes at the mean instead, to keep rare colours.
        let unbalance = ((split_value - range_min) - (range_max - split_value)).abs();
        if unbalance > (1536 >> level) {
            split_value = range_min + (range_max - range_min) / 2;
            sub_a = partition_by_mean(px, com, split_value);
        }

        // The bottom node for the transparency index.
        if node == NUM_COLORS / 2 {
            sub_a = 0;
            split_value = 0;
        }

        self.split_elt[node] = com as u8;
        self.split[node] = split_value as u8;

        let (a, b) = px.split_at_mut(sub_a);
        self.split_node(a, node * 2, level + 1, for_dither);
        self.split_node(b, node * 2 + 1, level + 1, for_dither);
    }
}

/// Per-channel fold (min or max) over all pixels.
fn fold_channels(px: &[[u8; 3]], init: u8, f: fn(u8, u8) -> u8) -> [u8; 3] {
    px.iter().fold([init; 3], |acc, p| {
        [f(acc[0], p[0]), f(acc[1], p[1]), f(acc[2], p[2])]
    })
}

/// Rounded per-channel mean. `u64` sums cannot overflow for any frame size.
fn mean_color(px: &[[u8; 3]]) -> [u8; 3] {
    let n = px.len() as u64;
    let sums = px.iter().fold([0u64; 3], |acc, p| {
        [
            acc[0] + u64::from(p[0]),
            acc[1] + u64::from(p[1]),
            acc[2] + u64::from(p[2]),
        ]
    });
    sums.map(|s| ((s + n / 2) / n) as u8)
}

/// gif-h `GifPartition`: the quicksort partition step over `left..right`.
/// Values equal to the pivot alternate sides so uniform runs split evenly.
fn partition(px: &mut [[u8; 3]], left: usize, right: usize, com: usize, pivot: i32) -> usize {
    let mut store = left;
    let mut split = false;
    for ii in left..right {
        let v = i32::from(px[ii][com]);
        if v < pivot {
            px.swap(ii, store);
            store += 1;
        } else if v == pivot {
            if split {
                px.swap(ii, store);
                store += 1;
            }
            split = !split;
        }
    }
    store
}

/// gif-h `GifPartitionByMedian` (quickselect), written as a loop so a large
/// frame cannot overflow a worker thread's stack.
fn partition_by_median(
    px: &mut [[u8; 3]],
    mut left: usize,
    mut right: usize,
    com: usize,
    center: usize,
) {
    while left + 1 < right {
        let pivot = i32::from(px[center][com]);
        px.swap(center, right - 1);
        let pivot_index = partition(px, left, right - 1, com, pivot);
        px.swap(pivot_index, right - 1);
        if pivot_index > center {
            right = pivot_index;
        } else if pivot_index < center {
            left = pivot_index + 1;
        } else {
            break;
        }
    }
}

/// gif-h `GifPartitionByMean`. Like gif-h it leaves the last pixel out.
fn partition_by_mean(px: &mut [[u8; 3]], com: usize, mean: i32) -> usize {
    let n = px.len();
    if n > 1 {
        partition(px, 0, n - 1, com, mean)
    } else {
        0
    }
}
