#[derive(Debug, thiserror::Error)]
pub enum RadioError {
    #[error("SoapySDR: {0}")]
    Soapy(#[from] soapysdr::Error),
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Msg(String),
}
