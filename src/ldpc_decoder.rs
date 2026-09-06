use crate::bitarray::BitArray;
use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;

/// Unified LDPC hard‑decision decoder supporting:
/// - WBF  (Weighted Bit‑Flip)
/// - MWBF (Modified Weighted Bit‑Flip)
/// - NWBF (Normalized Weighted Bit‑Flip)
/// - Gallager‑A
/// - Gallager‑B
///
/// Codewords `cw` are expected to be packed bit arrays of length `N / 8` bytes.
pub struct LdpcDecoder<const M: usize, const N: usize> {
    pub max_iter: usize,
    pub gallager_b_threshold: u8,
    row_to_cols: Box<[Vec<usize>; M]>,
    col_to_rows: Box<[Vec<usize>; N]>,
    check_weights_fixed: Box<[u32; M]>,
    col_degrees: Box<[u32; N]>,
}

impl<const M: usize, const N: usize> LdpcDecoder<M, N> {
    pub fn new(h: &[[u8; N]; M]) -> Self {
        let mut row_to_cols_vec = vec![Vec::new(); M];
        let mut col_to_rows_vec = vec![Vec::new(); N];
        let mut check_weights_fixed = vec![0u32; M];
        let mut col_degrees = vec![0u32; N];

        for (i, row) in row_to_cols_vec.iter_mut().enumerate().take(M) {
            let mut deg = 0;
            for j in 0..N {
                if h[i][j] == 1 {
                    row.push(j);
                    col_to_rows_vec[j].push(i);
                    deg += 1;
                }
            }
            let safe_deg = deg.max(1);
            check_weights_fixed[i] = 65536 / safe_deg;
        }

        for (j, col) in col_to_rows_vec.iter().enumerate().take(N) {
            col_degrees[j] = col.len() as u32;
        }

        let row_to_cols_array: [Vec<usize>; M] = row_to_cols_vec
            .try_into()
            .unwrap_or_else(|_| panic!("Failed to convert row_to_cols vector to array"));
        let col_to_rows_array: [Vec<usize>; N] = col_to_rows_vec
            .try_into()
            .unwrap_or_else(|_| panic!("Failed to convert col_to_rows vector to array"));
        let check_weights_fixed_array: [u32; M] = check_weights_fixed
            .try_into()
            .unwrap_or_else(|_| panic!("Failed to convert check_weights_fixed vector to array"));
        let col_degrees_array: [u32; N] = col_degrees
            .try_into()
            .unwrap_or_else(|_| panic!("Failed to convert col_degrees vector to array"));

        Self {
            max_iter: 50,
            gallager_b_threshold: 2,
            row_to_cols: Box::new(row_to_cols_array),
            col_to_rows: Box::new(col_to_rows_array),
            check_weights_fixed: Box::new(check_weights_fixed_array),
            col_degrees: Box::new(col_degrees_array),
        }
    }

    pub fn set_gallager_b_threshold(&mut self, threshold: u8) {
        debug_assert!(
            threshold > 1,
            "gallager_b_threshold must be > 1 or the decoder will not converge"
        );
        self.gallager_b_threshold = threshold;
    }

    pub fn set_max_iter(&mut self, it: usize) {
        self.max_iter = it;
    }

    pub fn iterate_bitflip(&self, cw: &mut [u8]) -> bool {
        self.iterate_wbf(cw)
    }

    /// Compute syndrome using chunk-optimized dot products where possible.
    pub fn get_parity(&self, cw: &[u8], sn: &mut [u8; M]) -> bool {
        debug_assert_eq!(cw.len() * 8, N);
        let mut valid = true;

        for (i, row) in self.row_to_cols.iter().enumerate() {
            let mut sum = 0u8;
            for &j in row {
                sum ^= BitArray::get_bit(cw, j);
            }
            sn[i] = sum;
            if sum != 0 {
                valid = false;
            }
        }

        valid
    }

    pub fn get_score(&self, sn: &[u8; M], en: &mut [u8; N]) {
        for (j, col) in self.col_to_rows.iter().enumerate() {
            let mut score = 0u8;
            for &i in col {
                if sn[i] == 1 {
                    score += 1;
                }
            }
            en[j] = score;
        }
    }

