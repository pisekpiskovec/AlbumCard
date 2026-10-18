/// Stars (0-5) to the raw byte writtent to files.
/// Conversion according to Microsoft Windows Media Player 9 Series.
pub fn stars_to_raw(stars: u8) -> u8 {
    match stars {
        0 => 0,
        1 => 1,
        2 => 64,
        3 => 128,
        4 => 196,
        _ => 255,
    }
}

/// Raw byte to the nearest star count (0-5).
pub fn raw_to_stars(stars: u8) -> u8 {
    match stars {
        0 => 0,
        1..=31 => 1,
        32..=95 => 2,
        96..=159 => 3,
        160..=223 => 4,
        224..=255 => 5,
    }
}
