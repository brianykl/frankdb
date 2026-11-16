
use crate::lsn::LogSequenceNumber;
use crate::value::Record;

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Response {
    Written(LogSequenceNumber),
    Row { record: Option<Record>, log_sequence_number: LogSequenceNumber },
}