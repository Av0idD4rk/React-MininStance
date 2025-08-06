ALTER TABLE instances
    ADD COLUMN user_id INT NOT NULL REFERENCES users(id);