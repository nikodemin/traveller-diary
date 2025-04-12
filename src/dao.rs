use std::str::{self, Utf8Error};

use chrono::{NaiveDateTime, ParseError};
use rusqlite::{
    Connection, ToSql,
    types::{FromSql, FromSqlError},
};

type Id = i64;

#[derive(Debug, Clone)]
pub struct Travel {
    pub id: Id,
    pub country: String,
    pub city: String,
    pub began: NaiveDateTime,
    pub ended: NaiveDateTime,
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
    pub began: NaiveDateTime,
    pub ended: NaiveDateTime,
}

impl PartialEq for Post {
    fn eq(&self, other: &Self) -> bool {
        self.photos == other.photos
            && self.text == other.text
            && self.began == other.began
            && self.ended == other.ended
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

struct Dao {
    connection: Connection,
}

impl Dao {
    const DT_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

    fn new(connection: Connection) -> Self {
        Dao { connection }
    }
}

type Res<T> = Result<T, rusqlite::Error>;

trait DaoOps {
    fn init(&mut self) -> Result<(), refinery::Error>;

    fn add_travel(&self, travel: Travel) -> Res<Id>;
    fn list_travels(&self, limut: u32, page: u32) -> Res<Vec<Travel>>;
    fn update_travel(&self, travel: Travel) -> Res<()>;
    fn delete_travels(&self, travel_ids: Vec<Id>) -> Res<()>;

    fn add_post_to_travel(&self, travel_id: Id, post: Post) -> Res<Id>;
    fn list_posts(&self, travel_id: Id, limit: u32, page: u32) -> Res<Vec<Post>>;
    fn update_post(&self, post: Post) -> Res<()>;
    fn delete_posts(&self, post_ids: Vec<Id>) -> Res<()>;

    fn add_photos_to_post(&self, post_id: Id, photos: Vec<Photo>) -> Res<()>;
    fn delete_photos(&self, photo_ids: Vec<Id>) -> Res<()>;
}

mod embedded {
    use refinery::embed_migrations;
    embed_migrations!("./migrations");
}

struct Wrapper<T> {
    value: T,
}

trait WrapperOps: Sized {
    fn wrap(self) -> Wrapper<Self>;
}

impl WrapperOps for NaiveDateTime {
    fn wrap(self) -> Wrapper<Self> {
        Wrapper { value: self }
    }
}

impl Into<FromSqlError> for Wrapper<ParseError> {
    fn into(self) -> FromSqlError {
        FromSqlError::Other(Box::new(self.value))
    }
}

impl Into<FromSqlError> for Wrapper<Utf8Error> {
    fn into(self) -> FromSqlError {
        FromSqlError::Other(Box::new(self.value))
    }
}

impl FromSql for Wrapper<NaiveDateTime> {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        match value {
            rusqlite::types::ValueRef::Text(text) => {
                let dt = NaiveDateTime::parse_from_str(
                    str::from_utf8(text).map_err(|e| Wrapper { value: e }.into())?,
                    Dao::DT_FORMAT,
                )
                .map_err(|e| Wrapper { value: e }.into())?;

                Ok(Wrapper { value: dt })
            }
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

impl ToSql for Wrapper<NaiveDateTime> {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        let str = self.value.format(Dao::DT_FORMAT).to_string();

        Ok(rusqlite::types::ToSqlOutput::from(str))
    }
}

impl DaoOps for Dao {
    fn init(&mut self) -> Result<(), refinery::Error> {
        embedded::migrations::runner().run(&mut self.connection)?;
        Ok(())
    }

    fn add_travel(&self, travel: Travel) -> Res<Id> {
        self.connection.execute(
            "insert into travel (id, country, city, began, ended) values (null, ?1, ?2, ?3, ?4)",
            (
                travel.country,
                travel.city,
                travel.began.wrap(),
                travel.ended.wrap(),
            ),
        )?;

        Ok(self.connection.last_insert_rowid())
    }

    fn list_travels(&self, limit: u32, page: u32) -> Res<Vec<Travel>> {
        let mut stmnt = self.connection.prepare(
            "select t.id, t.country, t.city, t.began, t.ended
           from travel t
           order by t.began desc limit ?1 offset ?2",
        )?;
        let iter = stmnt.query_map([limit, page * limit], |row| {
            Ok(Travel {
                id: row.get(0)?,
                country: row.get(1)?,
                city: row.get(2)?,
                began: row.get::<_, Wrapper<NaiveDateTime>>(3)?.value,
                ended: row.get::<_, Wrapper<NaiveDateTime>>(4)?.value,
            })
        })?;

        iter.collect()
    }

    fn update_travel(&self, travel: Travel) -> Res<()> {
        self.connection.execute(
            "UPDATE travel SET country = ?1, city = ?2, began = ?3, ended = ?4 WHERE id = ?5",
            (
                travel.country,
                travel.city,
                travel.began.wrap(),
                travel.ended.wrap(),
                travel.id,
            ),
        )?;

        Ok(())
    }

    fn delete_travels(&self, travel_ids: Vec<Id>) -> Res<()> {
        let placeholders = std::iter::repeat("?")
            .take(travel_ids.len())
            .collect::<Vec<_>>()
            .join(",");

        let mut statement = self.connection.prepare(&format!(
            "delete from travel where id IN ({})",
            placeholders
        ))?;

        travel_ids.iter().enumerate().fold(Ok(()), |acc, (i, id)| {
            acc.and_then(|_| statement.raw_bind_parameter(i + 1, id))
        })?;

        statement.raw_execute()?;

        Ok(())
    }

    fn add_post_to_travel(&self, travel_id: Id, post: Post) -> Res<Id> {
        self.connection.execute(
            "INSERT INTO post (id, travel_id, text, began, ended) VALUES (null, ?1, ?2, ?3, ?4)",
            (travel_id, post.text, post.began.wrap(), post.ended.wrap()),
        )?;

        Ok(self.connection.last_insert_rowid())
    }

    fn list_posts(&self, travel_id: Id, limit: u32, page: u32) -> Res<Vec<Post>> {
        let mut stmt = self.connection.prepare(
            "SELECT p.id, p.text, p.began, p.ended, ph.id, ph.data, ph.date
            FROM photo ph RIGHT JOIN (SELECT id, text, began, ended, travel_id FROM post ORDER BY ended DESC LIMIT ?2 OFFSET ?3) p
            ON p.id = ph.post_id
            WHERE p.travel_id = ?1
            ORDER BY p.id"
        )?;

        let mut rows = stmt.query((travel_id, limit, page * limit))?;

        let mut posts = Vec::new();

        while let Some(row) = rows.next()? {
            posts.push(Post {
                id: row.get(0)?,
                text: row.get(1)?,
                began: row.get::<_, Wrapper<NaiveDateTime>>(2)?.value,
                ended: row.get::<_, Wrapper<NaiveDateTime>>(3)?.value,
                photos: vec![Photo {
                    id: row.get(4)?,
                    data: row.get(5)?,
                    date: row.get::<_, Wrapper<NaiveDateTime>>(6)?.value,
                }],
            });
        }

        posts.iter().fold(
            (Vec::<Post>::new(), None::<Id>),
            |(mut posts, post_id), post| {
                if let Some(id) = post_id {
                    if id == post.id {
                        match posts.last_mut() {
                            Some(last_post) => {
                                last_post.photos.append(&mut post.photos.clone());
                            }
                            None => posts.push(post.to_owned()),
                        }
                    }
                }

                (posts, Some(post.id))
            },
        );

        Ok(posts)
    }

    fn update_post(&self, post: Post) -> Res<()> {
        self.connection.execute(
            "UPDATE posts SET text = ?, began = ?, ended = ? WHERE id = ?",
            (post.text, post.began.wrap(), post.ended.wrap(), post.id),
        )?;

        Ok(())
    }

    fn delete_posts(&self, post_ids: Vec<Id>) -> Res<()> {
        let placeholders = std::iter::repeat("?")
            .take(post_ids.len())
            .collect::<Vec<_>>()
            .join(",");

        let mut statement = self
            .connection
            .prepare(&format!("delete from posts where id IN ({})", placeholders))?;

        post_ids.iter().enumerate().fold(Ok(()), |acc, (i, id)| {
            acc.and_then(|_| statement.raw_bind_parameter(i, id))
        })?;

        statement.raw_execute()?;
        Ok(())
    }

    fn add_photos_to_post(&self, post_id: Id, photos: Vec<Photo>) -> Res<()> {
        let placeholders = std::iter::repeat("(NULL, ?, ?, ?)")
            .take(photos.len())
            .collect::<Vec<_>>()
            .join(",");

        let mut statement = self.connection.prepare(&format!(
            "INSERT INTO photo (id, post_id, data, date) VALUES {}",
            placeholders
        ))?;

        photos.iter().enumerate().fold(Ok(()), |acc, (i, photo)| {
            acc.and_then(|_| statement.raw_bind_parameter(i * 3, post_id))
                .and_then(|_| statement.raw_bind_parameter(i * 3 + 1, photo.data.clone()))
                .and_then(|_| statement.raw_bind_parameter(i * 3 + 2, photo.date.wrap()))
        })?;

        Ok(())
    }

    fn delete_photos(&self, photo_ids: Vec<Id>) -> Res<()> {
        let placeholders = std::iter::repeat("?")
            .take(photo_ids.len())
            .collect::<Vec<_>>()
            .join(",");

        let mut statement = self
            .connection
            .prepare(&format!("delete from photo where id IN ({})", placeholders))?;

        photo_ids.iter().enumerate().fold(Ok(()), |acc, (i, id)| {
            acc.and_then(|_| statement.raw_bind_parameter(i, id))
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Days, NaiveDateTime};

    use super::*;
    use {prop::prelude::*, proptest as prop};

    fn init() -> Dao {
        let conn: Connection = Connection::open_in_memory().unwrap();
        let mut dao = Dao::new(conn);
        dao.init().unwrap();
        dao
    }

    fn travel() -> impl Strategy<Value = Travel> {
        let country = prop::sample::select(["USA", "Canada", "Mexico"].as_slice());
        let city = prop::sample::select(["New York", "Toronto", "Mexico City"].as_slice());
        let began = prop::sample::select(
            [
                "2012-01-01 12:45:04",
                "2022-02-01 19:00:03",
                "2024-03-01 00:01:11",
            ]
            .as_slice(),
        );
        let duration = prop::sample::select((1..30).collect::<Vec<_>>());

        (country, city, began, duration).prop_map(|(country, city, began, duration)| {
            let began = NaiveDateTime::parse_from_str(began, Dao::DT_FORMAT).unwrap();
            let ended = began.checked_add_days(Days::new(duration)).unwrap();

            Travel {
                id: 0,
                country: country.to_string(),
                city: city.to_string(),
                began,
                ended,
            }
        })
    }

    fn post() -> impl Strategy<Value = Post> {
        let text = prop::string::string_regex("\\w{20,100}").unwrap();
        let began = prop::sample::select(
            [
                "2012-01-01 12:45:04",
                "2022-02-01 19:00:03",
                "2024-03-01 00:01:11",
            ]
            .as_slice(),
        );

        let duration = prop::sample::select((1..30).collect::<Vec<_>>());

        (text, began, duration).prop_map(|(text, began, duration)| {
            let began = NaiveDateTime::parse_from_str(began, Dao::DT_FORMAT).unwrap();
            let ended = began.checked_add_days(Days::new(duration)).unwrap();

            Post {
                id: 0,
                photos: vec![],
                text: text.to_string(),
                began,
                ended,
            }
        })
    }

    fn photo() -> impl Strategy<Value = Photo> {
        let data = prop::string::bytes_regex(".{100, 200}").unwrap();
        let date = prop::sample::select(
            [
                "2012-01-01 12:45:04",
                "2022-02-01 19:00:03",
                "2024-03-01 00:01:11",
            ]
            .as_slice(),
        );

        (data, date).prop_map(|(data, date)| {
            let date = NaiveDateTime::parse_from_str(date, Dao::DT_FORMAT).unwrap();

            Photo { id: 0, data, date }
        })
    }

    fn vec_gen<T: Strategy>(value: T) -> impl Strategy<Value = Vec<T::Value>> {
        prop::collection::vec(value, 10)
    }

    proptest! {
        #[test]
        fn create_and_list(travels in vec_gen(travel())) {
            let dao = init();

            for travel in travels.iter() {
                dao.add_travel(travel.clone()).unwrap();
            };

            let result = dao.list_travels(10,0).unwrap();
            let diff = travels.iter().filter(|x| !result.contains(x)).count();
            let diff2 = result.iter().filter(|x| !travels.contains(x)).count();

            prop_assert!(diff == 0 && diff2 == 0)
        }

        #[test]
        fn create_and_update_travel(travel in travel(), travel2 in travel()) {
            let dao = init();

            dao.add_travel(travel.clone()).unwrap();

            let updated = Travel {
                id: 1,
                ..travel2
            };
            dao.update_travel(updated.clone()).unwrap();

            let res = dao.list_travels(1, 0).unwrap();

            prop_assert_eq!(res, vec![updated])
        }

        #[test]
        fn create_and_delete_travel(travel in travel()) {
            let dao = init();

            dao.add_travel(travel.clone()).unwrap();

            dao.delete_travels(vec![1]).unwrap();

            let res = dao.list_travels(1, 0).unwrap();

            prop_assert_eq!(res, vec![])
        }

        #[test]
        fn add_post_to_travel(travel in travel(), posts in vec_gen(post())) {
            let dao = init();

            dao.add_travel(travel.clone()).unwrap();

            for post in posts.clone() {
                dao.add_post_to_travel(1, post.clone()).unwrap();
            }

            let res = dao.list_posts(1, 10, 0).unwrap();

            prop_assert_eq!(res, posts)
        }

    }

    #[test]
    fn indepotent_init() {
        let mut session_manager = init();
        session_manager.init().unwrap();
        session_manager.init().unwrap();
    }
}
