use thiserror::Error;

#[derive(Error, Debug)]
pub enum SyncError {
    #[error("Molecule error {0}")]
    Molecule(#[from] molecule::error::VerificationError),
}
