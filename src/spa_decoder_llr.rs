use std::f64;

pub struct SpaDecoderLLR {
    pub h: Vec<Vec<u8>>,
    pub m: usize,
    pub n: usize,
    pub max_iter: usize,

    pub rmn: Vec<Vec<f64>>,
    pub qnm: Vec<Vec<f64>>,
}

impl SpaDecoderLLR {
    pub fn new(h: Vec<Vec<u8>>) -> Self {
        assert!(
            !h.is_empty(),
            "parity-check matrix h must have at least one row"
        );
        let m = h.len();
        let n = h[0].len();
        debug_assert!(
            h.iter().all(|row| row.len() == n),
            "all rows of h must have the same length"
        );

        SpaDecoderLLR {
            h,
            m,
            n,
            max_iter: 50,
            rmn: vec![vec![0.0; n]; m],
            qnm: vec![vec![0.0; n]; m],
        }
    }

    pub fn set_max_iter(&mut self, iters: usize) {
        self.max_iter = iters;
    }

    pub fn decode(&mut self, llr: &[f64]) -> Vec<u8> {
        // initialize qnm with channel LLRs
        for (i, row) in self.h.iter().enumerate().take(self.m) {
            for (j, &h_ij) in row.iter().enumerate().take(self.n) {
                if h_ij == 1 {
                    self.qnm[i][j] = llr[j];
                }
            }
        }

        let mut hard = vec![0u8; self.n];

        // Reused across rows/iterations to avoid reallocating on every
        // check-node update (m * max_iter allocations otherwise).
        let mut signs: Vec<f64> = Vec::with_capacity(self.n);
        let mut mags: Vec<f64> = Vec::with_capacity(self.n);

        for _ in 0..self.max_iter {
            // check node update
            for (i, row) in self.h.iter().enumerate().take(self.m) {
                signs.clear();
                mags.clear();

                for (j, &h_ij) in row.iter().enumerate().take(self.n) {
                    if h_ij == 1 {
                        let v = self.qnm[i][j];
                        signs.push(v.signum());
                        mags.push(v.abs());
                    }
                }

                let global_sign: f64 = signs.iter().product();

                // Single-pass computation of the smallest magnitude
                // (min1), the position it occurs at (idx_min1), and the
                // second-smallest magnitude (min2). The outgoing message
                // to the variable at idx_min1 must use min2 - not min1 -
                // to stay extrinsic (exclude that variable's own
                // contribution); every other variable uses min1.
                let mut min1 = f64::INFINITY;
                let mut min2 = f64::INFINITY;
                let mut idx_min1 = 0usize;

                for (idx, &v) in mags.iter().enumerate() {
                    if v < min1 {
                        min2 = min1;
                        min1 = v;
                        idx_min1 = idx;
                    } else if v < min2 {
                        min2 = v;
                    }
                }

                let mut k = 0usize;
                for (j, &h_ij) in row.iter().enumerate().take(self.n) {
                    if h_ij == 1 {
                        let sign_j = signs[k];
                        let out_mag = if k == idx_min1 { min2 } else { min1 };
                        let out_sign = global_sign * sign_j;
                        self.rmn[i][j] = out_sign * out_mag;
                        k += 1;
                    }
                }
            }

            // variable node update
            for (j, hard_j) in hard.iter_mut().enumerate().take(self.n) {
                let mut sum = llr[j];

                for (i, row) in self.h.iter().enumerate().take(self.m) {
                    if row[j] == 1 {
                        sum += self.rmn[i][j];
                    }
                }

                *hard_j = if sum >= 0.0 { 0 } else { 1 };

                for (i, row) in self.h.iter().enumerate().take(self.m) {
                    if row[j] == 1 {
                        self.qnm[i][j] = sum - self.rmn[i][j];
                    }
                }
            }

            if self.check_syndrome(&hard) {
                break;
            }
        }

        hard
    }

