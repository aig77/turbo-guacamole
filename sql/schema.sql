CREATE TABLE IF NOT EXISTS urls (
  code VARCHAR(6) PRIMARY KEY,
  url TEXT NOT NULL UNIQUE,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS clicks (
  id BIGSERIAL PRIMARY KEY,
  code VARCHAR(6) REFERENCES urls(code) ON DELETE CASCADE,
  clicked_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- speeds up queries that filter or join on the code field in the clicks table
CREATE INDEX idx_clicks_code_date ON clicks(code, clicked_at);
