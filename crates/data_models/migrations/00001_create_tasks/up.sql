CREATE TABLE tasks (
                       name TEXT PRIMARY KEY,
                       dockerfile_path TEXT NOT NULL,
                       created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