    fn check_syndrome(&self, cw: &[u8]) -> bool {
        for (_i, row) in self.h.iter().enumerate().take(self.m) {
            let mut sum = 0u8;
            for (j, &h_ij) in row.iter().enumerate().take(self.n) {
                if h_ij == 1 {
                    sum ^= cw[j];
                }
            }
            if sum != 0 {
                return false;
            }
        }
        true
    }
}
to
pub struct SpaDecoderLLR {
    pub m: usize,
    pub n: usize,
    pub max_iter: usize,
    // Sparse representation caches to avoid dense pointer chasing
    row_to_cols: Box<[Vec<usize>; 256]>,
    col_to_rows: Box<[Vec<usize>; 512]>,
    // Internal message matrices preserved to avoid per-iteration allocations
    rmn: Vec<Vec<f64>>,
    qnm: Vec<Vec<f64>>,
}

impl SpaDecoderLLR {
    pub fn new(h: &[[u8; 512]; 256]) -> Self {
        let m = 256;
        let n = 512;

        let mut row_to_cols_vec = vec![Vec::new(); m];
        let mut col_to_rows_vec = vec![Vec::new(); n];

        for i in 0..m {
            for j in 0..n {
                if h[i][j] == 1 {
                    row_to_cols_vec[i].push(j);
                    col_to_rows_vec[j].push(i);
                }
            }
        }

        let row_to_cols = row_to_cols_vec.try_into().unwrap();
        let col_to_rows = col_to_rows_vec.try_into().unwrap();

        SpaDecoderLLR {
            m,
            n,
            max_iter: 50,
            row_to_cols,
            col_to_rows,
            rmn: vec![vec![0.0; n]; m],
            qnm: vec![vec![0.0; n]; m],
        }
    }

    pub fn set_max_iter(&mut self, iters: usize) {
        self.max_iter = iters;
    }

    pub fn decode(&mut self, llr: &[f64]) -> Vec<u8> {
        // Initialize qnm with channel LLRs sparsely
        for i in 0..self.m {
            for &j in &self.row_to_cols[i] {
                self.qnm[i][j] = llr[j];
            }
        }

        let mut hard = vec![0u8; self.n];
        let mut signs: Vec<f64> = Vec::with_capacity(8);
        let mut mags: Vec<f64> = Vec::with_capacity(8);

        for _ in 0..self.max_iter {
            // Check-node update
            for i in 0..self.m {
                signs.clear();
                mags.clear();

                for &j in &self.row_to_cols[i] {
                    let v = self.qnm[i][j];
                    signs.push(v.signum());
                    mags.push(v.abs());
                }

                let global_sign: f64 = signs.iter().product();

                let mut min1 = f64::INFINITY;
                let mut min2 = f64::INFINITY;
                let mut idx_min1 = 0usize;

                for (idx, &v) in mags.iter().enumerate() {
                    if v < min1 {
                        min2 = min1;
                        min1 = v;
                        idx_min1 = idx;
                    } else if v < min2 {
                        min2 = v;
                    }
                }

                for (k, &j) in self.row_to_cols[i].iter().enumerate() {
                    let sign_j = signs[k];
                    let out_mag = if k == idx_min1 { min2 } else { min1 };
                    let out_sign = global_sign * sign_j;
                    self.rmn[i][j] = out_sign * out_mag;
                }
            }

            // Variable-node update
            for j in 0..self.n {
                let mut sum = llr[j];
                for &i in &self.col_to_rows[j] {
                    sum += self.rmn[i][j];
                }

                hard[j] = if sum >= 0.0 { 0 } else { 1 };

                for &i in &self.col_to_rows[j] {
                    self.qnm[i][j] = sum - self.rmn[i][j];
                }
            }

            if self.check_syndrome(&hard) {
                break;
            }
        }

        hard
    }

    fn check_syndrome(&self, cw: &[u8]) -> bool {
        for i in 0..self.m {
            let mut sum = 0u8;
            for &j in &self.row_to_cols[i] {
                sum ^= cw[j];
            }
            if sum != 0 {
                return false;
            }
        }
        true
    }
}
