use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecoderError {
    InvalidInputLength,
}

#[derive(Debug, Clone)]
pub struct DecodeResult {
    pub codeword: Vec<u8>,
    pub iterations: usize,
    pub converged: bool,
}

pub struct SpaDecoderLLR<const M: usize, const N: usize> {
    pub max_iter: usize,
    pub scaling_factor: f64,
    row_to_cols: Box<[Vec<usize>; M]>,
    col_to_rows: Box<[Vec<usize>; N]>,
    col_to_row_edge_idxs: Box<[Vec<usize>; N]>,
    rmn: Box<[Vec<f64>; M]>,
    qnm: Box<[Vec<f64>; M]>,
    row_signs: Box<[Vec<i8>; M]>,
    row_mags: Box<[Vec<f64>; M]>,
}

impl<const M: usize, const N: usize> SpaDecoderLLR<M, N> {
    pub fn row_to_cols(&self) -> &[Vec<usize>] {
        &self.row_to_cols[..]
    }

    pub fn col_to_rows(&self) -> &[Vec<usize>] {
        &self.col_to_rows[..]
    }

    pub fn col_to_row_edge_idxs(&self) -> &[Vec<usize>] {
        &self.col_to_row_edge_idxs[..]
    }

    pub fn rmn(&self) -> &[Vec<f64>] {
        &self.rmn[..]
    }

    pub fn qnm(&self) -> &[Vec<f64>] {
        &self.qnm[..]
    }

    pub fn row_signs(&self) -> &[Vec<i8>] {
        &self.row_signs[..]
    }

    pub fn row_mags(&self) -> &[Vec<f64>] {
        &self.row_mags[..]
    }

    pub fn check_syndrome_public(&self, cw: &[u8]) -> bool {
        self.check_syndrome(cw)
    }

    pub fn new(h: &[[u8; N]; M]) -> Self {
        let mut row_to_cols_vec = vec![Vec::new(); M];
        let mut col_to_rows_vec = vec![Vec::new(); N];

        for i in 0..M {
            for j in 0..N {
                if h[i][j] == 1 {
                    row_to_cols_vec[i].push(j);
                    col_to_rows_vec[j].push(i);
                }
            }
        }

        let row_to_cols: [Vec<usize>; M] = row_to_cols_vec
            .try_into()
            .unwrap_or_else(|_| panic!("Failed to convert row_to_cols vector to array"));
        let col_to_rows: [Vec<usize>; N] = col_to_rows_vec
            .try_into()
            .unwrap_or_else(|_| panic!("Failed to convert col_to_rows vector to array"));

        let mut col_to_row_edge_idxs_vec = vec![Vec::new(); N];
        for j in 0..N {
            for &i in &col_to_rows[j] {
                let k = row_to_cols[i].iter().position(|&col| col == j).unwrap();
                col_to_row_edge_idxs_vec[j].push(k);
            }
        }
        let col_to_row_edge_idxs: [Vec<usize>; N] = col_to_row_edge_idxs_vec
            .try_into()
            .unwrap_or_else(|_| panic!("Failed to convert col_to_row_edge_idxs vector to array"));

        let mut rmn_vec = Vec::with_capacity(M);
        let mut qnm_vec = Vec::with_capacity(M);
        let mut row_signs_vec = Vec::with_capacity(M);
        let mut row_mags_vec = Vec::with_capacity(M);

        for cols in row_to_cols.iter() {
            let deg = cols.len();
            rmn_vec.push(vec![0.0; deg]);
            qnm_vec.push(vec![0.0; deg]);
            row_signs_vec.push(vec![1i8; deg]);
            row_mags_vec.push(vec![0.0; deg]);
        }

        SpaDecoderLLR {
            max_iter: 50,
            scaling_factor: 0.75,
            row_to_cols: Box::new(row_to_cols),
            col_to_rows: Box::new(col_to_rows),
            col_to_row_edge_idxs: Box::new(col_to_row_edge_idxs),
            rmn: rmn_vec.into_boxed_slice().try_into().unwrap(),
            qnm: qnm_vec.into_boxed_slice().try_into().unwrap(),
            row_signs: row_signs_vec.into_boxed_slice().try_into().unwrap(),
            row_mags: row_mags_vec.into_boxed_slice().try_into().unwrap(),
        }
    }

    pub fn set_max_iter(&mut self, iters: usize) {
        self.max_iter = iters;
    }

    pub fn set_scaling_factor(&mut self, alpha: f64) {
        self.scaling_factor = alpha;
    }

    pub fn decode(&mut self, llr: &[f64]) -> Result<DecodeResult, DecoderError> {
        if llr.len() != N {
            return Err(DecoderError::InvalidInputLength);
        }

        for i in 0..M {
            for (k, &j) in self.row_to_cols[i].iter().enumerate() {
                self.qnm[i][k] = llr[j];
            }
        }

        let mut hard = vec![0u8; N];
        let mut converged = false;
        let mut final_iter = 0;

        for iter in 0..self.max_iter {
            final_iter = iter + 1;
            for i in 0..M {
                let row_len = self.row_to_cols[i].len();
                let signs = &mut self.row_signs[i];
                let mags = &mut self.row_mags[i];
                let q_row = &self.qnm[i];

                for k in 0..row_len {
                    let v = q_row[k];
                    signs[k] = if v < 0.0 { -1 } else { 1 };
                    mags[k] = v.abs();
                }

                let mut global_sign: i8 = 1;
                for &s in signs.iter() {
                    global_sign *= s;
                }

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

                let r_row = &mut self.rmn[i];
                let scale = self.scaling_factor;
                for k in 0..row_len {
                    let sign_j = signs[k];
                    let out_mag = if k == idx_min1 { min2 } else { min1 };
                    let out_sign = (global_sign * sign_j) as f64;
                    r_row[k] = out_sign * out_mag * scale;
                }
            }

            for j in 0..N {
                let mut sum = llr[j];
                let check_nodes = &self.col_to_rows[j];
                let edge_idxs = &self.col_to_row_edge_idxs[j];

                for (idx, &i) in check_nodes.iter().enumerate() {
                    let k = edge_idxs[idx];
                    sum += self.rmn[i][k];
                }

                hard[j] = if sum >= 0.0 { 0 } else { 1 };

                for (idx, &i) in check_nodes.iter().enumerate() {
                    let k = edge_idxs[idx];
                    self.qnm[i][k] = sum - self.rmn[i][k];
                }
            }

            if self.check_syndrome(&hard) {
                converged = true;
                break;
            }
        }

        Ok(DecodeResult {
            codeword: hard,
            iterations: final_iter,
            converged,
        })
    }

    fn check_syndrome(&self, cw: &[u8]) -> bool {
        for i in 0..M {
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
