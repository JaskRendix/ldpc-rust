use ldpc_rust::bitarray::BitArray;

#[test]
fn get_bit_returns_correct_values() {
    let bytes = [0b0000_0001u8, 0b1000_0000u8];

    assert_eq!(BitArray::get_bit(&bytes, 0), 1); // LSB of first byte
    assert_eq!(BitArray::get_bit(&bytes, 1), 0);
    assert_eq!(BitArray::get_bit(&bytes, 7), 0);
    assert_eq!(BitArray::get_bit(&bytes, 8), 0); // LSB of second byte
    assert_eq!(BitArray::get_bit(&bytes, 15), 1); // MSB of second byte
}

#[test]
fn get_bit_out_of_bounds_returns_zero() {
    let bytes = [0b1010_1010u8];

    assert_eq!(BitArray::get_bit(&bytes, 8), 0);
    assert_eq!(BitArray::get_bit(&bytes, 100), 0);
}

#[test]
fn set_bit_sets_and_clears_bits_correctly() {
    let mut bytes = [0u8; 2];

    BitArray::set_bit(&mut bytes, 0, true);
    assert_eq!(bytes[0], 0b0000_0001);

    BitArray::set_bit(&mut bytes, 7, true);
    assert_eq!(bytes[0], 0b1000_0001);

    BitArray::set_bit(&mut bytes, 0, false);
    assert_eq!(bytes[0], 0b1000_0000);

    BitArray::set_bit(&mut bytes, 8, true);
    assert_eq!(bytes[1], 0b0000_0001);
}

#[test]
fn set_bit_out_of_bounds_is_noop() {
    let mut bytes = [0b1111_0000u8];

    BitArray::set_bit(&mut bytes, 8, true);
    BitArray::set_bit(&mut bytes, 100, false);

    assert_eq!(bytes, [0b1111_0000u8]);
}

#[test]
fn xor_bit_flips_target_bit_only() {
    let mut bytes = [0b0000_0000u8];

    BitArray::xor_bit(&mut bytes, 0);
    assert_eq!(bytes[0], 0b0000_0001);

    BitArray::xor_bit(&mut bytes, 0);
    assert_eq!(bytes[0], 0b0000_0000);

    BitArray::set_bit(&mut bytes, 7, true);
    BitArray::xor_bit(&mut bytes, 7);
    assert_eq!(bytes[0], 0b0000_0000);
}

#[test]
fn xor_bit_out_of_bounds_is_noop() {
    let mut bytes = [0b0101_0101u8];

    BitArray::xor_bit(&mut bytes, 8);
    BitArray::xor_bit(&mut bytes, 100);

    assert_eq!(bytes, [0b0101_0101u8]);
}

#[test]
fn xor_bytes_xors_equal_length_slices() {
    let mut dst = [0b1010_1010u8, 0b1111_0000u8];
    let src = [0b0101_0101u8, 0b0000_1111u8];

    BitArray::xor_bytes(&mut dst, &src);

    assert_eq!(dst, [0b1111_1111u8, 0b1111_1111u8]);
}

#[test]
fn xor_bytes_uses_min_length_for_mismatched_slices() {
    let mut dst = [0b0000_0000u8, 0b1111_0000u8, 0b1010_1010u8];
    let src = [0b1111_1111u8, 0b0000_1111u8];

    BitArray::xor_bytes(&mut dst, &src);

    assert_eq!(dst, [0b1111_1111u8, 0b1111_1111u8, 0b1010_1010u8]);
}

#[test]
fn xor_bytes_zero_length_is_noop() {
    let mut dst: [u8; 0] = [];
    let src: [u8; 0] = [];

    BitArray::xor_bytes(&mut dst, &src);

    assert_eq!(dst, []);
}

#[test]
fn xor_bytes_handles_non_aligned_lengths() {
    let mut dst = [0xFFu8; 10]; // 10 bytes, not multiple of 8
    let src = [0x0Fu8; 10];

    BitArray::xor_bytes(&mut dst, &src);

    for (i, &b) in dst.iter().enumerate() {
        assert_eq!(b, 0xFF ^ 0x0F, "index {i} mismatch");
    }
}

#[test]
fn xor_bytes_is_self_inverse() {
    let mut dst = [0xAAu8, 0x55u8, 0xFFu8, 0x00u8];
    let src = [0x0Fu8, 0xF0u8, 0x33u8, 0xCCu8];

    let original = dst;

    BitArray::xor_bytes(&mut dst, &src);
    BitArray::xor_bytes(&mut dst, &src);

    assert_eq!(dst, original);
}

#[test]
fn xor_bytes_does_not_panic_on_large_input() {
    let mut dst = [0u8; 64];
    let src = [0xFFu8; 64];

    BitArray::xor_bytes(&mut dst, &src);

    assert!(dst.iter().all(|&b| b == 0xFF));
}
