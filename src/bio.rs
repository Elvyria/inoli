#[derive(Clone, Copy)]
pub enum Sex {
    Male,
    Female
}

pub struct Bio {
    pub sex:    Sex,
    pub age:    u8,
    pub height: u8, // cm
    pub weight: u8, // kg
}

impl Default for Bio {
    fn default() -> Self {
        Self {
            sex:    Sex::Male,
            age:    14,
            height: 162,
            weight: 54,
        }
    }
}
