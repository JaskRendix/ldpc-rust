pub mod h_128_256;
pub mod h_256_512;

/// Macro to easily define custom parity-check matrices for arbitrary (M, N) block codes
#[macro_export]
macro_rules! define_custom_matrix {
    ($vis:vis struct $name:ident, const M = $m:expr, const N = $n:expr, $matrix_data:expr) => {
        #[derive(Debug, Clone, Copy)]
        $vis struct $name;

        impl $name {
            pub const ROWS: usize = $m;
            pub const COLS: usize = $n;
            pub const DATA: [[u8; $n]; $m] = $matrix_data;
        }
    };
}
