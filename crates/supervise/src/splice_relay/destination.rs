use std::os::fd::OwnedFd;

#[derive(Debug)]
pub enum Destination {
    Stdout,
    Stderr,
    OwnedFd { fd: OwnedFd },
}
