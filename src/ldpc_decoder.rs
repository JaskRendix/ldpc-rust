use crate::matrices::h_256_512::H_256_512;

/// Unified LDPC hard‑decision decoder supporting:
/// - WBF  (Weighted Bit‑Flip)
/// - MWBF (Modified Weighted Bit‑Flip)
/// - NWBF (Normalized Weighted Bit‑Flip)
/// - Gallager‑A
/// - Gallager‑B
pub struct LdpcDecoder {
    pub max_iter: usize,
    pub gallager_b_threshold: u8,
}

impl LdpcDecoder {
    pub fn new(_h: &[[u8; 512]; 256]) -> Self {
        Self {
            max_iter: 50,
            // Column (bit) degree in the CCSDS 256x512 matrix is small
            // (commonly 3-4). Requiring at least 2 disagreeing checks is
            // permissive enough to still correct errors, but no longer
            // degenerates on a single unsatisfied check touching a bit.
            gallager_b_threshold: 2,
        }
    }

    pub fn set_gallager_b_threshold(&mut self, threshold: u8) {
        debug_assert!(
            threshold > 1,
            "gallager_b_threshold must be > 1 or the decoder will not converge"
        );
        self.gallager_b_threshold = threshold;
    }

    /// Backwards‑compatible alias: simple bit‑flip = WBF
    pub fn iterate_bitflip(&self, cw: &mut [u8; 512]) -> bool {
        self.iterate_wbf(cw)
    }

    pub fn set_max_iter(&mut self, it: usize) {
        self.max_iter = it;
    }

    /// Compute syndrome: sn[i] = parity of row i
    pub fn get_parity(&self, cw: &[u8; 512], sn: &mut [u8; 256]) -> bool {
        let mut valid = true;

        for i in 0..256 {
            let mut sum = 0u8;
            for j in 0..512 {
                if H_256_512[i][j] == 1 {
                    sum ^= cw[j];
                }
            }
            sn[i] = sum;
            if sum != 0 {
                valid = false;
            }
        }

        valid
    }

    /// Score[j] = number of unsatisfied checks involving bit j
    pub fn get_score(&self, sn: &[u8; 256], en: &mut [u8; 512]) {
        for j in 0..512 {
            let mut score = 0u8;
            for i in 0..256 {
                if H_256_512[i][j] == 1 && sn[i] == 1 {
                    score += 1;
                }
            }
            en[j] = score;
        }
    }

    // -------------------------------------------------------------------------
    // GALLAGER‑A
    // -------------------------------------------------------------------------
    pub fn iterate_gallager_a(&self, cw: &mut [u8; 512]) -> bool {
        let mut sn = [0u8; 256];
        if self.get_parity(cw, &mut sn) {
            return true;
        }

        let mut flip = [false; 512];

        for j in 0..512 {
            let mut votes = 0u8;
            let mut total = 0u8;

            for i in 0..256 {
                if H_256_512[i][j] == 1 {
                    total += 1;
                    if sn[i] == 1 {
                        votes += 1;
                    }
                }
            }

            if votes > total / 2 {
                flip[j] = true;
            }
        }

        for j in 0..512 {
            if flip[j] {
                cw[j] ^= 1;
            }
        }

        false
    }

    // -------------------------------------------------------------------------
    // GALLAGER‑B
    // -------------------------------------------------------------------------
    pub fn iterate_gallager_b(&self, cw: &mut [u8; 512]) -> bool {
        let mut sn = [0u8; 256];
        if self.get_parity(cw, &mut sn) {
            return true;
        }

        let mut flip = [false; 512];

        for j in 0..512 {
            let mut votes = 0u8;
            let mut total = 0u8;

            for i in 0..256 {
                if H_256_512[i][j] == 1 {
                    total += 1;
                    if sn[i] == 1 {
                        votes += 1;
                    }
                }
            }

            // Gallager‑B: flip only if at least `gallager_b_threshold`
            // incident checks are unsatisfied. Unlike Gallager‑A this
            // threshold is fixed ahead of time rather than recomputed as
            // a majority of the bit's current degree.
            if total > 0 && votes >= self.gallager_b_threshold {
                flip[j] = true;
            }
        }

        for j in 0..512 {
            if flip[j] {
                cw[j] ^= 1;
            }
        }

        false
    }

    // -------------------------------------------------------------------------
    // WBF (Weighted Bit‑Flip)
    // -------------------------------------------------------------------------
    pub fn iterate_wbf(&self, cw: &mut [u8; 512]) -> bool {
        let mut sn = [0u8; 256];
        if self.get_parity(cw, &mut sn) {
            return true;
        }

        // Compute check weights: w_i = 1 / degree(i)
        let mut w = [0f64; 256];
        for i in 0..256 {
            let deg = H_256_512[i].iter().filter(|&&b| b == 1).count() as f64;
            w[i] = 1.0 / deg.max(1.0);
        }

        // Compute weighted scores
        let mut score = [0f64; 512];
        for j in 0..512 {
            let mut s = 0.0;
            for i in 0..256 {
                if H_256_512[i][j] == 1 && sn[i] == 1 {
                    s += w[i];
                }
            }
            score[j] = s;
        }

        // Flip only the single bit with maximum score
        let (best_j, _) = score
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .unwrap();

        cw[best_j] ^= 1;

        false
    }

    // -------------------------------------------------------------------------
    // MWBF (Modified Weighted Bit‑Flip)
    // -------------------------------------------------------------------------
    pub fn iterate_mwbf(&self, cw: &mut [u8; 512]) -> bool {
        let mut sn = [0u8; 256];
        if self.get_parity(cw, &mut sn) {
            return true;
        }

        let mut score = [0f64; 512];

        for j in 0..512 {
            let mut s = 0f64;
            for i in 0..256 {
                if H_256_512[i][j] == 1 && sn[i] == 1 {
                    s += 1.0;
                }
            }
            score[j] = s;
        }

        let max_score = score.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        for j in 0..512 {
            if score[j] == max_score {
                cw[j] ^= 1;
            }
        }

        false
    }

    // -------------------------------------------------------------------------
    // NWBF (Normalized Weighted Bit‑Flip)
    // -------------------------------------------------------------------------
    pub fn iterate_nwbf(&self, cw: &mut [u8; 512]) -> bool {
        let mut sn = [0u8; 256];
        if self.get_parity(cw, &mut sn) {
            return true;
        }

        let mut score = [0f64; 512];

        for j in 0..512 {
            let mut s = 0f64;
            let mut deg = 0f64;

            for i in 0..256 {
                if H_256_512[i][j] == 1 {
                    deg += 1.0;
                    if sn[i] == 1 {
                        s += 1.0;
                    }
                }
            }

            score[j] = s / deg.max(1.0);
        }

        let max_score = score.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        for j in 0..512 {
            if score[j] == max_score {
                cw[j] ^= 1;
            }
        }

        false
    }
}
