
use crate::request::Request;
use crate::response::Response;
use crate::error::DbError;

pub trait Executor: Send + Sync {
    fn execute(&self, request: Request) -> impl Future<Output = Result<Response, DbError>> + Send;
}