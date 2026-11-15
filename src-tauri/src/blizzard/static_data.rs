use crate::blizzard::client::{BlizzardClient, FetchError};
use crate::blizzard::tooltip::{parse_tooltip, ItemTooltip};
use crate::settings::{get_kv, set_kv};
use futures::stream::{self, StreamExt};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

pub const NAMESPACE_STATIC: &str = "static-forever-us";
/// Forever's static data lacks some vanilla items (e.g. random-suffix gear); Classic Era has them.
pub const NAMESPACE_FALLBACK: &str = "static-classic1x-us";
/// `static_ns` of items fetched from the fallback namespace is prefixed with this.
pub const FALLBACK_PREFIX: &str = "fallback:";
/// `static_ns` of placeholder rows for items found in neither namespace.
pub const NS_MISSING: &str = "missing";
const CONCURRENCY: usize = 10;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ItemMeta {
    pub item_id: i64,
    pub name: String,
    pub quality: Option<String>,
    pub item_class: Option<String>,
    pub item_subclass: Option<String>,
    pub level: Option<i64>,
    pub icon_url: Option<String>,
    pub purchase_price: Option<i64>,
    pub purchase_quantity: Option<i64>,
    pub sell_price: Option<i64>,
    /// Resolved static namespace, e.g. `static-1.60.1_70399-forever-us`.
    pub static_ns: String,
    pub tooltip: Option<ItemTooltip>,
}

/// Build number in `static-1.60.1_70399-forever-us` (70399); 0 when absent.
fn ns_build(ns: &str) -> i64 {
    ns.split('_').nth(1).and_then(|s| s.split('-').next()).and_then(|s| s.parse().ok()).unwrap_or(0)
}

fn namespace_from_href(href: &str) -> Option<String> {
    href.split(['?', '&']).find_map(|kv| kv.strip_prefix("namespace=")).map(str::to_string)
}

/// Item ids in each feed's latest snapshot without metadata, plus items fetched under an older static namespace.
pub fn missing_item_ids(conn: &Connection, limit: usize) -> rusqlite::Result<Vec<i64>> {
    let current = get_kv(conn, "static_ns");
    let mut st = conn.prepare(
        "SELECT item_id FROM (
            SELECT DISTINCT item_id FROM item_aggregates
            WHERE snapshot_id IN (SELECT MAX(id) FROM feed_snapshots GROUP BY house, feed)
              AND item_id NOT IN (SELECT item_id FROM items)
            UNION SELECT item_id FROM items WHERE ?1 IS NOT NULL AND static_ns != ?1 AND static_ns != 'missing' AND static_ns NOT LIKE 'fallback:%'
         ) ORDER BY item_id LIMIT ?2",
    )?;
    let rows = st.query_map(params![current, limit as i64], |r| r.get(0))?;
    rows.collect()
}

enum Fetched {
    Found(Box<ItemMeta>),
    NotFound,
    Failed,
}

/// The item endpoint response, from Forever or else Classic Era (`true` = fallback). `Ok(None)` when neither has it.
async fn fetch_item_json(client: &BlizzardClient, id: i64) -> Result<Option<(serde_json::Value, bool)>, FetchError> {
    for (ns, is_fallback) in [(NAMESPACE_STATIC, false), (NAMESPACE_FALLBACK, true)] {
        match client.get_json(&format!("/data/wow/item/{id}?namespace={ns}&locale=en_US")).await {
            Ok(v) => return Ok(Some((v, is_fallback))),
            Err(FetchError::Status(404)) => continue,
            Err(e) => return Err(e),
        }
    }
    Ok(None)
}

