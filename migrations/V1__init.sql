CREATE TABLE IF NOT EXISTS travel (
    id INTEGER PRIMARY KEY,
    country VARCHAR(200),
    city VARCHAR(200),
    began TIMESTAMP,
    ended TIMESTAMP,
    photo_id INTEGER,
    FOREIGN KEY (photo_id) REFERENCES photo (id)
);

CREATE TABLE IF NOT EXISTS post (
    id INTEGER PRIMARY KEY,
    text VARCHAR,
    created TIMESTAMP,
    travel_id INTEGER,
    FOREIGN KEY (travel_id) REFERENCES travel (id)
);

CREATE TABLE IF NOT EXISTS photo (
    id INTEGER PRIMARY KEY,
    data BLOB,
    date TIMESTAMP,
    post_id INTEGER,
    FOREIGN KEY (post_id) REFERENCES post (id)
);
