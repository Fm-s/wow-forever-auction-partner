use crate::blizzard::client::BlizzardClient;
use crate::settings::{get_kv, set_kv};
use futures::stream::{self, StreamExt};
use rusqlite::{params, Connection};
use serde::Serialize;

pub const NAMESPACE_STATIC: &str = "static-forever-us";
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
}

/// Build number in `static-1.60.1_70399-forever-us` (70399); 0 when absent.
fn ns_build(ns: &str) -> i64 {
    ns.split('_').nth(1).and_then(|s| s.split('-').next()).and_then(|s| s.parse().ok()).unwrap_or(0)
}

fn namespace_from_href(href: &str) -> Option<String> {
    href.split(['?', '&']).find_map(|kv| kv.strip_prefix("namespace=")).map(str::to_string)
}

/// Item ids seen in aggregates without metadata, plus items fetched under an older static namespace.
pub fn missing_item_ids(conn: &Connection, limit: usize) -> rusqlite::Result<Vec<i64>> {
    let current = get_kv(conn, "static_ns");
    let mut st = conn.prepare(
        "SELECT item_id FROM (
            SELECT DISTINCT item_id FROM item_aggregates WHERE item_id NOT IN (SELECT item_id FROM items)
            UNION SELECT item_id FROM items WHERE ?1 IS NOT NULL AND static_ns != ?1
         ) ORDER BY item_id LIMIT ?2",
    )?;
    let rows = st.query_map(params![current, limit as i64], |r| r.get(0))?;
    rows.collect()
}

async fn fetch_one(client: &BlizzardClient, id: i64) -> Option<ItemMeta> {
    let item = client.get_json(&format!("/data/wow/item/{id}?namespace={NAMESPACE_STATIC}&locale=en_US")).await.ok()?;
    let name = item["name"].as_str()?.to_string();
    let static_ns = item["_links"]["self"]["href"].as_str().and_then(namespace_from_href).unwrap_or_else(|| NAMESPACE_STATIC.to_string());
    let icon_url = match client.get_json(&format!("/data/wow/media/item/{id}?namespace={NAMESPACE_STATIC}&locale=en_US")).await {
        Ok(media) => media["assets"]
            .as_array()
            .and_then(|a| a.iter().find(|x| x["key"] == "icon"))
            .and_then(|x| x["value"].as_str())
            .map(str::to_string),
        Err(_) => None,
    };
    let s = |v: &serde_json::Value| v.as_str().map(str::to_string);
    Some(ItemMeta {
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
    })
}

/// Fetches item + media metadata, 10 at a time; items that fail are skipped (retried next cycle).
pub async fn fetch_items(client: &BlizzardClient, ids: &[i64]) -> Vec<ItemMeta> {
    stream::iter(ids.iter().copied())
        .map(|id| fetch_one(client, id))
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
        }
    }
    if let Some(newest) = items.iter().map(|m| m.static_ns.as_str()).max_by_key(|ns| ns_build(ns)) {
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
        ItemMeta { item_id: id, name: format!("Item {id}"), quality: None, item_class: None, item_subclass: None, level: None, icon_url: None, purchase_price: None, purchase_quantity: None, sell_price: None, static_ns: ns.into() }
    }

    #[tokio::test]
    async fn fetch_item_reads_fields_and_namespace() {
        let server = MockServer::start_async().await;
        server.mock_async(|when, then| { when.method(POST).path("/token"); then.status(200).json_body(serde_json::json!({"access_token":"t","expires_in":3600})); }).await;
        server.mock_async(|when, then| { when.method(GET).path("/data/wow/item/2589").query_param("namespace", "static-forever-us"); then.status(200).body(include_str!("../../tests/fixtures/item_2589.json")); }).await;
        server.mock_async(|when, then| { when.method(GET).path("/data/wow/media/item/2589"); then.status(200).body(include_str!("../../tests/fixtures/media_2589.json")); }).await;
        server.mock_async(|when, then| { when.method(GET).path("/data/wow/item/999"); then.status(404); }).await;
        let client = BlizzardClient::new(BlizzardConfig { api_base: server.base_url(), token_url: server.url("/token"), client_id: "a".into(), client_secret: "b".into() });
        let items = fetch_items(&client, &[2589, 999]).await;
        assert_eq!(items.len(), 1, "per-item errors are skipped");
        let m = &items[0];
        assert_eq!(m.name, "Linen Cloth");
        assert_eq!(m.quality.as_deref(), Some("Common"));
        assert_eq!(m.item_class.as_deref(), Some("Trade Goods"));
        assert_eq!(m.item_subclass.as_deref(), Some("Cloth"));
        assert_eq!((m.purchase_price, m.purchase_quantity, m.sell_price), (Some(55), Some(1), Some(13)));
        assert_eq!(m.static_ns, "static-1.60.1_70399-forever-us");
        assert_eq!(m.icon_url.as_deref(), Some("https://render.worldofwarcraft.com/forever-us/icons/56/132889.jpg"));
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
    fn missing_ids_respects_limit() {
        let mut c = open_in_memory();
        c.execute("INSERT INTO feed_snapshots(id, house, feed, last_modified, fetched_at, listing_count) VALUES (1, 'alliance', 'items', 1, 1, 3)", []).unwrap();
        c.execute("INSERT INTO item_aggregates(snapshot_id, item_id, variant_key, quantity, listings) VALUES (1, 1, '', 1, 1), (1, 2, '', 1, 1), (1, 3, '', 1, 1)", []).unwrap();
        assert_eq!(missing_item_ids(&c, 2).unwrap().len(), 2);
        let _ = &mut c;
    }
}
