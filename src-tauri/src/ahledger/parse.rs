/// One AHledger price table (`GET /v1/pricetable/{market}`), prices in copper.
#[derive(Clone, Debug, PartialEq)]
pub struct PriceTable {
    /// Dotted market id, e.g. `forever.pvp.alliance.us`.
    pub market: String,
    /// Unix seconds of the newest scan behind the table.
    pub as_of: i64,
    pub rows: Vec<AhlRow>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AhlRow {
    pub item_id: i64,
    pub median: Option<i64>,
    pub min_buyout: Option<i64>,
    pub quantity: Option<i64>,
    pub median_7d: Option<i64>,
    pub median_30d: Option<i64>,
    pub low_30d: Option<i64>,
    pub high_30d: Option<i64>,
    /// Estimated units sold per day x 10.
    pub sales_per_day_x10: Option<i64>,
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("bad price table header: {0:?}")]
    Header(String),
    #[error("bad price table row {line}: {text:?}")]
    Row { line: usize, text: String },
}

fn opt(field: Option<&str>) -> Result<Option<i64>, ()> {
    match field.map(str::trim) {
        None | Some("") => Ok(None),
        Some(s) => s.parse().map(Some).map_err(|_| ()),
    }
}

/// Parses `AHL1|market/with/slashes|unixTime|rowCount` followed by
/// `itemId:median:minBuyout:quantity:median7d:median30d:low30d:high30d:salesPerDayX10` rows.
pub fn parse_pricetable(text: &str) -> Result<PriceTable, ParseError> {
    let mut lines = text.lines();
    let header = lines.next().unwrap_or_default();
    let parts: Vec<&str> = header.split('|').collect();
    if parts.len() < 3 || parts[0] != "AHL1" {
        return Err(ParseError::Header(header.to_string()));
    }
    let market = parts[1].replace('/', ".");
    let as_of = parts[2].trim().parse().map_err(|_| ParseError::Header(header.to_string()))?;
    let mut rows = Vec::new();
    for (i, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let bad = || ParseError::Row { line: i + 2, text: line.to_string() };
        let f: Vec<&str> = line.split(':').collect();
        let item_id = f[0].trim().parse().map_err(|_| bad())?;
        let g = |n: usize| opt(f.get(n).copied()).map_err(|_| bad());
        rows.push(AhlRow {
            item_id,
            median: g(1)?,
            min_buyout: g(2)?,
            quantity: g(3)?,
            median_7d: g(4)?,
            median_30d: g(5)?,
            low_30d: g(6)?,
            high_30d: g(7)?,
            sales_per_day_x10: g(8)?,
        });
    }
    Ok(PriceTable { market, as_of, rows })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_market_uses_dots() {
        let t = parse_pricetable("AHL1|forever/pvp/horde/us|1793000000|1\n2589:20:18:3420:22:21:18:25:3600\n").unwrap();
        assert_eq!(t.market, "forever.pvp.horde.us");
        assert_eq!(t.as_of, 1793000000);
        assert_eq!(t.rows[0].item_id, 2589);
        assert_eq!(t.rows[0].median, Some(20));
        assert_eq!(t.rows[0].min_buyout, Some(18));
        assert_eq!(t.rows[0].quantity, Some(3420));
        assert_eq!(t.rows[0].high_30d, Some(25));
        assert_eq!(t.rows[0].sales_per_day_x10, Some(3600));
    }

    #[test]
    fn empty_field_is_none() {
        let t = parse_pricetable("AHL1|forever/pvp/horde/us|1|1\n12784:1500000::2:::::\n").unwrap();
        assert_eq!(t.rows[0].min_buyout, None);
        assert_eq!(t.rows[0].quantity, Some(2));
        assert_eq!(t.rows[0].sales_per_day_x10, None);
    }

    #[test]
    fn wrong_format_tag_is_error() {
        assert!(parse_pricetable("XYZ|a|1|0\n").is_err());
    }

    #[test]
    fn non_numeric_item_id_is_error() {
        assert!(parse_pricetable("AHL1|forever/pvp/horde/us|1|1\nabc:1:1:1:1:1:1:1:1\n").is_err());
    }

    #[test]
    fn fixture_has_300_rows() {
        let t = parse_pricetable(include_str!("../../tests/fixtures/pricetable_pvp_alliance.txt")).unwrap();
        assert_eq!(t.rows.len(), 300);
        assert_eq!(t.market, "forever.pvp.alliance.us");
    }
}
