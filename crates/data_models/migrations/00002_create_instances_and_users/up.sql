CREATE TABLE users (
                       id SERIAL PRIMARY KEY,
                       username TEXT UNIQUE NOT NULL,
                       created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE sessions (
                          id TEXT PRIMARY KEY,
                          user_id INT NOT NULL REFERENCES users(id),
                          created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                          expires_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE instances (
                           id SERIAL PRIMARY KEY,
                           task_name TEXT NOT NULL REFERENCES tasks(name),
                           container_id TEXT NOT NULL,
                           port INT NOT NULL,
                           created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                           expires_at TIMESTAMPTZ NOT NULL,
                           status TEXT NOT NULL  -- e.g. 'Running','Stopped','Expired'
);
