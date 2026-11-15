//! In-game style item tooltip built from the item endpoint's `preview_item`.
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Line colours, mirroring the game's tooltip palette.
pub mod tone {
    pub const NORMAL: &str = "normal";
    pub const GREEN: &str = "green";
    pub const YELLOW: &str = "yellow";
    pub const BLUE: &str = "blue";
    pub const MUTED: &str = "muted";
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TooltipLine {
    /// Empty text is a spacer.
    pub left: String,
    /// Right-aligned text (e.g. weapon type, speed).
    pub right: Option<String>,
    /// One of [`tone`], or `q-<quality>` (e.g. `q-rare`) for item names.
    pub tone: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemTooltip {
    pub name: String,
    pub quality: Option<String>,
    /// Lines under the name, in game order.
    pub lines: Vec<TooltipLine>,
    /// Vendor sell price in copper.
    pub sell_price: Option<i64>,
}

/// Builds the tooltip from an item endpoint response (`None` without `preview_item`).
pub fn parse_tooltip(item: &Value) -> Option<ItemTooltip> {
    let p = item.get("preview_item").filter(|v| v.is_object())?;
    let mut out = Lines::default();

    out.push(text(&p["binding"]["name"]), None, tone::NORMAL);
    out.push(text(&p["unique_equipped"]), None, tone::NORMAL);
    out.push(text(&p["crafting_reagent"]), None, tone::BLUE);

    if p["inventory_type"]["type"].as_str().is_some_and(|t| t != "NON_EQUIP") {
        let slot = text(&p["inventory_type"]["name"]);
        let sub = text(&p["item_subclass"]["name"]).filter(|s| p["is_subclass_hidden"] != true && Some(s) != slot.as_ref());
        out.push(slot, sub, tone::NORMAL);
    }

    let w = &p["weapon"];
    out.push(text(&w["damage"]["display_string"]), text(&w["attack_speed"]["display_string"]), tone::NORMAL);
    out.push(text(&w["dps"]["display_string"]), None, tone::NORMAL);
    out.push(text(&p["armor"]["display"]["display_string"]), None, tone::NORMAL);

    for s in p["stats"].as_array().into_iter().flatten() {
        let c = &s["display"]["color"];
        let green = c["r"] == 0 && c["g"] == 255;
        out.push(text(&s["display"]["display_string"]), None, if green { tone::GREEN } else { tone::NORMAL });
    }

    out.push(text(&p["container_slots"]["display_string"]), None, tone::NORMAL);
    out.push(text(&p["durability"]["display_string"]), None, tone::NORMAL);

    if let Some(req) = p["requirements"].as_object() {
        const ORDER: [&str; 5] = ["playable_classes", "playable_races", "skill", "reputation", "level"];
        let rest = req.keys().filter(|k| !ORDER.contains(&k.as_str())).map(String::as_str);
        for k in ORDER.into_iter().chain(rest) {
            out.push(req.get(k).and_then(|r| text(&r["display_string"])), None, tone::NORMAL);
        }
    }
    out.push(text(&p["level"]["display_string"]), None, tone::MUTED);

    for s in p["spells"].as_array().into_iter().flatten() {
        out.push(text(&s["description"]), None, tone::GREEN);
    }

    let recipe = &p["recipe"];
    if let Some(d) = text(&p["description"]) {
        if recipe.is_object() {
            out.push(Some(format!("Use: {d}")), None, tone::GREEN);
        } else {
            out.push(Some(format!("\"{d}\"")), None, tone::YELLOW);
        }
    }

    let set = &p["set"];
    if let Some(name) = text(&set["display_string"]) {
        out.spacer();
        out.push(Some(name), None, tone::YELLOW);
        for i in set["items"].as_array().into_iter().flatten() {
            out.push(text(&i["item"]["name"]).map(|n| format!("  {n}")), None, tone::MUTED);
        }
        out.spacer();
        for e in set["effects"].as_array().into_iter().flatten() {
            out.push(text(&e["display_string"]), None, tone::MUTED);
        }
    }

    if let Some(made) = text(&recipe["item"]["name"]) {
        out.spacer();
        let q = text(&recipe["item"]["quality"]["name"]).map_or_else(|| tone::NORMAL.to_string(), |q| format!("q-{}", q.to_lowercase()));
        out.lines.push(TooltipLine { left: made, right: None, tone: q });
        out.push(text(&recipe["reagents_display_string"]).map(|r| format!("Requires {r}")), None, tone::NORMAL);
    }

    Some(ItemTooltip {
        name: text(&p["name"]).or_else(|| text(&item["name"])).unwrap_or_default(),
        quality: text(&p["quality"]["name"]),
        lines: out.lines,
        sell_price: p["sell_price"]["value"].as_i64(),
    })
}

fn text(v: &Value) -> Option<String> {
    v.as_str().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string)
}

#[derive(Default)]
struct Lines {
    lines: Vec<TooltipLine>,
}

impl Lines {
    /// Adds a line when `left` is present.
    fn push(&mut self, left: Option<String>, right: Option<String>, tone: &str) {
        if let Some(left) = left {
            self.lines.push(TooltipLine { left, right, tone: tone.into() });
        }
    }

