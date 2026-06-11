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
}

impl DataStore {
    pub fn new() -> Self {
        Self { version: 1, entries: vec![] }
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
