fn battery_icon(level: u8) -> char {
    match level {
        90..101 => '',
        65..90  => '',
        35..65  => '',
        5..35   => '',
        0..5    => '',
    }
}