async fn fetch_one(client: &BlizzardClient, id: i64) -> Fetched {
    let (item, fallback) = match fetch_item_json(client, id).await {
        Ok(Some((v, fb))) => (Some(v), fb),
        Ok(None) => (None, false),
        Err(_) => return Fetched::Failed,
    };
    let icon_url = match client.get_json(&format!("/data/wow/media/item/{id}?namespace={NAMESPACE_STATIC}&locale=en_US")).await {
        Ok(media) => media["assets"]
            .as_array()
            .and_then(|a| a.iter().find(|x| x["key"] == "icon"))
            .and_then(|x| x["value"].as_str())
            .map(str::to_string),
        Err(_) => None,
    };
    let Some(item) = item else {
        return if icon_url.is_some() { Fetched::Found(Box::new(placeholder(id, icon_url))) } else { Fetched::NotFound };
    };
    let Some(name) = item["name"].as_str().map(str::to_string) else { return Fetched::Failed };
    let resolved = item["_links"]["self"]["href"].as_str().and_then(namespace_from_href);
    let static_ns = match (fallback, resolved) {
        (false, Some(ns)) => ns,
        (false, None) => NAMESPACE_STATIC.to_string(),
        (true, ns) => format!("{FALLBACK_PREFIX}{}", ns.unwrap_or_else(|| NAMESPACE_FALLBACK.to_string())),
    };
    let s = |v: &serde_json::Value| v.as_str().map(str::to_string);
    Fetched::Found(Box::new(ItemMeta {
        item_id: id,
        name,
        quality: s(&item["quality"]["name"]),
        item_class: s(&item["item_class"]["name"]),
        item_subclass: s(&item["item_subclass"]["name"]),
        level: item["level"].as_i64(),
        icon_url,
        purchase_price: item["purchase_price"].as_i64(),
        purchase_quantity: item["purchase_quantity"].as_i64(),
        sell_price: item["sell_price"].as_i64(),
        static_ns,
        tooltip: parse_tooltip(&item),
    }))
}

fn placeholder(id: i64, icon_url: Option<String>) -> ItemMeta {
    ItemMeta {
        item_id: id,
        name: format!("Item #{id}"),
        quality: None,
        item_class: None,
        item_subclass: None,
        level: None,
        icon_url,
        purchase_price: None,
        purchase_quantity: None,
        sell_price: None,
        static_ns: NS_MISSING.to_string(),
        tooltip: None,
    }
}

/// Locally cached tooltip: `None` = never fetched, `Some(None)` = item has no tooltip data.
pub fn cached_tooltip(conn: &Connection, item_id: i64) -> rusqlite::Result<Option<Option<ItemTooltip>>> {
    let row: Option<Option<String>> = conn.query_row("SELECT tooltip_json FROM item_tooltips WHERE item_id = ?1", [item_id], |r| r.get(0)).optional()?;
    // An unreadable row (older format) counts as never fetched.
    Ok(row.and_then(|json| match json {
        None => Some(None),
        Some(j) => serde_json::from_str(&j).ok().map(Some),
    }))
}

/// Caches a tooltip (or its absence) for an item.
pub fn store_tooltip(conn: &Connection, item_id: i64, tooltip: Option<&ItemTooltip>, now: i64) -> rusqlite::Result<()> {
    let json = tooltip.map(|t| serde_json::to_string(t).expect("tooltip serializes"));
    conn.prepare_cached(
        "INSERT INTO item_tooltips(item_id, tooltip_json, fetched_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(item_id) DO UPDATE SET tooltip_json = excluded.tooltip_json, fetched_at = excluded.fetched_at",
    )?
    .execute(params![item_id, json, now])?;
    Ok(())
}

/// Fetches one item's tooltip (Forever, then Classic Era). `Ok(None)` when neither has the item.
pub async fn fetch_tooltip(client: &BlizzardClient, item_id: i64) -> Result<Option<ItemTooltip>, FetchError> {
    Ok(fetch_item_json(client, item_id).await?.and_then(|(v, _)| parse_tooltip(&v)))
}

/// Fetches item + media metadata, 10 at a time. Forever 404s fall back to Classic Era; items found in
/// neither get a placeholder row (not refetched); network/other failures are skipped and retried next cycle.
pub async fn fetch_items(client: &BlizzardClient, ids: &[i64]) -> Vec<ItemMeta> {
    stream::iter(ids.iter().copied())
        .map(|id| async move {
            match fetch_one(client, id).await {
                Fetched::Found(m) => Some(*m),
                Fetched::NotFound => Some(placeholder(id, None)),
                Fetched::Failed => None,
            }
        })
        .buffer_unordered(CONCURRENCY)
        .filter_map(|x| async move { x })
        .collect()
        .await
}

