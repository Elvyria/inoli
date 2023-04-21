#[macro_export]
macro_rules! ensure_length {
    ($b:expr,$expected:expr,$o:expr) => {
        if $b.len() == $expected {
            Ok($o)
        } else {
            Err(Error::Length { expected: $expected, actual: $b.len() })
        }
    }
}
