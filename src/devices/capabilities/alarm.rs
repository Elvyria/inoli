pub enum AlarmFrequency {
    Once,
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,

    Workweek,
    Weekends,
    Everyday,
}

impl AlarmFrequency {
    pub fn as_bits(self) -> u8 {
        match self {
            AlarmFrequency::Once      => 0b0,
            AlarmFrequency::Monday    => 0b1,
            AlarmFrequency::Tuesday   => 0b10,
            AlarmFrequency::Wednesday => 0b100,
            AlarmFrequency::Thursday  => 0b1000,
            AlarmFrequency::Friday    => 0b10000,
            AlarmFrequency::Saturday  => 0b100000,
            AlarmFrequency::Sunday    => 0b1000000,
            AlarmFrequency::Workweek  => 0b11111,
            AlarmFrequency::Weekends  => 0b1100000,
            AlarmFrequency::Everyday  => 0b1111111,
        }
    }
}