    pub fn iterate_gallager_a(&self, cw: &mut [u8]) -> bool {
        debug_assert_eq!(cw.len() * 8, N);
        let mut sn = vec![0u8; M].try_into().unwrap();
        if self.get_parity(cw, &mut sn) {
            return true;
        }

        let mut flip = vec![false; N];

        for (j, col) in self.col_to_rows.iter().enumerate() {
            let mut votes = 0u8;
            let total = self.col_degrees[j] as u8;

            for &i in col {
                if sn[i] == 1 {
                    votes += 1;
                }
            }

            if votes > total / 2 {
                flip[j] = true;
            }
        }

        for (j, &should_flip) in flip.iter().enumerate() {
            if should_flip {
                BitArray::xor_bit(cw, j);
            }
        }

        false
    }

    pub fn iterate_gallager_b(&self, cw: &mut [u8]) -> bool {
        debug_assert_eq!(cw.len() * 8, N);
        let mut sn = vec![0u8; M].try_into().unwrap();
        if self.get_parity(cw, &mut sn) {
            return true;
        }

        let mut flip = vec![false; N];

        for (j, col) in self.col_to_rows.iter().enumerate() {
            let mut votes = 0u8;
            let total = self.col_degrees[j] as u8;

            for &i in col {
                if sn[i] == 1 {
                    votes += 1;
                }
            }

            if total > 0 && votes >= self.gallager_b_threshold {
                flip[j] = true;
            }
        }

        for (j, &should_flip) in flip.iter().enumerate() {
            if should_flip {
                BitArray::xor_bit(cw, j);
            }
        }

        false
    }

    pub fn iterate_wbf(&self, cw: &mut [u8]) -> bool {
        debug_assert_eq!(cw.len() * 8, N);
        let mut sn = vec![0u8; M].try_into().unwrap();
        if self.get_parity(cw, &mut sn) {
            return true;
        }

        let mut scores = vec![0u32; N];
        for (j, col) in self.col_to_rows.iter().enumerate() {
            let mut s = 0u32;
            for &i in col {
                if sn[i] == 1 {
                    s += self.check_weights_fixed[i];
                }
            }
            scores[j] = s;
        }

        let mut max_score = 0u32;
        let mut best_j = 0usize;

        for (j, &score) in scores.iter().enumerate() {
            if score > max_score {
                max_score = score;
                best_j = j;
            }
        }

        if max_score > 0 {
            BitArray::xor_bit(cw, best_j);
        }

        false
    }

    pub fn iterate_mwbf(&self, cw: &mut [u8]) -> bool {
        debug_assert_eq!(cw.len() * 8, N);
        let mut sn = vec![0u8; M].try_into().unwrap();
        if self.get_parity(cw, &mut sn) {
            return true;
        }

        let mut scores = vec![0u32; N];
        for (j, col) in self.col_to_rows.iter().enumerate() {
            let mut s = 0u32;
            for &i in col {
                if sn[i] == 1 {
                    s += 1;
                }
            }
            scores[j] = s;
        }

        let mut max_score = 0u32;
        for &score in scores.iter() {
            if score > max_score {
                max_score = score;
            }
        }

        if max_score > 0 {
            for (j, &score) in scores.iter().enumerate() {
                if score == max_score {
                    BitArray::xor_bit(cw, j);
                }
            }
        }

        false
    }

    pub fn iterate_nwbf(&self, cw: &mut [u8]) -> bool {
        debug_assert_eq!(cw.len() * 8, N);
        let mut sn = vec![0u8; M].try_into().unwrap();
        if self.get_parity(cw, &mut sn) {
            return true;
        }

        let mut scores = vec![0u32; N];
        for (j, col) in self.col_to_rows.iter().enumerate() {
            let mut votes = 0u32;
            for &i in col {
                if sn[i] == 1 {
                    votes += 1;
                }
            }
            let deg = self.col_degrees[j].max(1);
            scores[j] = (votes << 16) / deg;
        }

        let mut max_score = 0u32;
        for &score in scores.iter() {
            if score > max_score {
                max_score = score;
            }
        }

        if max_score > 0 {
            for (j, &score) in scores.iter().enumerate() {
                if score == max_score {
                    BitArray::xor_bit(cw, j);
                }
            }
        }

        false
    }
}
