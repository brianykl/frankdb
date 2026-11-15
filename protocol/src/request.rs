use crate::schema::Schema;
use crate::value::Record;
use crate::value::Value;
use crate::lsn::LogSequenceNumber;

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    CreateTable { table_name: String, schema: Schema },
    Insert { table_name: String, record: Record },
    Get { table_name: String, key: Value, minimum_log_sequence_number: Option<LogSequenceNumber> },

}