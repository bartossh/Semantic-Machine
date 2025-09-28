CREATE TABLE rss_items (
    hash TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    link TEXT NOT NULL,
    description TEXT NOT NULL,
    published_timestamp BIGINT NOT NULL,
    fetched_timestamp BIGINT NOT NULL,
    comments_url TEXT,
    category TEXT NOT NULL,
    author TEXT NOT NULL,
    article TEXT NOT NULL
);