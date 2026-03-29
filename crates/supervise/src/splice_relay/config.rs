#[derive(Clone, Debug)]
pub struct Config {
    pub buffer_size: usize,
    pub chunk_size: usize,
}

impl Default for Config {
    fn default() -> Self { Self { buffer_size: 128 * 1024, chunk_size: 128 * 1024 } }
}

impl Config {
    #[must_use]
    pub const fn new(buffer_size: usize, chunk_size: usize) -> Self {
        Self { buffer_size, chunk_size }
    }

    #[must_use]
    pub const fn with_buffer_size(mut self, buffer_size: usize) -> Self {
        self.buffer_size = buffer_size;
        self
    }

    #[must_use]
    pub const fn with_chunk_size(mut self, chunk_size: usize) -> Self {
        self.chunk_size = chunk_size;
        self
    }
}
