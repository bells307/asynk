#[derive(Debug, thiserror::Error)]
pub enum ReactorError {
    #[error("slab queue is full")]
    SlabQueueFull,
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
