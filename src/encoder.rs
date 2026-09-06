use crate::matrices::h_256_512::H_256_512;

#[cfg(feature = "std")]
use std::sync::LazyLock as Lazy;

#[cfg(not(feature = "std"))]
use spin::LazyLock as Lazy;

/// Global, lazily-initialized CCSDS LDPC encoder for the (512, 256) code.
pub static LDPC_ENCODER: Lazy<LdpcEncoder<256, 512>> = Lazy::new(|| LdpcEncoder::new(&H_256_512));

pub struct LdpcEncoder<const M: usize, const N: usize> {
    /// Parity generator matrix P (M × M)
    parity_generator: [[u8; M]; M],
}

impl<const M: usize, const N: usize> LdpcEncoder<M, N> {
    pub fn new(h: &[[u8; N]; M]) -> Self {
        let parity_generator = Self::compute_generator_matrix(h);
        Self { parity_generator }
    }

    /// Encode an M-bit message into an N-bit systematic codeword [u | p].
    pub fn encode(&self, message: &[u8; M]) -> [u8; N] {
        let mut cw = [0u8; N];

        // Systematic part
        cw[..M].copy_from_slice(message);

        // Parity part: p_i = XOR_j (message[j] & P[i][j])
        for (i, parity_bit) in cw[M..].iter_mut().enumerate() {
            let mut sum = 0u8;
            for (j, &msg_bit) in message.iter().enumerate() {
                sum ^= msg_bit & self.parity_generator[i][j];
            }
            *parity_bit = sum;
        }

        cw
    }

    /// Compute generator matrix P = B^{-1} A over GF(2).
    fn compute_generator_matrix(h: &[[u8; N]; M]) -> [[u8; M]; M] {
        let mut a = [[0u8; M]; M];
        let mut b = [[0u8; M]; M];

        // Split H = [A | B]
        for (i, row) in h.iter().enumerate() {
            a[i].copy_from_slice(&row[..M]);
            b[i].copy_from_slice(&row[M..N]);
        }

        let b_inv = Self::invert_gf2(&b);

        // Compute P = B^{-1} A
        let mut p = [[0u8; M]; M];
        for (i, p_row) in p.iter_mut().enumerate() {
            for (j, p_val) in p_row.iter_mut().enumerate() {
                let mut sum = 0u8;
                for k in 0..M {
                    sum ^= b_inv[i][k] & a[k][j];
                }
                *p_val = sum;
            }
        }

        p
    }

    /// Invert an M×M matrix over GF(2) using Gauss-Jordan elimination.
    fn invert_gf2(mat: &[[u8; M]; M]) -> [[u8; M]; M] {
        let mut a = *mat;
        let mut inv = [[0u8; M]; M];

        // Identity matrix
        for (i, row) in inv.iter_mut().enumerate() {
            row[i] = 1;
        }

        for col in 0..M {
            // Find pivot
            let mut pivot = col;
            while pivot < M && a[pivot][col] == 0 {
                pivot += 1;
            }

            if pivot >= M {
                #[cfg(feature = "std")]
                {
                    std::panic!("Matrix B is singular at column {col}");
                }
                #[cfg(not(feature = "std"))]
                {
                    core::panic!("Matrix B is singular");
                }
            }

            // Swap rows
            if pivot != col {
                a.swap(col, pivot);
                inv.swap(col, pivot);
            }

            // Eliminate other rows
            for row in 0..M {
                if row != col && a[row][col] == 1 {
                    for k in 0..M {
                        a[row][k] ^= a[col][k];
                        inv[row][k] ^= inv[col][k];
                    }
                }
            }
        }

        inv
    }
}
