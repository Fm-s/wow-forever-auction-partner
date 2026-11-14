use serde::Serialize;
use std::collections::HashMap;

pub const KEYCHAIN_SERVICE: &str = "app.auctioneer.desktop";
const ACCOUNT_ID: &str = "blizzard_client_id";
const ACCOUNT_SECRET: &str = "blizzard_client_secret";
const ACCOUNT_AHLEDGER: &str = "ahledger_key";

#[derive(Clone, PartialEq)]
pub struct Credentials {
    pub blizzard_client_id: String,
    pub blizzard_client_secret: String,
    pub ahledger_key: Option<String>,
}

impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Credentials { .. }")
    }
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CredentialsStatus {
    pub blizzard_set: bool,
    pub ahledger_set: bool,
}

/// Parses KEY=VALUE lines; ignores blanks and `#` comments; strips matching quotes.
pub fn parse_env_file(text: &str) -> HashMap<String, String> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| {
            let v = v.trim();
            let v = v.strip_prefix('"').and_then(|x| x.strip_suffix('"')).or_else(|| v.strip_prefix('\'').and_then(|x| x.strip_suffix('\''))).unwrap_or(v);
            (k.trim().to_string(), v.to_string())
        })
        .collect()
}

fn non_empty(v: Option<&String>) -> Option<String> {
    v.filter(|s| !s.trim().is_empty()).cloned()
}

pub fn credentials_from_env(m: &HashMap<String, String>) -> Option<Credentials> {
    Some(Credentials {
        blizzard_client_id: non_empty(m.get("BLIZZARD_CLIENT_ID"))?,
        blizzard_client_secret: non_empty(m.get("BLIZZARD_CLIENT_SECRET"))?,
        ahledger_key: non_empty(m.get("AHLEDGER_KEY")),
    })
}

fn keychain_get(account: &str) -> Option<String> {
    keyring::Entry::new(KEYCHAIN_SERVICE, account).ok()?.get_password().ok().filter(|s| !s.is_empty())
}

fn dev_env_credentials() -> Option<Credentials> {
    if !cfg!(debug_assertions) {
        return None;
    }
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.env");
    credentials_from_env(&parse_env_file(&std::fs::read_to_string(path).ok()?))
}

/// Keychain first; debug builds fall back to the repo-root `.env`.
pub fn load_credentials() -> Option<Credentials> {
    match (keychain_get(ACCOUNT_ID), keychain_get(ACCOUNT_SECRET)) {
        (Some(id), Some(secret)) => Some(Credentials { blizzard_client_id: id, blizzard_client_secret: secret, ahledger_key: keychain_get(ACCOUNT_AHLEDGER) }),
        _ => dev_env_credentials(),
    }
}

/// Stores all three values; an empty/None AHledger key removes it.
pub fn save_credentials(c: &Credentials) -> Result<(), keyring::Error> {
    keyring::Entry::new(KEYCHAIN_SERVICE, ACCOUNT_ID)?.set_password(&c.blizzard_client_id)?;
    keyring::Entry::new(KEYCHAIN_SERVICE, ACCOUNT_SECRET)?.set_password(&c.blizzard_client_secret)?;
    let ahl = keyring::Entry::new(KEYCHAIN_SERVICE, ACCOUNT_AHLEDGER)?;
    match c.ahledger_key.as_deref().filter(|k| !k.trim().is_empty()) {
        Some(k) => ahl.set_password(k)?,
        None => match ahl.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => {}
            Err(e) => return Err(e),
        },
    }
    Ok(())
}

/// Which credentials are available; never the secrets themselves.
pub fn credentials_status() -> CredentialsStatus {
    let c = load_credentials();
    CredentialsStatus { blizzard_set: c.is_some(), ahledger_set: c.and_then(|c| c.ahledger_key).is_some() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_parsing() {
        let m = parse_env_file("BLIZZARD_CLIENT_ID=abc\n# c\nBLIZZARD_CLIENT_SECRET = \"s3\"\n\nAHLEDGER_KEY='k'\n");
        assert_eq!(m["BLIZZARD_CLIENT_ID"], "abc");
        assert_eq!(m["BLIZZARD_CLIENT_SECRET"], "s3");
        assert_eq!(m["AHLEDGER_KEY"], "k");
        assert_eq!(m.len(), 3);
    }

    #[test]
    fn credentials_from_env_map_requires_both_blizzard_values() {
        let m = parse_env_file("BLIZZARD_CLIENT_ID=abc\n");
        assert!(credentials_from_env(&m).is_none());
        let m = parse_env_file("BLIZZARD_CLIENT_ID=abc\nBLIZZARD_CLIENT_SECRET=s\nAHLEDGER_KEY=\n");
        let c = credentials_from_env(&m).unwrap();
        assert_eq!((c.blizzard_client_id.as_str(), c.blizzard_client_secret.as_str(), c.ahledger_key), ("abc", "s", None));
    }
}
