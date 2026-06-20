use chrono::prelude::*;

#[derive(Debug, Clone)]
pub struct AppState {
    pub language: String,
    pub travels: Vec<String>,
}

pub enum Cmd {
    LoadTravels { limit: u32, page: u32 },
    AddTravel { travel: Travel },
}

pub enum Response {
    LoadTravels { page: u32, travels: Vec<Travel> },
    AddTravel { id: Id },
}
pub type Id = i64;
#[derive(Debug, Clone)]
pub struct Travel {
    pub id: Id,
    pub country: String,
    pub city: String,
    pub began: NaiveDateTime,
    pub ended: NaiveDateTime,
    pub cover: Option<Photo>,
}

impl PartialEq for Travel {
    fn eq(&self, other: &Self) -> bool {
        self.country == other.country
            && self.city == other.city
            && self.began == other.began
            && self.ended == other.ended
    }
}

#[derive(Debug, Clone)]
pub struct Post {
    pub id: Id,
    pub photos: Vec<Photo>,
    pub text: String,
    pub created: NaiveDateTime,
}

impl PartialEq for Post {
    fn eq(&self, other: &Self) -> bool {
        self.photos == other.photos && self.text == other.text && self.created == other.created
    }
}
#[derive(Debug, Clone)]
pub struct Photo {
    pub id: Id,
    pub data: Vec<u8>,
    pub date: NaiveDateTime,
}

impl PartialEq for Photo {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data && self.date == other.date
    }
}
