-- ================================
-- Database Schema
-- ================================

-- Guilds Table
CREATE TABLE guilds (
    uuid INT PRIMARY KEY,
    name TEXT UNIQUE NOT NULL,
    experience_points NUMERIC DEFAULT 0 -- Guild experience points
);

-- Ranks Table (Dynamic Rank System)
CREATE TABLE ranks (
    id SERIAL PRIMARY KEY,
    name TEXT UNIQUE NOT NULL
);

-- Rank Permissions Table
CREATE TABLE rank_permissions (
    rank_id INT REFERENCES ranks(id) ON DELETE CASCADE,
    permission TEXT NOT NULL,
    PRIMARY KEY (rank_id, permission)
);

-- Players Table
CREATE TABLE players (
    uuid NUMERIC(39,0) PRIMARY KEY,
    rank_id INT REFERENCES ranks(id) ON DELETE SET NULL,
    is_banned BOOLEAN DEFAULT FALSE,
    guild_id INT REFERENCES guilds(uuid) ON DELETE SET NULL,
    first_login TIMESTAMP NOT NULL DEFAULT NOW(),
    last_login TIMESTAMP NOT NULL DEFAULT NOW(),
    coins INT DEFAULT 0,
    experience_points NUMERIC DEFAULT 0, -- Server experience points
    level INT GENERATED ALWAYS AS (
        FLOOR(
            (1 + SQRT(1 + 8 * experience_points / 500)) / 2
        )
    ) STORED -- Level scaling formula: XP needed increases exponentially
);

-- Friends Table (Many-to-Many Relationship)
CREATE TABLE friends (
    player1 NUMERIC(39,0) REFERENCES players(uuid) ON DELETE CASCADE,
    player2 NUMERIC(39,0) REFERENCES players(uuid) ON DELETE CASCADE,
    PRIMARY KEY (player1, player2),
    CHECK (player1 <> player2)
);

-- Minigame Stats Table
CREATE TABLE minigame_stats (
    id SERIAL PRIMARY KEY,
    player_id NUMERIC(39,0) REFERENCES players(uuid) ON DELETE CASCADE,
    minigame TEXT NOT NULL,
    stat_key TEXT NOT NULL,
    stat_value NUMERIC DEFAULT 0,
    UNIQUE (player_id, minigame, stat_key)
);

-- Minigame Inventories Table
CREATE TABLE minigame_inventories (
    player_id NUMERIC(39,0) REFERENCES players(uuid) ON DELETE CASCADE,
    minigame TEXT NOT NULL,
    inventory JSONB NOT NULL,
    PRIMARY KEY (player_id, minigame)
);

-- Achievements Table
CREATE TABLE achievements (
    id SERIAL PRIMARY KEY,
    achievement_name TEXT UNIQUE NOT NULL,
    description TEXT NOT NULL,
    reward NUMERIC DEFAULT 0
);

-- Player Achievements Table
CREATE TABLE player_achievements (
    player_id NUMERIC(39,0) REFERENCES players(uuid) ON DELETE CASCADE,
    achievement_id INT REFERENCES achievements(id) ON DELETE CASCADE,
    earned_at TIMESTAMP DEFAULT NOW(),
    PRIMARY KEY (player_id, achievement_id)
);
