use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtraField {
    pub key_name: String,
    pub value: String,
    pub encrypted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub id: u32,
    pub service_name: String,
    pub account: String,
    pub password: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub keyword: String,
    pub category: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub otp_uri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    pub status: i32,
    #[serde(default)]
    pub extra_fields: Vec<ExtraField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataStore {
    pub version: u32,
    pub entries: Vec<Entry>,
    #[serde(default)]
    pub categories: Vec<String>,
}

impl DataStore {
    pub fn new() -> Self {
        Self { version: 1, entries: vec![], categories: vec![] }
    }
}

/// インポート／エクスポート用のフラット形式（extra1_* ～ extra3_*）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlatEntry {
    pub id: u32,
    pub service_name: String,
    pub account: String,
    pub password: String,
    #[serde(default)]
    pub status: i32,
    #[serde(default)]
    pub keyword: String,
    #[serde(default)]
    pub category: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra1_key_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra1_value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra1_encrypted: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra2_key_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra2_value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra2_encrypted: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra3_key_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra3_value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra3_encrypted: Option<bool>,
}

impl From<FlatEntry> for Entry {
    fn from(f: FlatEntry) -> Self {
        let mut extra_fields = Vec::new();
        for (key, val, enc) in [
            (&f.extra1_key_name, &f.extra1_value, f.extra1_encrypted),
            (&f.extra2_key_name, &f.extra2_value, f.extra2_encrypted),
            (&f.extra3_key_name, &f.extra3_value, f.extra3_encrypted),
        ] {
            if let (Some(k), Some(v)) = (key, val) {
                if !k.is_empty() {
                    extra_fields.push(ExtraField {
                        key_name: k.clone(),
                        value: v.clone(),
                        encrypted: enc.unwrap_or(false),
                    });
                }
            }
        }
        Entry {
            id: f.id,
            service_name: f.service_name,
            account: f.account,
            password: f.password,
            url: None,
            keyword: f.keyword,
            category: f.category,
            otp_uri: None,
            notes: None,
            status: f.status,
            extra_fields,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_flat(id: u32) -> FlatEntry {
        FlatEntry {
            id,
            service_name: "AWS".to_string(),
            account: "alice".to_string(),
            password: "pass".to_string(),
            status: 1,
            keyword: "cloud".to_string(),
            category: "biz".to_string(),
            extra1_key_name: Some("email".to_string()),
            extra1_value: Some("alice@example.com".to_string()),
            extra1_encrypted: Some(false),
            extra2_key_name: Some("token".to_string()),
            extra2_value: Some("secret-token".to_string()),
            extra2_encrypted: Some(true),
            extra3_key_name: None,
            extra3_value: None,
            extra3_encrypted: None,
        }
    }

    #[test]
    fn flat_to_entry_converts_extra_fields() {
        let entry: Entry = sample_flat(1).into();
        assert_eq!(entry.id, 1);
        assert_eq!(entry.extra_fields.len(), 2);
        assert_eq!(entry.extra_fields[0].key_name, "email");
        assert!(!entry.extra_fields[0].encrypted);
        assert_eq!(entry.extra_fields[1].key_name, "token");
        assert!(entry.extra_fields[1].encrypted);
    }

    #[test]
    fn flat_to_entry_skips_empty_key_name() {
        let mut flat = sample_flat(1);
        flat.extra1_key_name = Some("".to_string());
        flat.extra1_value = Some("value".to_string());
        let entry: Entry = flat.into();
        // key_name が空のフィールドはスキップされる
        assert_eq!(entry.extra_fields.len(), 1);
        assert_eq!(entry.extra_fields[0].key_name, "token");
    }

    #[test]
    fn entry_to_flat_roundtrip() {
        let original = sample_flat(42);
        let entry: Entry = original.clone().into();
        let back: FlatEntry = entry.into();
        assert_eq!(back.id, 42);
        assert_eq!(back.extra1_key_name, original.extra1_key_name);
        assert_eq!(back.extra1_value, original.extra1_value);
        assert_eq!(back.extra1_encrypted, original.extra1_encrypted);
        assert_eq!(back.extra2_key_name, original.extra2_key_name);
        assert_eq!(back.extra2_encrypted, original.extra2_encrypted);
        assert!(back.extra3_key_name.is_none());
    }

    #[test]
    fn entry_with_no_extras_exports_all_none() {
        let entry = Entry {
            id: 1,
            service_name: "test".to_string(),
            account: "user".to_string(),
            password: "pass".to_string(),
            url: None,
            keyword: "".to_string(),
            category: "".to_string(),
            otp_uri: None,
            notes: None,
            status: 1,
            extra_fields: vec![],
        };
        let flat: FlatEntry = entry.into();
        assert!(flat.extra1_key_name.is_none());
        assert!(flat.extra2_key_name.is_none());
        assert!(flat.extra3_key_name.is_none());
    }

    #[test]
    fn data_store_new_is_empty() {
        let store = DataStore::new();
        assert_eq!(store.version, 1);
        assert!(store.entries.is_empty());
    }
}

impl From<Entry> for FlatEntry {
    fn from(e: Entry) -> Self {
        let get = |i: usize| e.extra_fields.get(i).cloned();
        let ef = |i: usize| -> (Option<String>, Option<String>, Option<bool>) {
            match get(i) {
                Some(f) => (Some(f.key_name), Some(f.value), Some(f.encrypted)),
                None => (None, None, None),
            }
        };
        let (k1, v1, e1) = ef(0);
        let (k2, v2, e2) = ef(1);
        let (k3, v3, e3) = ef(2);
        FlatEntry {
            id: e.id,
            service_name: e.service_name,
            account: e.account,
            password: e.password,
            status: e.status,
            keyword: e.keyword,
            category: e.category,
            extra1_key_name: k1,
            extra1_value: v1,
            extra1_encrypted: e1,
            extra2_key_name: k2,
            extra2_value: v2,
            extra2_encrypted: e2,
            extra3_key_name: k3,
            extra3_value: v3,
            extra3_encrypted: e3,
        }
    }
}
