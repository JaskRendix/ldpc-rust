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

    #[inline]
    pub fn xor_bytes(dst: &mut [u8], src: &[u8]) {
        let len = core::cmp::min(dst.len(), src.len());
        let (dst_chunks, dst_remainder) = dst[..len].split_at_mut(len & !7usize);
        let (src_chunks, src_remainder) = src[..len].split_at(len & !7usize);

        let (dst_word_chunks, _) = dst_chunks.as_chunks_mut::<8>();
        let (src_word_chunks, _) = src_chunks.as_chunks::<8>();

        for (d, s) in dst_word_chunks.iter_mut().zip(src_word_chunks.iter()) {
            let d_word = u64::from_ne_bytes(*d);
            let s_word = u64::from_ne_bytes(*s);
            *d = (d_word ^ s_word).to_ne_bytes();
        }

        for (d, s) in dst_remainder.iter_mut().zip(src_remainder.iter()) {
            *d ^= *s;
        }
    }
}
