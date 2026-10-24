CREATE TABLE sessions (
  id          TEXT PRIMARY KEY,           -- ULID
  name        TEXT,
  actor       TEXT NOT NULL,
  started_at  INTEGER NOT NULL,           -- unix ms
  ended_at    INTEGER
);

CREATE TABLE events (
  id          TEXT PRIMARY KEY,           -- ULID (time-sortable)
  ts          INTEGER NOT NULL,           -- unix ms
  kind        TEXT NOT NULL,              -- http | db | capture
  source      TEXT NOT NULL,              -- app | cli | simulator | browser | port
  actor       TEXT NOT NULL,              -- me | cursor | claude-code | codex | ...
  session_id  TEXT REFERENCES sessions(id),
  workspace   TEXT,                       -- absolute path of project root
  env         TEXT,
  request_ref TEXT,                       -- request file path, if any
  correlation TEXT,                       -- request id / trace id for timeline joins
  summary     TEXT NOT NULL,              -- "GET /users 200 34ms"
  note        TEXT
);
CREATE INDEX events_ts ON events(ts);
CREATE INDEX events_actor_ts ON events(actor, ts);
CREATE INDEX events_session ON events(session_id, ts);
CREATE INDEX events_correlation ON events(correlation);

CREATE TABLE http_exchanges (
  event_id     TEXT PRIMARY KEY REFERENCES events(id) ON DELETE CASCADE,
  method       TEXT NOT NULL,
  url          TEXT NOT NULL,             -- redacted
  req_headers  TEXT NOT NULL,             -- JSON, redacted
  req_body     BLOB,                      -- redacted
  status       INTEGER,
  res_headers  TEXT,
  res_body     BLOB,
  duration_ms  INTEGER,
  error        TEXT,
  redacted     TEXT NOT NULL DEFAULT '[]' -- JSON list of secret names masked
);

CREATE TABLE db_queries (
  event_id    TEXT PRIMARY KEY REFERENCES events(id) ON DELETE CASCADE,
  connection  TEXT NOT NULL,
  sql         TEXT NOT NULL,
  rows        INTEGER,
  duration_ms INTEGER,
  error       TEXT
);

CREATE TABLE tags (
  event_id TEXT NOT NULL REFERENCES events(id) ON DELETE CASCADE,
  tag      TEXT NOT NULL,
  PRIMARY KEY (event_id, tag)
);
CREATE INDEX tags_tag ON tags(tag);

CREATE VIRTUAL TABLE events_fts USING fts5(
  event_id UNINDEXED, summary, note, url, req_body, res_body,
  tokenize='unicode61'
);
