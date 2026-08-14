use crate::SourceInfo;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RoutingError {
    Unsupported,
    Unavailable(String),
}

pub trait AudioRoutingBackend: Send {
    fn list_sources(&self) -> Result<Vec<SourceInfo>, RoutingError>;
    fn list_outputs(&self) -> Result<Vec<String>, RoutingError>;
    fn restore(&mut self) -> Result<(), RoutingError>;
}
