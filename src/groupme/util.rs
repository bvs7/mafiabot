use anyhow::{Context, Result};
use serde_json::Value as JsonValue;
use std::env;

pub fn get_token() -> Result<String> {
    env::var("GROUPME_TOKEN").with_context(|| "failed to get groupme token")
}

#[derive(Debug)]
pub struct UnexpectedJsonError<'a> {
    value: JsonValue,
    path: &'a str,
    source: Option<serde_json::Error>,
}
impl<'a> std::error::Error for UnexpectedJsonError<'a> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_ref()
            .map(|e| e as &(dyn std::error::Error + 'static))
    }
}
impl<'a> std::fmt::Display for UnexpectedJsonError<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unexpected JSON error")
    }
}

pub fn json_access<'a, 'b, T: serde::de::DeserializeOwned>(
    value: &'b JsonValue,
    path: &'a str,
) -> Result<T, UnexpectedJsonError<'a>> {
    let keys = path.split(".");
    let mut v = value;
    for key in keys {
        // If we have a key, first ensure v is indexable
        match v {
            JsonValue::Object(_) | JsonValue::Array(_) | JsonValue::String(_) => {}
            _ => {
                return Err(UnexpectedJsonError {
                    value: value.clone(),
                    path,
                    source: None,
                })
            }
        }
        match key.parse::<usize>() {
            Ok(k) => v = &v[k],
            Err(_) => v = &v[key],
        }
    }
    let t: T = match serde_json::from_value(v.clone()) {
        Ok(t) => t,
        Err(err) => {
            return Err(UnexpectedJsonError {
                value: value.clone(),
                path,
                source: Some(err),
            })
        }
    };
    return Ok(t);
}

#[cfg(test)]
mod tests {
    use super::{get_token, json_access};
    use anyhow::Result;
    use serde_json::{json, Value as JsonValue};

    #[test]
    fn test_json_access() -> Result<()> {
        let value = json!({
            "outer" : {
                "inner" : {
                    "array" : [
                        "0",
                        "1",
                        "2",
                        "3",
                    ],
                    "number" : 100,
                    "string" : "string",
                    "bool": true,
                    "null": null,
                }
            }
        });

        let outer: JsonValue = json_access(&value, "outer")?;
        let inner: JsonValue = json_access(&value, "outer.inner")?;
        let inner_: JsonValue = json_access(&outer, "inner")?;
        assert!(inner == inner_);

        assert!(json_access::<u64>(&value, "badrefs").is_err());

        let array: Vec<String> = json_access(&value, "outer.inner.array")?;

        let array_element: String = json_access(&value, "outer.inner.array.1")?;

        assert!(json_access::<String>(&value, "outer.inner.array.100").is_err());

        let number: u32 = json_access(&value, "outer.inner.number")?;
        assert_eq!(number, 100);
        let string: String = json_access(&value, "outer.inner.string")?;
        assert_eq!(string, "string".to_owned());
        let boolean: bool = json_access(&value, "outer.inner.bool")?;
        assert_eq!(boolean, true);

        assert!(json_access::<()>(&value, "outer.inner.null").is_ok());
        Ok(())
    }

    #[test]
    fn test_get_token() {
        assert!(get_token().is_ok())
    }
}
