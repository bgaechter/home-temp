CREATE TABLE device (
  active_time BIGINT,
  create_time BIGINT,
  id TEXT PRIMARY KEY,
  name TEXT,
  online BOOLEAN,
  sub BOOLEAN,
  time_zone TEXT,
  update_time BIGINT,
  device_type TEXT
);

CREATE TABLE device_status (
  id TEXT,
  code TEXT,
  value TEXT,
	device_id TEXT,
	update_time TIMESTAMP,
  PRIMARY KEY (id),
  FOREIGN KEY (device_id) REFERENCES device(id)
);

