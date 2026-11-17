//! A named set of variables (`local`, `staging`, `prod`…) of a collection.

use crate::key_value::KeyValues;
use crate::template::Variables;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Environment {
    pub name: String,
    pub variables: KeyValues,
}

impl Environment {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            variables: KeyValues::default(),
        }
    }

    /// Its enabled variables, resolved against each other.
    pub fn resolve(&self) -> Variables {
        Variables::resolve(
            self.variables
                .active()
                .map(|kv| (kv.key.as_str(), kv.value.as_str())),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key_value::KeyValue;

    #[test]
    fn disabled_variables_are_not_defined() {
        let mut env = Environment::new("dev");
        env.variables = KeyValues(vec![
            KeyValue::new("host", "localhost"),
            KeyValue {
                enabled: false,
                ..KeyValue::new("token", "secret")
            },
        ]);
        let vars = env.resolve();
        assert_eq!(vars.get("host"), Some("localhost"));
        assert!(!vars.contains("token"));
    }
}
