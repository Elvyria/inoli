pub mod capabilities;

use std::fmt::Debug;

use chrono::{Local, Utc};
use derive_more::{Deref, From};

automod::dir!(pub "src/devices");

#[derive(Deref)]
pub struct Version([u8; 4]);

#[derive(Debug)]
pub enum WearLocation {
    Left,
    Right,
    Neck,
    Pocket,
}

#[derive(Debug, Deref, From)]
pub struct DateTime(chrono::DateTime<Utc>);

impl From<chrono::DateTime<Local>> for DateTime {
    fn from(dt: chrono::DateTime<Local>) -> Self {
        Self(dt.with_timezone(&chrono::Utc))
    }
}

trait Status {}

enum Authorized {}
enum Unauthorized {}

impl Status for Authorized {}
impl Status for Unauthorized {}

struct Device<S: Status> {
    status:  std::marker::PhantomData<S>,
}

impl Device<Unauthorized> {
    fn authenticate() {}
}
