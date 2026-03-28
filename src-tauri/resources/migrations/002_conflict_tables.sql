CREATE TABLE IF NOT EXISTS mod_files (
    mod_id TEXT NOT NULL,
    relative_path TEXT NOT NULL,
    file_type TEXT NOT NULL,
    PRIMARY KEY (mod_id, relative_path)
);

CREATE INDEX IF NOT EXISTS idx_mod_files_path ON mod_files(relative_path);

CREATE TABLE IF NOT EXISTS script_ids (
    mod_id TEXT NOT NULL,
    script_type TEXT NOT NULL,
    script_id TEXT NOT NULL,
    file_path TEXT NOT NULL,
    PRIMARY KEY (mod_id, script_type, script_id, file_path)
);

CREATE INDEX IF NOT EXISTS idx_script_ids_type_id ON script_ids(script_type, script_id);
