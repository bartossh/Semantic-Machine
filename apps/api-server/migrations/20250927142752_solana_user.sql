CREATE TABLE IF NOT EXISTS solana_users (
    solana_wallet_public_key BYTEA PRIMARY KEY,
    created_at BIGINT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_solana_users_created_at
ON solana_users (created_at);