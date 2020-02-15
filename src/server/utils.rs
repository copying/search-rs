use tokio_postgres::Error;
use tonic::{Status, Code, metadata::AsciiMetadataValue};

pub fn err_status<T>(result: Result<T, Error>) -> Result<T, Status> {
    result.map_err(|e| {
        eprintln!("{}", e);
        Status::new(Code::Internal, "Internal error")
    })
}

pub fn require_arg(value: Option<&AsciiMetadataValue>) -> Result<String, Status> {
    match value {
        Some(v) => match v.to_str() {
            Ok(v) => Ok(v.to_string()),
            Err(_) => Err(Status::new(Code::InvalidArgument, "Invalid index provided"))
        },
        None => Err(Status::new(Code::InvalidArgument, "No index was provided in the metadata")),
    }
}
