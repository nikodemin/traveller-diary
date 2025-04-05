CREATE IF NOT EXISTS TABLE travel (
    id INTEGER PRIMARY KEY,
    country VARCHAR(200),
    city VARCHAR(200),
    began TIMESTAMP,
    ended TIMESTAMP,
);

CREATE IF NOT EXISTS TABLE post (
    id INTEGER PRIMARY KEY,
    text VARCHAR,
    began TIMESTAMP,
    ended TIMESTAMP,
    travel_id INTEGER,
    FOREIGN KEY (travel_id) REFERENCES travel(id)
);

CREATE TABLE photo (id INTEGER PRIMARY KEY, data: BLOB, date TIMESTAMP, post_id INTEGER, FOREIGN KEY (post_id) REFERENCES post(id));
