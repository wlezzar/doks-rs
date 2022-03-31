use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub source: String,
    pub title: String,
    pub link: String,
    pub content: String,
    pub metadata: HashMap<String, String>,
}
