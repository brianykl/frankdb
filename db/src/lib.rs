#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LogSequenceNumber {
    lsn: u64,
}

impl LogSequenceNumber {
    pub fn new(lsn: u64) -> LogSequenceNumber {
        LogSequenceNumber { lsn }
    }

    pub fn lsn(&self) -> u64 {
        self.lsn
    }

    pub fn next(&self) -> LogSequenceNumber {
        LogSequenceNumber::new(self.lsn.checked_add(1).expect("happy frank overflow"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = LogSequenceNumber::new(1).next();
        assert_eq!(result, LogSequenceNumber::new(2));
    }

    #[test]
    #[should_panic]
    fn it_overflows() {
        LogSequenceNumber::new(u64::MAX).next();
    }
}