    fn spacer(&mut self) {
        if self.lines.last().is_some_and(|l| !l.left.is_empty()) {
            self.lines.push(TooltipLine { left: String::new(), right: None, tone: tone::NORMAL.into() });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tip(fixture: &str) -> ItemTooltip {
        parse_tooltip(&serde_json::from_str(fixture).unwrap()).expect("tooltip")
    }

    fn line<'a>(t: &'a ItemTooltip, left: &str) -> &'a TooltipLine {
        t.lines.iter().find(|l| l.left == left).unwrap_or_else(|| panic!("no line {left:?} in {:#?}", t.lines))
    }

    fn lefts(t: &ItemTooltip) -> Vec<&str> {
        t.lines.iter().map(|l| l.left.as_str()).collect()
    }

    #[test]
    fn weapon_lines_in_game_order() {
        let t = tip(include_str!("../../tests/fixtures/item_19019.json"));
        assert_eq!(t.name, "Thunderfury, Blessed Blade of the Windseeker");
        assert_eq!(t.quality.as_deref(), Some("Legendary"));
        assert_eq!(
            lefts(&t),
            vec![
                "Binds when picked up",
                "Unique",
                "One-Hand",
                "44 - 115 Damage",
                "(41.8 damage per second)",
                "+5 Agility",
                "+8 Stamina",
                "+8 Fire Resistance",
                "+9 Nature Resistance",
                "Durability 125 / 125",
                "Requires Level 60",
                "Item Level 80",
                "Chance on hit: Blasts your enemy with lightning, dealing 300 Nature damage and then jumping to additional nearby enemies.  Each jump reduces that victim's Nature resistance by 25. Affects 5 targets. Your primary target is also consumed by a cyclone, slowing its attack speed by 20% for 12 sec.",
            ]
        );
        assert_eq!(line(&t, "One-Hand").right.as_deref(), Some("Sword"));
        assert_eq!(line(&t, "44 - 115 Damage").right.as_deref(), Some("Speed 1.90"));
        assert_eq!(line(&t, "+5 Agility").tone, tone::NORMAL);
        assert_eq!(line(&t, "+8 Fire Resistance").tone, tone::GREEN, "stat colour comes from the API");
        assert_eq!(line(&t, "Item Level 80").tone, tone::MUTED);
        assert_eq!(t.lines.last().unwrap().tone, tone::GREEN);
        assert_eq!(t.sell_price, None);
    }

    #[test]
    fn armor_set_and_class_requirement() {
        let t = tip(include_str!("../../tests/fixtures/item_16922.json"));
        assert!(lefts(&t).contains(&"Classes: Priest"));
        assert_eq!(line(&t, "Vestments of Transcendence (0/8)").tone, tone::YELLOW);
        assert_eq!(line(&t, "  Belt of Transcendence").tone, tone::MUTED);
        let bonus = line(&t, "(3) Set: Allows 15% of your Mana regeneration to continue while casting.");
        assert_eq!(bonus.tone, tone::MUTED);
        let classes = lefts(&t).iter().position(|l| *l == "Classes: Priest").unwrap();
        let level = lefts(&t).iter().position(|l| *l == "Requires Level 60").unwrap();
        assert!(classes < level, "classes come before the level requirement");
        assert!(t.sell_price.is_some_and(|p| p > 0));
    }

    #[test]
    fn recipe_shows_use_effect_crafted_item_and_reagents() {
        let t = tip(include_str!("../../tests/fixtures/item_13477.json"));
        assert_eq!(line(&t, "Use: Teaches you how to make a Superior Mana Potion.").tone, tone::GREEN);
        assert_eq!(line(&t, "Requires Alchemy (260)").tone, tone::NORMAL);
        assert_eq!(line(&t, "Superior Mana Potion").tone, "q-common");
        assert!(lefts(&t).contains(&"Requires Sungrass (2), Blindweed (2), Crystal Vial"));
        assert!(lefts(&t).contains(&""), "a spacer separates the crafted item");
    }

    #[test]
    fn non_equippable_has_no_slot_line_and_hidden_subclass() {
        let t = tip(include_str!("../../tests/fixtures/item_13444.json"));
        assert_eq!(lefts(&t), vec!["Requires Level 49", "Use: Restores 1,350 to 2,250 mana. (2 Min Cooldown)"]);
        assert_eq!(t.sell_price, Some(1500));
    }

    #[test]
    fn bag_slots_and_no_duplicate_subclass() {
        let t = tip(include_str!("../../tests/fixtures/item_4500.json"));
        assert_eq!(line(&t, "Bag").right, None);
        assert!(lefts(&t).contains(&"16 Slot Bag"));
    }

    #[test]
    fn flavor_text_is_quoted_yellow() {
        let v = serde_json::json!({"preview_item": {"name": "Toy", "quality": {"name": "Common"}, "description": "It squeaks."}});
        let t = parse_tooltip(&v).unwrap();
        assert_eq!((t.lines[0].left.as_str(), t.lines[0].tone.as_str()), ("\"It squeaks.\"", tone::YELLOW));
    }

    #[test]
    fn missing_preview_is_none() {
        assert_eq!(parse_tooltip(&serde_json::json!({"name": "x"})), None);
    }
}
