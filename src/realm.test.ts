import { expect, test } from "vitest";
import { realmLabel, realmSwitchNote } from "./realm";

test("realm label", () => {
  expect(realmLabel({ region: "eu", realmType: "rppvp" })).toBe("Roleplay PvP · EU");
});

test("switch note only when the realm changes, and says the old history is kept", () => {
  expect(realmSwitchNote({ region: "us", realmType: "pvp" }, { region: "us", realmType: "pvp" })).toBeNull();
  expect(realmSwitchNote({ region: "us", realmType: "pvp" }, { region: "eu", realmType: "normal" })).toBe(
    "Collection switches to Normal · EU when you save. PvP · US history is kept; switch back anytime.",
  );
});
