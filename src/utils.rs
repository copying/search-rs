use std::fmt;
use tonic::{Status, Code, metadata::AsciiMetadataValue};


pub fn internal_error<E: fmt::Display>(error: E) -> Status {
    eprintln!("{}", error);
    Status::new(Code::Internal, "Internal error")
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
