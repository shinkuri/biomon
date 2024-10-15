-- Step 1: Create a new table with the UNIQUE constraint
CREATE TABLE IF NOT EXISTS bp_new (
    id          INTEGER PRIMARY KEY,
    timestamp   INTEGER UNIQUE NOT NULL,
    sys         INTEGER NOT NULL,
    dia         INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS heartrate_new (
    id          INTEGER PRIMARY KEY,
    timestamp   INTEGER UNIQUE NOT NULL,
    heartrate   INTEGER NOT NULL,
    duration    INTEGER DEFAULT (0) NOT NULL
);
CREATE TABLE IF NOT EXISTS mood_new (
    id          INTEGER PRIMARY KEY,
    timestamp   INTEGER UNIQUE NOT NULL,
    mood        TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS weight_new (
    id          INTEGER PRIMARY KEY,
    timestamp   INTEGER UNIQUE NOT NULL,
    weight      REAL NOT NULL
);

-- Step 2: Copy data from the old table to the new table
INSERT INTO bp_new (id, timestamp, sys, dia)
SELECT id, timestamp, sys, dia
FROM bp;

INSERT INTO heartrate_new (id, timestamp, heartrate)
SELECT id, timestamp, heartrate
FROM bp;

INSERT INTO mood_new (id, timestamp, mood)
SELECT id, timestamp, mood
FROM bp;

INSERT INTO weight_new (id, timestamp, weight)
SELECT id, timestamp, weight
FROM bp;

-- Step 3: Drop the old table
DROP TABLE bp;
DROP TABLE heartrate;
DROP TABLE mood;
DROP TABLE weight;

-- Step 4: Rename the new table to the old table's name
ALTER TABLE bp_new RENAME TO bp;
ALTER TABLE heartrate_new RENAME TO heartrate;
ALTER TABLE mood_new RENAME TO mood;
ALTER TABLE weight_new RENAME TO weight;
