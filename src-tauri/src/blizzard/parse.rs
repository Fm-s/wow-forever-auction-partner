use crate::domain::{Listing, TimeLeft};
use serde::Deserialize;

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("invalid feed JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unknown time_left {0:?}")]
    TimeLeft(String),
}

#[derive(Deserialize)]
struct Feed<T> {
    #[serde(default = "Vec::new")]
    auctions: Vec<T>,
}

#[derive(Deserialize)]
struct ItemRef {
    id: i64,
    #[serde(default)]
    bonus_lists: Option<Vec<i64>>,
}

#[derive(Deserialize)]
struct ItemAuction {
    id: i64,
    item: ItemRef,
    buyout: Option<i64>,
    quantity: i64,
    time_left: String,
}

#[derive(Deserialize)]
struct CommodityAuction {
    id: i64,
    item: ItemRef,
    unit_price: i64,
    quantity: i64,
    time_left: String,
}

/// Bonus lists sorted ascending and joined with ':'; "" when there are none.
pub fn variant_key(bonus_lists: Option<&[i64]>) -> String {
    let mut v: Vec<i64> = bonus_lists.unwrap_or(&[]).to_vec();
    v.sort_unstable();
    v.iter().map(i64::to_string).collect::<Vec<_>>().join(":")
}

fn time_left(s: &str) -> Result<TimeLeft, ParseError> {
    TimeLeft::from_str(s).ok_or_else(|| ParseError::TimeLeft(s.to_string()))
}

/// Non-commodity feed: unit price = buyout / quantity (truncated); bid-only listings have no price.
pub fn parse_items_feed(body: &[u8]) -> Result<Vec<Listing>, ParseError> {
    let feed: Feed<ItemAuction> = serde_json::from_slice(body)?;
    feed.auctions
        .into_iter()
        .map(|a| {
            Ok(Listing {
                auction_id: a.id,
                item_id: a.item.id,
                variant_key: variant_key(a.item.bonus_lists.as_deref()),
                unit_price: a.buyout.filter(|_| a.quantity > 0).map(|b| b / a.quantity),
                quantity: a.quantity,
                time_left: time_left(&a.time_left)?,
            })
        })
        .collect()
}

/// Commodity feed: unit_price is given per unit; commodities have no variants.
pub fn parse_commodities_feed(body: &[u8]) -> Result<Vec<Listing>, ParseError> {
    let feed: Feed<CommodityAuction> = serde_json::from_slice(body)?;
    feed.auctions
        .into_iter()
        .map(|a| {
            Ok(Listing {
                auction_id: a.id,
                item_id: a.item.id,
                variant_key: String::new(),
                unit_price: Some(a.unit_price),
                quantity: a.quantity,
                time_left: time_left(&a.time_left)?,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::TimeLeft;
    use std::collections::HashSet;

    const FIX_ITEMS: &[u8] = include_bytes!("../../tests/fixtures/items_110.json");
    const FIX_COMM: &[u8] = include_bytes!("../../tests/fixtures/commodities_110.json");

    #[test]
    fn variant_key_sorts_and_joins() {
        assert_eq!(variant_key(Some(&[12761, 5, 900])), "5:900:12761");
        assert_eq!(variant_key(None), "");
        assert_eq!(variant_key(Some(&[])), "");
    }

    #[test]
    fn items_feed_unit_price_is_buyout_over_quantity() {
        let body = br#"{"auctions":[{"id":7,"item":{"id":5,"bonus_lists":[3,1]},"buyout":600,"quantity":3,"time_left":"LONG"}]}"#;
        let l = parse_items_feed(body).unwrap();
        assert_eq!(
            l,
            vec![Listing { auction_id: 7, item_id: 5, variant_key: "1:3".into(), unit_price: Some(200), quantity: 3, time_left: TimeLeft::Long }]
        );
    }

    #[test]
    fn items_feed_bid_only_has_no_price() {
        let body = br#"{"auctions":[{"id":1,"item":{"id":5},"bid":100,"quantity":1,"time_left":"SHORT"}]}"#;
        let l = parse_items_feed(body).unwrap();
        assert_eq!(l[0].unit_price, None);
        assert_eq!(l[0].quantity, 1);
    }

    #[test]
    fn items_fixture_parses_with_variants() {
        let l = parse_items_feed(FIX_ITEMS).unwrap();
        assert!(l.len() >= 300);
        let variants: HashSet<_> = l.iter().filter(|x| x.item_id == 727).map(|x| &x.variant_key).collect();
        assert!(variants.len() > 1);
    }

    #[test]
    fn commodities_fixture_uses_unit_price() {
        let l = parse_commodities_feed(FIX_COMM).unwrap();
        assert_eq!(l.len(), 300);
        assert!(l.iter().all(|x| x.unit_price.is_some() && x.variant_key.is_empty()));
    }

    #[test]
    fn unknown_time_left_is_error() {
        let body = br#"{"auctions":[{"id":1,"item":{"id":5},"buyout":1,"quantity":1,"time_left":"FOREVER"}]}"#;
        assert!(parse_items_feed(body).is_err());
    }

    #[test]
    fn empty_house_has_no_auctions_key() {
        // Empty houses (e.g. 112) return no "auctions" array.
        assert!(parse_items_feed(br#"{"id":112,"name":"Alliance Auction House"}"#).unwrap().is_empty());
    }
}
