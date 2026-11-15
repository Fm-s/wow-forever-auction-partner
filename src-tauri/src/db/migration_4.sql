-- User-defined item groups (shared by all houses); members are exact variants.
CREATE TABLE item_groups (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    created_at INTEGER NOT NULL
);
CREATE UNIQUE INDEX idx_item_groups_name ON item_groups(name COLLATE NOCASE);
CREATE TABLE item_group_members (
    group_id INTEGER NOT NULL REFERENCES item_groups(id) ON DELETE CASCADE,
    item_id INTEGER NOT NULL,
    variant_key TEXT NOT NULL,
    added_at INTEGER NOT NULL,
    PRIMARY KEY(group_id, item_id, variant_key)
);
