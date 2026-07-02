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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_store_new_is_empty() {
        let store = DataStore::new();
        assert_eq!(store.version, 1);
        assert!(store.entries.is_empty());
        assert!(store.categories.is_empty());
    }

    // --- DataStore categories ---

    #[test]
    fn data_store_categories_deserialize_with_default_when_absent() {
        // categories フィールドが存在しない古い形式でも正常にデシリアライズできる
        let json = r#"{"version":1,"entries":[]}"#;
        let store: DataStore = serde_json::from_str(json).unwrap();
        assert!(store.categories.is_empty());
    }

    #[test]
    fn data_store_categories_roundtrip() {
        let mut store = DataStore::new();
        store.categories = vec!["仕事".to_string(), "プライベート".to_string()];
        let json = serde_json::to_string(&store).unwrap();
        let restored: DataStore = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.categories, vec!["仕事", "プライベート"]);
    }

    #[test]
    fn data_store_categories_preserves_order() {
        let mut store = DataStore::new();
        store.categories = vec!["Z".to_string(), "A".to_string(), "M".to_string()];
        let json = serde_json::to_string(&store).unwrap();
        let restored: DataStore = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.categories, vec!["Z", "A", "M"]);
    }
}

