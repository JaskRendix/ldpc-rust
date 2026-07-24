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
