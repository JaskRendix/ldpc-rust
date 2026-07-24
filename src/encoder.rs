use crate::matrices::h_256_512::H_256_512;
use std::sync::LazyLock;

/// Global, lazily-initialized CCSDS LDPC encoder.
/// Computes B^{-1}A only once, on first use.
pub static LDPC_ENCODER: LazyLock<LdpcEncoder> = LazyLock::new(LdpcEncoder::new);

pub struct LdpcEncoder {
    /// Parity generator matrix P = B^{-1} A (256 × 256)
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

    /// Encode a 256-bit message into a 512-bit systematic codeword.
    pub fn encode(&self, message: &[u8; 256]) -> [u8; 512] {
        let mut cw = [0u8; 512];

        // Systematic part
        cw[..256].copy_from_slice(message);

        // Parity part: p = Pᵀ * u
        for (i, parity_bit) in cw[256..].iter_mut().enumerate() {
            let mut sum = 0u8;
            for (j, &msg_bit) in message.iter().enumerate() {
                sum ^= self.parity_generator[j][i] & msg_bit;
            }
            *parity_bit = sum;
        }

        cw
    }

    /// Compute P = B^{-1} A over GF(2).
    fn compute_generator_matrix(h: &[[u8; 512]; 256]) -> [[u8; 256]; 256] {
        let mut a = [[0u8; 256]; 256];
        let mut b = [[0u8; 256]; 256];

        // Split H = [A | B]
        for (i, row) in h.iter().enumerate() {
            a[i].copy_from_slice(&row[..256]);
            b[i].copy_from_slice(&row[256..]);
        }

        let b_inv = Self::invert_gf2(&b);

        // Compute P = B^{-1} A
        let mut p = [[0u8; 256]; 256];
        for (i, p_row) in p.iter_mut().enumerate() {
            for (j, p_val) in p_row.iter_mut().enumerate() {
                let mut sum = 0u8;
                for k in 0..256 {
                    sum ^= b_inv[i][k] & a[k][j];
                }
                *p_val = sum;
            }
        }

        p
    }

    /// Invert a 256×256 matrix over GF(2) using Gaussian elimination.
    fn invert_gf2(mat: &[[u8; 256]; 256]) -> [[u8; 256]; 256] {
        let mut a = *mat;
        let mut inv = [[0u8; 256]; 256];

        // Identity matrix
        for (i, row) in inv.iter_mut().enumerate() {
            row[i] = 1;
        }

        for col in 0..256 {
            // Find pivot
            let mut pivot = col;
            while pivot < 256 && a[pivot][col] == 0 {
                pivot += 1;
            }
            assert!(pivot < 256, "Matrix is singular over GF(2)");

            // Swap rows
            if pivot != col {
                a.swap(col, pivot);
                inv.swap(col, pivot);
            }

            // Eliminate other rows
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
