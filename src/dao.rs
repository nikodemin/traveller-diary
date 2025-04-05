use std::str::{self, Utf8Error};

use chrono::{DateTime, ParseError, Utc};
use egui::Image;
use rusqlite::{
    Connection,
    types::{FromSql, FromSqlError},
};

#[derive(Debug, Clone)]
pub struct Travel {
    pub id: i32,
    pub country: String,
    pub city: String,
    pub began: DateTime<Utc>,
    pub ended: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Post {
    pub id: i32,
    pub photos: Vec<Photo>,
    pub text: String,
    pub began: DateTime<Utc>,
    pub ended: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Photo {
    pub id: i32,
    pub data: Vec<u8>,
    pub date: DateTime<Utc>,
}

struct Dao {
    connection: Connection,
}

impl Dao {
    const DT_FORMAT: &str = "YYYY-MM-DD HH:MM:SS";

    fn new(connection: Connection) -> Self {
        Dao { connection }
    }
}

type Res<T> = Result<T, rusqlite::Error>;

trait DaoOps {
    fn init(&mut self) -> Result<(), refinery::Error>;

    fn add_travel(&self, travel: Travel) -> Res<()>;
    fn list_travels(&self, limut: u32, page: u32) -> Res<Vec<Travel>>;
    fn update_travel(&self, travel: Travel) -> Res<()>;
    fn delete_travels(&self, travel_ids: Vec<String>) -> Res<()>;

    fn add_post_to_travel(&self, travel_id: String, post: Post) -> Res<()>;
    fn list_posts(&self, travel_id: String, limit: u32, page: u32) -> Res<Vec<Post>>;
    fn update_post(&self, post: Post) -> Res<()>;
    fn delete_posts(&self, post_ids: Vec<String>) -> Res<()>;

    fn add_photo_to_post(&self, post_id: String, photo: Image) -> Res<()>;
    fn delete_photos(&self, photo_ids: Vec<String>) -> Res<()>;
}

mod embedded {
    use refinery::embed_migrations;
    embed_migrations!("./migrations");
}

struct Wrapper<T> {
    value: T,
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

impl FromSql for Wrapper<DateTime<Utc>> {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        match value {
            rusqlite::types::ValueRef::Text(text) => {
                let dt = DateTime::parse_from_str(
                    str::from_utf8(text).map_err(|e| Wrapper { value: e }.into())?,
                    Dao::DT_FORMAT,
                )
                .map_err(|e| Wrapper { value: e }.into())?
                .to_utc();

                Ok(Wrapper { value: dt })
            }
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

impl DaoOps for Dao {
    fn init(&mut self) -> Result<(), refinery::Error> {
        embedded::migrations::runner().run(&mut self.connection)?;
        Ok(())
    }

    fn add_travel(&self, travel: Travel) -> Res<()> {
        self.connection.execute(
            "insert into travel (id, country, city, began, ended) values (null, ?1, ?2, ?3, ?4)",
            (
                travel.country,
                travel.city,
                travel.began.format(Self::DT_FORMAT).to_string(),
                travel.ended.format(Self::DT_FORMAT).to_string(),
            ),
        )?;

        Ok(())
    }

    fn list_travels(&self, limut: u32, page: u32) -> Res<Vec<Travel>> {
        let mut stmnt = self.connection.prepare(
            "select t.id, t.country, t.city, t.began, t.ended
           from travel t
           order by t.began desc limit ?1 offset ?2",
        )?;
        let iter = stmnt.query_map([limut, page], |row| {
            Ok(Travel {
                id: row.get(0)?,
                country: row.get(1)?,
                city: row.get(2)?,
                began: row.get::<_, Wrapper<DateTime<Utc>>>(3)?.value,
                ended: row.get::<_, Wrapper<DateTime<Utc>>>(4)?.value,
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
                travel.began.format(Self::DT_FORMAT).to_string(),
                travel.ended.format(Self::DT_FORMAT).to_string(),
                travel.id,
            ),
        )?;

        Ok(())
    }

    fn delete_travels(&self, travel_ids: Vec<String>) -> Res<()> {
        let placeholders = std::iter::repeat("?")
            .take(travel_ids.len())
            .collect::<Vec<_>>()
            .join(",");

        let mut statement = self.connection.prepare(&format!(
            "delete from travel where id IN ({})",
            placeholders
        ))?;

        travel_ids.iter().enumerate().fold(Ok(()), |acc, (i, id)| {
            acc.and_then(|_| statement.raw_bind_parameter(i, id))
        })?;

        statement.raw_execute()?;

        Ok(())
    }

    fn add_post_to_travel(&self, travel_id: String, post: Post) -> Res<()> {
        self.connection.execute(
            "INSERT INTO post (id, travel_id, text, began, ended) VALUES (null, ?1, ?2, ?3, ?4)",
            (
                travel_id,
                post.text,
                post.began.format(Self::DT_FORMAT).to_string(),
                post.ended.format(Self::DT_FORMAT).to_string(),
            ),
        )?;

        Ok(())
    }

    fn list_posts(&self, travel_id: String, limit: u32, page: u32) -> Res<Vec<Post>> {
        let mut stmt = self.connection.prepare(
            "SELECT p.id, p.text, p.began, p.ended, ph.id, ph.data, ph.date
            FROM photo ph RIGHT JOIN (SELECT id, text, began, ended, travel_id FROM post LIMIT ?2 OFFSET ?3 ORDER BY ended DESC) p
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
                began: row.get::<_, Wrapper<DateTime<Utc>>>(2)?.value,
                ended: row.get::<_, Wrapper<DateTime<Utc>>>(3)?.value,
                photos: vec![Photo {
                    id: row.get(4)?,
                    data: row.get(5)?,
                    date: row.get::<_, Wrapper<DateTime<Utc>>>(6)?.value,
                }],
            });
        }

        posts.iter().fold(
            (Vec::<Post>::new(), None::<i32>),
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
        todo!()
    }

    fn delete_posts(&self, post_ids: Vec<String>) -> Res<()> {
        todo!()
    }

    fn add_photo_to_post(&self, post_id: String, photo: Image) -> Res<()> {
        todo!()
    }

    fn delete_photos(&self, photo_ids: Vec<String>) -> Res<()> {
        todo!()
    }
}
