pub mod common;
pub mod health;
pub mod history;
pub mod market;

#[cfg(test)]
pub(crate) mod testutil {
    use crate::ahledger::parse::{AhlRow, PriceTable};
    use crate::domain::{Feed, House, Listing, TimeLeft};
    use crate::processing::ingest::{ingest_feed, ingest_pricetable};
    use rusqlite::Connection;

    pub const NOW: i64 = 1794614400 + 10 * 86400;
    pub const H: i64 = 3600;

    pub fn li(auction_id: i64, item_id: i64, variant: &str, price: i64, qty: i64, t: TimeLeft) -> Listing {
        Listing { auction_id, item_id, variant_key: variant.into(), unit_price: Some(price), quantity: qty, time_left: t }
    }

    pub fn seed(c: &mut Connection, house: House, feed: Feed, t: i64, ls: &[Listing]) {
        ingest_feed(c, house, feed, t, t, ls, H).unwrap();
    }

    pub fn ahl(c: &mut Connection, market: &str, as_of: i64, rows: &[(i64, i64, Option<i64>)]) {
        let table = PriceTable {
            market: market.into(),
            as_of,
            rows: rows
                .iter()
                .map(|&(item_id, min, sales)| AhlRow { item_id, median: Some(min), min_buyout: Some(min), quantity: Some(1), median_7d: None, median_30d: None, low_30d: None, high_30d: None, sales_per_day_x10: sales })
                .collect(),
        };
        ingest_pricetable(c, &table).unwrap();
    }

    pub fn name_item(c: &Connection, id: i64, name: &str) {
        c.execute("INSERT INTO items(item_id, name, static_ns, fetched_at) VALUES (?1, ?2, 'ns', 0)", rusqlite::params![id, name]).unwrap();
    }
}
