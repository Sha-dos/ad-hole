CREATE TABLE queries (
  id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
  ts BIGINT NOT NULL,
  domain TEXT NOT NULL,
  blocked INTEGER NOT NULL
);

CREATE INDEX queries_ts ON queries (ts);
CREATE INDEX queries_domain ON queries (domain);