/// Upserts metadata and advances the `static_ns` setting to the newest namespace seen.
pub fn store_items(conn: &mut Connection, items: &[ItemMeta], now: i64) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    {
        let mut st = tx.prepare_cached(
            "INSERT INTO items(item_id, name, quality, item_class, item_subclass, level, icon_url, purchase_price, purchase_quantity, sell_price, static_ns, fetched_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT(item_id) DO UPDATE SET name = excluded.name, quality = excluded.quality, item_class = excluded.item_class,
             item_subclass = excluded.item_subclass, level = excluded.level, icon_url = excluded.icon_url, purchase_price = excluded.purchase_price,
             purchase_quantity = excluded.purchase_quantity, sell_price = excluded.sell_price, static_ns = excluded.static_ns, fetched_at = excluded.fetched_at",
        )?;
        for m in items {
            st.execute(params![m.item_id, m.name, m.quality, m.item_class, m.item_subclass, m.level, m.icon_url, m.purchase_price, m.purchase_quantity, m.sell_price, m.static_ns, now])?;
            store_tooltip(&tx, m.item_id, m.tooltip.as_ref(), now)?;
        }
    }
    let forever_ns = items.iter().map(|m| m.static_ns.as_str()).filter(|ns| ns.starts_with("static-") && ns.ends_with("-forever-us"));
    if let Some(newest) = forever_ns.max_by_key(|ns| ns_build(ns)) {
        let current = get_kv(&tx, "static_ns");
        if current.as_deref().is_none_or(|c| ns_build(newest) > ns_build(c)) {
            set_kv(&tx, "static_ns", newest)?;
        }
    }
    tx.commit()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blizzard::client::BlizzardConfig;
    use crate::db::open_in_memory;
    use httpmock::prelude::*;

    fn meta(id: i64, ns: &str) -> ItemMeta {
        ItemMeta { item_id: id, name: format!("Item {id}"), quality: None, item_class: None, item_subclass: None, level: None, icon_url: None, purchase_price: None, purchase_quantity: None, sell_price: None, static_ns: ns.into(), tooltip: None }
    }

    fn mock_token(server: &MockServer) -> BlizzardClient {
        server.mock(|when, then| { when.method(POST).path("/token"); then.status(200).json_body(serde_json::json!({"access_token":"t","expires_in":3600})); });
        BlizzardClient::new(BlizzardConfig { api_base: server.base_url(), token_url: server.url("/token"), client_id: "a".into(), client_secret: "b".into() })
    }

    #[test]
    fn tooltip_cache_distinguishes_unknown_missing_and_stored() {
        let c = open_in_memory();
        let t = crate::blizzard::tooltip::parse_tooltip(&serde_json::from_str(include_str!("../../tests/fixtures/item_13444.json")).unwrap()).unwrap();
        assert_eq!(cached_tooltip(&c, 13444).unwrap(), None);
        store_tooltip(&c, 13444, Some(&t), 5).unwrap();
        store_tooltip(&c, 31337, None, 5).unwrap();
        assert_eq!(cached_tooltip(&c, 13444).unwrap(), Some(Some(t)));
        assert_eq!(cached_tooltip(&c, 31337).unwrap(), Some(None));
    }

    #[test]
    fn store_items_caches_tooltips_including_missing() {
        let mut c = open_in_memory();
        let t = crate::blizzard::tooltip::parse_tooltip(&serde_json::from_str(include_str!("../../tests/fixtures/item_4500.json")).unwrap()).unwrap();
        let mut bag = meta(4500, "static-1.60.1_70399-forever-us");
        bag.tooltip = Some(t.clone());
        store_items(&mut c, &[bag, placeholder(31337, None)], 0).unwrap();
        assert_eq!(cached_tooltip(&c, 4500).unwrap(), Some(Some(t)));
        assert_eq!(cached_tooltip(&c, 31337).unwrap(), Some(None), "missing items are not fetched again on hover");
    }

    #[tokio::test]
    async fn fetch_tooltip_falls_back_reports_missing_and_errors() {
        let server = MockServer::start_async().await;
        let client = mock_token(&server);
        server.mock_async(|when, then| { when.method(GET).path("/data/wow/item/16922").query_param("namespace", "static-forever-us"); then.status(404); }).await;
        server.mock_async(|when, then| { when.method(GET).path("/data/wow/item/16922").query_param("namespace", "static-classic1x-us"); then.status(200).body(include_str!("../../tests/fixtures/item_16922.json")); }).await;
        server.mock_async(|when, then| { when.method(GET).path("/data/wow/item/31337"); then.status(404); }).await;
        server.mock_async(|when, then| { when.method(GET).path("/data/wow/item/999"); then.status(500); }).await;
        let t = fetch_tooltip(&client, 16922).await.unwrap().expect("fallback tooltip");
        assert!(t.lines.iter().any(|l| l.left == "Classes: Priest"));
        assert_eq!(fetch_tooltip(&client, 31337).await.unwrap(), None);
        assert!(fetch_tooltip(&client, 999).await.is_err(), "server errors are not cached as missing");
    }

    #[tokio::test]
    async fn fetch_item_reads_fields_and_namespace() {
        let server = MockServer::start_async().await;
        server.mock_async(|when, then| { when.method(POST).path("/token"); then.status(200).json_body(serde_json::json!({"access_token":"t","expires_in":3600})); }).await;
        server.mock_async(|when, then| { when.method(GET).path("/data/wow/item/2589").query_param("namespace", "static-forever-us"); then.status(200).body(include_str!("../../tests/fixtures/item_2589.json")); }).await;
        server.mock_async(|when, then| { when.method(GET).path("/data/wow/media/item/2589"); then.status(200).body(include_str!("../../tests/fixtures/media_2589.json")); }).await;
        server.mock_async(|when, then| { when.method(GET).path("/data/wow/item/999"); then.status(500); }).await;
        let client = BlizzardClient::new(BlizzardConfig { api_base: server.base_url(), token_url: server.url("/token"), client_id: "a".into(), client_secret: "b".into() });
        let items = fetch_items(&client, &[2589, 999]).await;
        assert_eq!(items.len(), 1, "server errors are skipped (retried next cycle)");
        let m = &items[0];
        assert_eq!(m.name, "Linen Cloth");
        assert_eq!(m.quality.as_deref(), Some("Common"));
        assert_eq!(m.item_class.as_deref(), Some("Trade Goods"));
        assert_eq!(m.item_subclass.as_deref(), Some("Cloth"));
        assert_eq!((m.purchase_price, m.purchase_quantity, m.sell_price), (Some(55), Some(1), Some(13)));
        assert_eq!(m.static_ns, "static-1.60.1_70399-forever-us");
        assert_eq!(m.icon_url.as_deref(), Some("https://render.worldofwarcraft.com/forever-us/icons/56/132889.jpg"));
        assert_eq!(m.tooltip.as_ref().map(|t| t.name.as_str()), Some("Linen Cloth"), "backfill keeps the tooltip so hover needs no fetch");
    }

    #[tokio::test]
    async fn fetch_item_falls_back_to_classic_namespace() {
        let server = MockServer::start_async().await;
        server.mock_async(|when, then| { when.method(POST).path("/token"); then.status(200).json_body(serde_json::json!({"access_token":"t","expires_in":3600})); }).await;
        server.mock_async(|when, then| { when.method(GET).path("/data/wow/item/727").query_param("namespace", "static-forever-us"); then.status(404); }).await;
        server.mock_async(|when, then| { when.method(GET).path("/data/wow/item/727").query_param("namespace", "static-classic1x-us");
            then.status(200).json_body(serde_json::json!({"id":727,"name":"Notched Shortsword","quality":{"name":"Uncommon"},"item_class":{"name":"Weapon"},"_links":{"self":{"href":"https://x/data/wow/item/727?namespace=static-1.15.8_1-classic1x-us"}}})); }).await;
        server.mock_async(|when, then| { when.method(GET).path("/data/wow/media/item/727"); then.status(200).json_body(serde_json::json!({"assets":[{"key":"icon","value":"https://icon/727.jpg"}]})); }).await;
        server.mock_async(|when, then| { when.method(GET).path("/data/wow/item/31337"); then.status(404); }).await;
        server.mock_async(|when, then| { when.method(GET).path("/data/wow/media/item/31337"); then.status(404); }).await;
        let client = BlizzardClient::new(BlizzardConfig { api_base: server.base_url(), token_url: server.url("/token"), client_id: "a".into(), client_secret: "b".into() });
        let mut items = fetch_items(&client, &[727, 31337]).await;
        items.sort_by_key(|m| m.item_id);
        assert_eq!(items[0].name, "Notched Shortsword");
        assert_eq!(items[0].static_ns, "fallback:static-1.15.8_1-classic1x-us");
        assert_eq!(items[0].icon_url.as_deref(), Some("https://icon/727.jpg"));
        assert_eq!((items[1].item_id, items[1].name.as_str(), items[1].static_ns.as_str()), (31337, "Item #31337", NS_MISSING));
    }

    #[test]
    fn fallback_and_missing_items_are_not_refetched_or_advance_namespace() {
        let mut c = open_in_memory();
        store_items(&mut c, &[meta(1, "static-1.60.1_70399-forever-us"), meta(2, "fallback:static-1.15.8_99999-classic1x-us"), meta(3, NS_MISSING)], 0).unwrap();
        assert_eq!(crate::settings::get_kv(&c, "static_ns").as_deref(), Some("static-1.60.1_70399-forever-us"));
        assert!(missing_item_ids(&c, 100).unwrap().is_empty());
    }

    #[test]
    fn missing_ids_lists_unknown_items() {
        let mut c = open_in_memory();
        c.execute("INSERT INTO feed_snapshots(id, house, feed, last_modified, fetched_at, listing_count) VALUES (1, 'alliance', 'items', 1, 1, 2)", []).unwrap();
        c.execute("INSERT INTO item_aggregates(snapshot_id, item_id, variant_key, quantity, listings) VALUES (1, 2589, '', 1, 1), (1, 727, '', 1, 1), (1, 727, '12718', 1, 1)", []).unwrap();
        store_items(&mut c, &[meta(2589, "static-1.60.1_70399-forever-us")], 0).unwrap();
        assert_eq!(missing_item_ids(&c, 100).unwrap(), vec![727]);
    }

    #[test]
    fn namespace_change_marks_items_for_refetch() {
        let mut c = open_in_memory();
        store_items(&mut c, &[meta(1, "static-1.60.1_70399-forever-us")], 0).unwrap();
        store_items(&mut c, &[meta(2, "static-1.60.2_70500-forever-us")], 0).unwrap();
        assert_eq!(crate::settings::get_kv(&c, "static_ns").as_deref(), Some("static-1.60.2_70500-forever-us"));
        assert_eq!(missing_item_ids(&c, 100).unwrap(), vec![1]);
        // an older namespace seen later does not move the setting back
        store_items(&mut c, &[meta(3, "static-1.60.1_70399-forever-us")], 0).unwrap();
        assert_eq!(crate::settings::get_kv(&c, "static_ns").as_deref(), Some("static-1.60.2_70500-forever-us"));
    }

    #[test]
    fn missing_ids_only_considers_latest_snapshots() {
        let c = open_in_memory();
        c.execute("INSERT INTO feed_snapshots(id, house, feed, last_modified, fetched_at, listing_count) VALUES (1, 'alliance', 'items', 1, 1, 1), (2, 'alliance', 'items', 2, 2, 1)", []).unwrap();
        c.execute("INSERT INTO item_aggregates(snapshot_id, item_id, variant_key, quantity, listings) VALUES (1, 9, '', 1, 1), (2, 10, '', 1, 1)", []).unwrap();
        assert_eq!(missing_item_ids(&c, 100).unwrap(), vec![10]);
    }

    #[test]
    fn missing_ids_respects_limit() {
        let mut c = open_in_memory();
        c.execute("INSERT INTO feed_snapshots(id, house, feed, last_modified, fetched_at, listing_count) VALUES (1, 'alliance', 'items', 1, 1, 3)", []).unwrap();
        c.execute("INSERT INTO item_aggregates(snapshot_id, item_id, variant_key, quantity, listings) VALUES (1, 1, '', 1, 1), (1, 2, '', 1, 1), (1, 3, '', 1, 1)", []).unwrap();
        assert_eq!(missing_item_ids(&c, 2).unwrap().len(), 2);
        let _ = &mut c;
    }
}
