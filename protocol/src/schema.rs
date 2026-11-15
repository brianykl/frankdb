use crate::value::ColumnType;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq)]
pub struct Column {
    name: String,
    column_type: ColumnType,
    nullable: bool,
}

impl Column {
    pub fn new(name: &str, column_type: ColumnType, nullable: bool) -> Self {
        Column {
            name: name.to_string(),
            column_type,
            nullable,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Schema {
    columns: Vec<Column>,
    primary_key: usize,
}

#[derive(Debug, PartialEq)]
pub enum SchemaError {
    PrimaryKeyDoesNotExist(String),
    PrimaryKeyIsNullable(String),
    DuplicateColumnName(String),
    PrimaryKeyIsFloat(String),
}

impl Schema {
    pub fn new(columns: Vec<Column>, primary_key: &str) -> Result<Self, SchemaError> {
        let mut seen = HashSet::new();
        for column in &columns {
            if !seen.insert(&column.name) {
                return Err(SchemaError::DuplicateColumnName(column.name.to_string()));
            }
        }

        let Some(index) = columns.iter().position(|c| c.name == primary_key) else {
            return Err(SchemaError::PrimaryKeyDoesNotExist(primary_key.to_string()));
        };

        let column = &columns[index];

        if column.column_type == ColumnType::Float {
            return Err(SchemaError::PrimaryKeyIsFloat(primary_key.to_string()));
        }

        if column.nullable {
            return Err(SchemaError::PrimaryKeyIsNullable(primary_key.to_string()));
        }

        Ok(Schema {
            columns,
            primary_key: index,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frank_columns() -> Vec<Column> {
        vec![
            Column::new("id", ColumnType::Text, false),
            Column::new("price", ColumnType::Integer, false),
            Column::new("note", ColumnType::Text, true),
        ]
    }

    #[test]
    fn valid_schema_is_accepted() {
        assert!(Schema::new(frank_columns(), "id").is_ok());
    }

    #[test]
    fn integer_primary_key_is_accepted() {
        assert!(Schema::new(frank_columns(), "price").is_ok());
    }

    #[test]
    fn duplicate_column_name_is_rejected() {
        let columns = vec![
            Column::new("id", ColumnType::Text, false),
            Column::new("price", ColumnType::Integer, false),
            Column::new("price", ColumnType::Text, true),
        ];
        assert_eq!(
            Schema::new(columns, "id").err(),
            Some(SchemaError::DuplicateColumnName("price".to_string()))
        );
    }

    #[test]
    fn missing_primary_key_is_rejected() {
        assert_eq!(
            Schema::new(frank_columns(), "uqid").err(),
            Some(SchemaError::PrimaryKeyDoesNotExist("uqid".to_string()))
        );
    }

    #[test]
    fn nullable_primary_key_is_rejected() {
        assert_eq!(
            Schema::new(frank_columns(), "note").err(),
            Some(SchemaError::PrimaryKeyIsNullable("note".to_string()))
        );
    }

    #[test]
    fn float_primary_key_is_rejected() {
        let columns = vec![Column::new("score", ColumnType::Float, false)];
        assert_eq!(
            Schema::new(columns, "score").err(),
            Some(SchemaError::PrimaryKeyIsFloat("score".to_string()))
        );
    }

    #[test]
    fn empty_schema_is_rejected() {
        assert_eq!(
            Schema::new(Vec::new(), "id").err(),
            Some(SchemaError::PrimaryKeyDoesNotExist("id".to_string()))
        );
    }
}
