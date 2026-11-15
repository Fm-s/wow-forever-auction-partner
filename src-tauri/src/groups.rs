//! User-defined item groups, shared by all houses. Members are exact variants (item id + bonus-list key).
use rusqlite::{params, Connection};
use serde::Serialize;

pub const MAX_NAME_CHARS: usize = 60;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupMember {
    pub item_id: i64,
    pub variant_key: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub id: i64,
    pub name: String,
    pub members: Vec<GroupMember>,
}

#[derive(thiserror::Error, Debug)]
pub enum GroupError {
    #[error("{0}")]
    Invalid(String),
    #[error(transparent)]
    Db(#[from] rusqlite::Error),
}

/// All groups with their members, ordered by name (case-insensitive).
pub fn list_groups(conn: &Connection) -> rusqlite::Result<Vec<Group>> {
    let mut groups: Vec<Group> = conn
        .prepare("SELECT id, name FROM item_groups ORDER BY name COLLATE NOCASE, id")?
        .query_map([], |r| Ok(Group { id: r.get(0)?, name: r.get(1)?, members: vec![] }))?
        .collect::<rusqlite::Result<_>>()?;
    let mut st = conn.prepare("SELECT group_id, item_id, variant_key FROM item_group_members ORDER BY added_at, item_id, variant_key")?;
    let mut rows = st.query([])?;
    while let Some(r) = rows.next()? {
        let gid: i64 = r.get(0)?;
        if let Some(g) = groups.iter_mut().find(|g| g.id == gid) {
            g.members.push(GroupMember { item_id: r.get(1)?, variant_key: r.get(2)? });
        }
    }
    Ok(groups)
}

/// Trimmed, non-empty, at most [`MAX_NAME_CHARS`], and not used by another group (ignoring case).
fn valid_name(conn: &Connection, name: &str, except: Option<i64>) -> Result<String, GroupError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(GroupError::Invalid("Group name can't be empty".into()));
    }
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(GroupError::Invalid(format!("Group names are at most {MAX_NAME_CHARS} characters")));
    }
    let taken: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM item_groups WHERE name = ?1 COLLATE NOCASE AND id IS NOT ?2)",
        params![name, except],
        |r| r.get(0),
    )?;
    if taken {
        return Err(GroupError::Invalid(format!("A group named {name} already exists")));
    }
    Ok(name.to_string())
}

pub fn create_group(conn: &Connection, name: &str, now: i64) -> Result<Group, GroupError> {
    let name = valid_name(conn, name, None)?;
    conn.execute("INSERT INTO item_groups(name, created_at) VALUES (?1, ?2)", params![name, now])?;
    Ok(Group { id: conn.last_insert_rowid(), name, members: vec![] })
}

pub fn rename_group(conn: &Connection, id: i64, name: &str) -> Result<(), GroupError> {
    let name = valid_name(conn, name, Some(id))?;
    conn.execute("UPDATE item_groups SET name = ?1 WHERE id = ?2", params![name, id])?;
    Ok(())
}

pub fn delete_group(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM item_groups WHERE id = ?1", [id])?;
    Ok(())
}

/// Adds (`member = true`) or removes a variant. Both are no-ops when already in that state.
pub fn set_member(conn: &Connection, group_id: i64, item_id: i64, variant_key: &str, member: bool, now: i64) -> rusqlite::Result<()> {
    if member {
        conn.execute(
            "INSERT OR IGNORE INTO item_group_members(group_id, item_id, variant_key, added_at) VALUES (?1, ?2, ?3, ?4)",
            params![group_id, item_id, variant_key, now],
        )?;
    } else {
        conn.execute("DELETE FROM item_group_members WHERE group_id = ?1 AND item_id = ?2 AND variant_key = ?3", params![group_id, item_id, variant_key])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;

    fn names(c: &Connection) -> Vec<String> {
        list_groups(c).unwrap().into_iter().map(|g| g.name).collect()
    }

    fn invalid(r: Result<impl std::fmt::Debug, GroupError>) -> String {
        match r {
            Err(GroupError::Invalid(m)) => m,
            other => panic!("expected a validation error, got {other:?}"),
        }
    }

    #[test]
    fn create_trims_and_lists_sorted() {
        let c = open_in_memory();
        let g = create_group(&c, "  Herbs ", 1).unwrap();
        assert_eq!((g.name.as_str(), g.members.len()), ("Herbs", 0));
        create_group(&c, "flips", 1).unwrap();
        create_group(&c, "Armor", 1).unwrap();
        assert_eq!(names(&c), vec!["Armor", "flips", "Herbs"]);
    }

    #[test]
    fn names_are_validated() {
        let c = open_in_memory();
        assert_eq!(invalid(create_group(&c, "   ", 1)), "Group name can't be empty");
        assert_eq!(invalid(create_group(&c, &"x".repeat(61), 1)), "Group names are at most 60 characters");
        assert!(create_group(&c, &"é".repeat(60), 1).is_ok(), "the limit counts characters, not bytes");
        let id = create_group(&c, "Ok", 1).unwrap().id;
        assert_eq!(invalid(rename_group(&c, id, "")), "Group name can't be empty");
    }

    #[test]
    fn duplicate_names_rejected_ignoring_case() {
        let c = open_in_memory();
        create_group(&c, "Flips", 1).unwrap();
        assert_eq!(invalid(create_group(&c, "flips ", 1)), "A group named flips already exists");
        let other = create_group(&c, "Herbs", 1).unwrap().id;
        assert_eq!(invalid(rename_group(&c, other, "FLIPS")), "A group named FLIPS already exists");
    }

    #[test]
    fn rename_to_own_name_in_other_case_is_allowed() {
        let c = open_in_memory();
        let id = create_group(&c, "flips", 1).unwrap().id;
        rename_group(&c, id, "Flips").unwrap();
        assert_eq!(names(&c), vec!["Flips"]);
    }

    #[test]
    fn membership_is_per_variant_and_idempotent() {
        let c = open_in_memory();
        let id = create_group(&c, "Swords", 1).unwrap().id;
        set_member(&c, id, 727, "12920", true, 1).unwrap();
        set_member(&c, id, 727, "12920", true, 2).unwrap();
        set_member(&c, id, 727, "12718", true, 2).unwrap();
        set_member(&c, id, 2589, "", true, 2).unwrap();
        set_member(&c, id, 727, "12718", false, 3).unwrap();
        set_member(&c, id, 9999, "", false, 3).unwrap();
        let g = &list_groups(&c).unwrap()[0];
        let keys: Vec<(i64, &str)> = g.members.iter().map(|m| (m.item_id, m.variant_key.as_str())).collect();
        assert_eq!(keys, vec![(727, "12920"), (2589, "")]);
    }

    #[test]
    fn deleted_ids_are_never_reused() {
        let c = open_in_memory();
        let first = create_group(&c, "A", 1).unwrap().id;
        let newest = create_group(&c, "B", 1).unwrap().id;
        delete_group(&c, newest).unwrap();
        let next = create_group(&c, "C", 1).unwrap().id;
        assert!(next != newest && next != first, "a new group must not inherit a deleted group's id (and its filter selection)");
    }

    #[test]
    fn delete_removes_members() {
        let c = open_in_memory();
        let id = create_group(&c, "Gone", 1).unwrap().id;
        set_member(&c, id, 727, "", true, 1).unwrap();
        delete_group(&c, id).unwrap();
        assert!(list_groups(&c).unwrap().is_empty());
        let left: i64 = c.query_row("SELECT COUNT(*) FROM item_group_members", [], |r| r.get(0)).unwrap();
        assert_eq!(left, 0);
    }
}
