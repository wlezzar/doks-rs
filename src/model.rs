use std::collections::HashMap;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Document {
    pub id: String,
    pub source: String,
    pub title: String,
    pub link: String,
    pub content: String,
    pub metadata: HashMap<String, String>,
}
