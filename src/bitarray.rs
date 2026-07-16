pub struct BitArray;

impl BitArray {
    #[inline]
    pub fn get_bit(bytes: &[u8], bit: usize) -> u8 {
        let byte = bit >> 3;
        let offset = bit & 7;

        if byte >= bytes.len() {
            return 0;
        }

        (bytes[byte] >> offset) & 1
    }

    #[inline]
    pub fn set_bit(bytes: &mut [u8], bit: usize, value: bool) {
        let byte = bit >> 3;
        let offset = bit & 7;

        if byte >= bytes.len() {
            return;
        }

        if value {
            bytes[byte] |= 1 << offset;
        } else {
            bytes[byte] &= !(1 << offset);
        }
    }

    #[inline]
    pub fn xor_bit(bytes: &mut [u8], bit: usize) {
        let byte = bit >> 3;
        let offset = bit & 7;

        if byte >= bytes.len() {
            return;
        }

        bytes[byte] ^= 1 << offset;
    }
}
