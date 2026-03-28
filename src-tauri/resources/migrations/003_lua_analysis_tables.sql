-- Lua-level analysis tables for enhanced conflict detection

-- Global function/table definitions found in each mod's Lua files
CREATE TABLE IF NOT EXISTS lua_globals (
    mod_id TEXT NOT NULL,
    symbol_name TEXT NOT NULL,
    symbol_type TEXT NOT NULL DEFAULT 'function', -- 'function', 'table', 'variable'
    file_path TEXT NOT NULL,
    line INTEGER,
    PRIMARY KEY (mod_id, symbol_name, file_path)
);

CREATE INDEX IF NOT EXISTS idx_lua_globals_symbol ON lua_globals(symbol_name);

-- Event hooks registered by each mod
CREATE TABLE IF NOT EXISTS event_hooks (
    mod_id TEXT NOT NULL,
    event_name TEXT NOT NULL,
    callback_name TEXT NOT NULL,
    file_path TEXT NOT NULL,
    line INTEGER,
    PRIMARY KEY (mod_id, event_name, callback_name, file_path)
);

CREATE INDEX IF NOT EXISTS idx_event_hooks_event ON event_hooks(event_name);
