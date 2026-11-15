use std::collections::BTreeMap;
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Integer(i64),
    Float(f64),
    Text(String),
    Boolean(bool),
    Null,
}

#[derive(Debug, Clone, PartialEq, Copy, Eq)]
pub enum ColumnType {
    Integer,
    Float,
    Text,
    Boolean,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Record {
    columns: BTreeMap<String, Value>,
}

impl Record {
    pub fn new() -> Record {
        Record {
            columns: BTreeMap::new(),
        }
    }

    pub fn set(&mut self, column_name: &str, value: Value) {
        self.columns.insert(column_name.to_string(), value);
    }

    pub fn get(&self, column_name: &str) -> Option<&Value> {
        self.columns.get(column_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_then_get_returns_value() {
        let mut record = Record::new();
        record.set("price", Value::Integer(5000));
        assert_eq!(record.get("price"), Some(&Value::Integer(5000)));
    }

    #[test]
    fn get_missing_column_returns_none() {
        let record = Record::new();
        assert_eq!(record.get("missing"), None);
    }

    #[test]
    fn set_overwrites_existing_value() {
        let mut record = Record::new();
        record.set("price", Value::Integer(5000));
        record.set("price", Value::Integer(6000));
        assert_eq!(record.get("price"), Some(&Value::Integer(6000)));
    }

    #[test]
    fn records_with_same_columns_are_equal_regardless_of_set_order() {
        let mut a = Record::new();
        a.set("price", Value::Integer(5000));
        a.set("event_id", Value::Text("evt-17".to_string()));

        let mut b = Record::new();
        b.set("event_id", Value::Text("evt-17".to_string()));
        b.set("price", Value::Integer(5000));

        assert_eq!(a, b);
    }
}
