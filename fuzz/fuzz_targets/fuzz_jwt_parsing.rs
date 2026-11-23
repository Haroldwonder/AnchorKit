#![no_main]

use libfuzzer_sys::fuzz_target;

fn has_nonzero_trailing_bits(input: &[u8]) -> bool {
    let mut end = input.len();
    while end > 0 && input[end - 1] == b'=' {
        end -= 1;
    }
    let input = &input[..end];
    let mask = match input.len() % 4 {
        2 => 0x0f,
        3 => 0x03,
        _ => return false,
    };
    let Some(&last) = input.last() else {
        return false;
    };
    let value = match last {
        b'A'..=b'Z' => last - b'A',
        b'a'..=b'z' => last - b'a' + 26,
        b'0'..=b'9' => last - b'0' + 52,
        b'-' => 62,
        b'_' => 63,
        _ => return false,
    };
    value & mask != 0
}

fuzz_target!(|input: &[u8]| {
    for segment in input.split(|&byte| byte == b'.') {
        let decoded = anchorkit::sep10_jwt::base64url_decode(segment);
        if has_nonzero_trailing_bits(segment) {
            assert!(decoded.is_err());
        }
    }
});
