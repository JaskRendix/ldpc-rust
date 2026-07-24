use crate::matrices::h_256_512::H_256_512;
use std::sync::LazyLock;

/// Global, lazily-initialized CCSDS LDPC encoder.
pub static LDPC_ENCODER: LazyLock<LdpcEncoder> = LazyLock::new(LdpcEncoder::new);

pub struct LdpcEncoder {
    /// Parity generator matrix P (256 × 256) such that codeword is [message | parity]
    parity_generator: [[u8; 256]; 256],
}

impl Default for LdpcEncoder {
    fn default() -> Self {
        Self::new()
    }
}

impl LdpcEncoder {
    pub fn new() -> Self {
        let parity_generator = Self::compute_generator_matrix(&H_256_512);
        Self { parity_generator }
    }

    /// Encode a 256-bit message into a 512-bit systematic codeword [u | p].
    pub fn encode(&self, message: &[u8; 256]) -> [u8; 512] {
        let mut cw = [0u8; 512];

        // Systematic part: u
        cw[..256].copy_from_slice(message);

        // Parity part: p_i = sum_j (message[j] * parity_generator[j][i])
        for i in 0..256 {
            let mut sum = 0u8;
            for j in 0..256 {
                sum ^= message[j] & self.parity_generator[j][i];
            }
            cw[256 + i] = sum;
        }

        cw
    }

    /// Compute generator sub-matrix P from H = [A | B] where H is 128 rows (or 256 rows depending on rank).
    /// For CCSDS 256x512, H has 256 rows and 512 columns.
    fn compute_generator_matrix(h: &[[u8; 512]; 256]) -> [[u8; 256]; 256] {
        // Rearrange or split H into H = [A | B] where B is 256x256
        let mut a = [[0u8; 256]; 256];
        let mut b = [[0u8; 256]; 256];

        for (i, row) in h.iter().enumerate() {
            // Standard CCSDS systematic form often places parity on the right or requires Gaussian elimination
            // to find an invertible 256x256 submatrix B. 
            // Assuming columns 256..512 form B:
            for j in 0..256 {
                a[i][j] = row[j];
                b[i][j] = row[256 + j];
            }
        }

        let b_inv = Self::invert_gf2(&b);

        // P = B^{-1} * A over GF(2)
        let mut p = [[0u8; 256]; 256];
        for i in 0..256 {
            for j in 0..256 {
                let mut sum = 0u8;
                for k in 0..256 {
                    sum ^= b_inv[i][k] & a[k][j];
                }
                p[i][j] = sum;
            }
        }

        p
    }

    /// Invert a 256×256 matrix over GF(2) using Gauss-Jordan elimination.
    fn invert_gf2(mat: &[[u8; 256]; 256]) -> [[u8; 256]; 256] {
        let mut a = *mat;
        let mut inv = [[0u8; 256]; 256];

        for i in 0..256 {
            inv[i][i] = 1;
        }

        for col in 0..256 {
            let mut pivot = col;
            while pivot < 256 && a[pivot][col] == 0 {
                pivot += 1;
            }
            
            if pivot >= 256 {
                // If singular, try finding a pivot in subsequent columns or handle gracefully.
                // For standard CCSDS H_256_512, B should be full rank or require column swaps.
                panic!("Matrix B is singular or non-invertible at column {col}");
            }

            if pivot != col {
                a.swap(col, pivot);
                inv.swap(col, pivot);
            }

            for row in 0..256 {
                if row != col && a[row][col] == 1 {
                    for k in 0..256 {
                        a[row][k] ^= a[col][k];
                        inv[row][k] ^= inv[col][k];
                    }
                }
            }
        }

        inv
    }
}
