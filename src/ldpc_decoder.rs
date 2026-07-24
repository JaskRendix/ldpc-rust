use crate::bitarray::BitArray;
use crate::matrices::h_256_512::H_256_512;

/// Unified LDPC hard‑decision decoder supporting:
/// - WBF  (Weighted Bit‑Flip)
/// - MWBF (Modified Weighted Bit‑Flip)
/// - NWBF (Normalized Weighted Bit‑Flip)
/// - Gallager‑A
/// - Gallager‑B
///
/// Codewords `cw` are expected to be packed bit arrays of 64 bytes (512 bits).
pub struct LdpcDecoder {
    pub max_iter: usize,
    pub gallager_b_threshold: u8,
    row_to_cols: Box<[Vec<usize>; 256]>,
    col_to_rows: Box<[Vec<usize>; 512]>,
    check_weights_fixed: Box<[u32; 256]>,
    col_degrees: Box<[u32; 512]>,
}

impl LdpcDecoder {
    pub fn new(_h: &[[u8; 512]; 256]) -> Self {
        let mut row_to_cols_vec = vec![Vec::new(); 256];
        let mut col_to_rows_vec = vec![Vec::new(); 512];
        let mut check_weights_fixed = [0u32; 256];
        let mut col_degrees = [0u32; 512];

        for (i, row) in row_to_cols_vec.iter_mut().enumerate().take(256) {
            let mut deg = 0;
            for j in 0..512 {
                if H_256_512[i][j] == 1 {
                    row.push(j);
                    col_to_rows_vec[j].push(i);
                    deg += 1;
                }
            }
            let safe_deg = deg.max(1);
            check_weights_fixed[i] = 65536 / safe_deg;
        }

        for (j, col) in col_to_rows_vec.iter().enumerate().take(512) {
            col_degrees[j] = col.len() as u32;
        }

        let row_to_cols_array: [Vec<usize>; 256] = row_to_cols_vec
            .try_into()
            .unwrap_or_else(|_| panic!("Failed to convert row_to_cols vector to array"));
        let col_to_rows_array: [Vec<usize>; 512] = col_to_rows_vec
            .try_into()
            .unwrap_or_else(|_| panic!("Failed to convert col_to_rows vector to array"));

        Self {
            max_iter: 50,
            gallager_b_threshold: 2,
            row_to_cols: Box::new(row_to_cols_array),
            col_to_rows: Box::new(col_to_rows_array),
            check_weights_fixed: Box::new(check_weights_fixed),
            col_degrees: Box::new(col_degrees),
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

    pub fn iterate_bitflip(&self, cw: &mut [u8; 64]) -> bool {
        self.iterate_wbf(cw)
    }

    /// Compute syndrome using chunk-optimized dot products where possible.
    pub fn get_parity(&self, cw: &[u8; 64], sn: &mut [u8; 256]) -> bool {
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

    pub fn get_score(&self, sn: &[u8; 256], en: &mut [u8; 512]) {
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

    pub fn iterate_gallager_a(&self, cw: &mut [u8; 64]) -> bool {
        let mut sn = [0u8; 256];
        if self.get_parity(cw, &mut sn) {
            return true;
        }

        let mut flip = [false; 512];

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

    pub fn iterate_gallager_b(&self, cw: &mut [u8; 64]) -> bool {
        let mut sn = [0u8; 256];
        if self.get_parity(cw, &mut sn) {
            return true;
        }

        let mut flip = [false; 512];

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

    pub fn iterate_wbf(&self, cw: &mut [u8; 64]) -> bool {
        let mut sn = [0u8; 256];
        if self.get_parity(cw, &mut sn) {
            return true;
        }

        let mut scores = [0u32; 512];
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

    pub fn iterate_mwbf(&self, cw: &mut [u8; 64]) -> bool {
        let mut sn = [0u8; 256];
        if self.get_parity(cw, &mut sn) {
            return true;
        }

        let mut scores = [0u32; 512];
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

    pub fn iterate_nwbf(&self, cw: &mut [u8; 64]) -> bool {
        let mut sn = [0u8; 256];
        if self.get_parity(cw, &mut sn) {
            return true;
        }

        let mut scores = [0u32; 512];
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
