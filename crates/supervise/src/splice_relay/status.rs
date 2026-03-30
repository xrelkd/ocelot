#[derive(Clone, Debug, Default)]
pub struct Status {
    pub active_relays: usize,
    pub total_added: u64,
    pub total_removed: u64,
    pub bytes_transferred: u64,
}
