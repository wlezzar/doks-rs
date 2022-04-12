use anyhow::bail;
use serde::de::DeserializeOwned;
use serde_json::Value;

pub fn get_path<'a>(json: &'a Value, path: &[&str]) -> anyhow::Result<&'a Value> {
    let response = path.iter().fold::<Option<_>, _>(Some(json), |acc, node| {
        acc.and_then(|element| element.get(node))
    });

    match response {
        None => bail!("Path not found: {:?}", path),
        Some(value) => Ok(value),
    }
}

pub fn get_array<'a>(json: &'a Value, path: &[&str]) -> anyhow::Result<&'a Vec<Value>> {
    let value = get_path(json, path)?;

    match value {
        Value::Array(ref arr) => Ok(arr),
        other => bail!("Expected array at path '{:?}' but found: {}", path, other),
    }
}

pub fn parse_json<'a, V>(json: &'a Value, path: &[&str]) -> anyhow::Result<V>
    where V: DeserializeOwned {
    let value: &Value = get_path(json, path)?;
    let parsed: V = serde_json::from_value(value.clone())?;
    Ok(parsed)
}


#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use crate::utils::json::{get_array, get_path};

    #[test]
    fn test_get_path() -> anyhow::Result<()> {
        let data = json!({
            "hello": "world",
            "nested": {
                "data": "was nested data"
            }
        });

        assert_eq!(get_path(&data, &["hello"])?, &Value::String("world".to_string()));
        assert_eq!(get_path(&data, &["nested", "data"])?, &Value::String("was nested data".to_string()));

        Ok(())
    }

    #[test]
    fn test_get_array() -> anyhow::Result<()> {
        let data = json!({
            "hello": "world",
            "nested": {
                "array": ["1", "2", "3"]
            }
        });

        assert_eq!(
            get_array(&data, &["nested", "array"])?,
            &vec!["1", "2", "3"],
        );

        Ok(())
    }
}