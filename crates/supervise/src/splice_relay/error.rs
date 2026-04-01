use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Failed to create eventfd: {source}"))]
    CreateEventfd { source: nix::Error },

    #[snafu(display("Failed to create epoll instance: {source}"))]
    CreateEpoll { source: nix::Error },

    #[snafu(display("Failed to add file descriptor to epoll: {source}"))]
    AddEpollFd { source: nix::Error },

    #[snafu(display("Failed to delete file descriptor from epoll: {source}"))]
    DelEpollFd { source: nix::Error },
}
