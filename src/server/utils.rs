use tokio_postgres::Error;
use tonic::{Status, Code};

pub fn err_status<T>(result: Result<T, Error>) -> Result<T, Status> {
    result.map_err(|e| {
        eprintln!("{}", e);
        Status::new(Code::Internal, "Internal error")
    })
}
