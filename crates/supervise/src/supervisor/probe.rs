use std::{net::SocketAddr, time::Duration};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpStream, ToSocketAddrs},
};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ProbeHandler {
    HttpGet { host: Option<String>, path: String, port: u16 },
    TcpSocket { host: Option<String>, port: u16 },
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Probe {
    pub handler: ProbeHandler,

    pub initial_delay: Duration,

    pub period: Duration,

    pub timeout: Duration,

    pub failure_threshold: i32,

    pub success_threshold: i32,
}

impl Default for Probe {
    fn default() -> Self {
        Self {
            handler: ProbeHandler::HttpGet { host: None, path: "/".to_string(), port: 80 },
            initial_delay: Duration::ZERO,
            period: Duration::from_secs(10),
            timeout: Duration::from_secs(1),
            failure_threshold: 3,
            success_threshold: 1,
        }
    }
}

impl Probe {
    pub async fn check(&self) -> bool {
        let Some(socket_address) = self.resolve_socket_address().await else {
            return false;
        };

        match &self.handler {
            ProbeHandler::HttpGet { path, .. } => {
                probe_http(&socket_address, self.timeout, path).await
            }
            ProbeHandler::TcpSocket { .. } => probe_tcp(&socket_address, self.timeout).await,
        }
    }

    async fn resolve_socket_address(&self) -> Option<SocketAddr> {
        let addr = match &self.handler {
            ProbeHandler::HttpGet { host, port, .. } | ProbeHandler::TcpSocket { host, port } => {
                let host = host.as_deref().unwrap_or("localhost");
                format!("{host}:{port}")
            }
        };
        tokio::net::lookup_host(&addr).await.ok()?.next()
    }
}

async fn probe_tcp<A>(socket_address: A, timeout: Duration) -> bool
where
    A: ToSocketAddrs,
{
    tokio::time::timeout(timeout, TcpStream::connect(socket_address))
        .await
        .ok()
        .is_some_and(|result| result.is_ok())
}

async fn probe_http(socket_address: &SocketAddr, timeout: Duration, path: &str) -> bool {
    let result = tokio::time::timeout(timeout, async {
        let mut stream = TcpStream::connect(socket_address).await.ok()?;
        let request = format!("GET {path} HTTP/1.1\r\nConnection: close\r\n\r\n");
        stream.write_all(request.as_bytes()).await.ok()?;
        let mut response = String::new();
        let _ = stream.read_to_string(&mut response).await.ok()?;
        let status_code =
            response.lines().next()?.split_whitespace().nth(1)?.parse::<u16>().ok()?;
        Some(status_code)
    })
    .await;

    result.is_ok_and(|opt| opt.is_some_and(|code| (200..=299).contains(&code)))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::supervisor::probe::{Probe, ProbeHandler};

    #[test]
    fn test_probe_default() {
        let probe = Probe::default();
        assert_eq!(
            probe.handler,
            ProbeHandler::HttpGet { host: None, path: "/".to_string(), port: 80 }
        );
        assert_eq!(probe.initial_delay, Duration::ZERO);
        assert_eq!(probe.period, Duration::from_secs(10));
        assert_eq!(probe.timeout, Duration::from_secs(1));
        assert_eq!(probe.failure_threshold, 3);
        assert_eq!(probe.success_threshold, 1);
    }

    #[test]
    fn test_probe_handler_http_get() {
        let handler = ProbeHandler::HttpGet {
            host: Some("localhost".to_string()),
            path: "/health".to_string(),
            port: 8080,
        };
        assert!(matches!(handler, ProbeHandler::HttpGet { .. }));
    }

    #[test]
    fn test_probe_handler_tcp_socket() {
        let handler = ProbeHandler::TcpSocket { host: Some("localhost".to_string()), port: 5432 };
        assert!(matches!(handler, ProbeHandler::TcpSocket { .. }));
    }

    #[test]
    fn test_probe_handler_eq() {
        let handler1 = ProbeHandler::HttpGet {
            host: Some("localhost".to_string()),
            path: "/health".to_string(),
            port: 8080,
        };
        let handler2 = ProbeHandler::HttpGet {
            host: Some("localhost".to_string()),
            path: "/health".to_string(),
            port: 8080,
        };
        let handler3 = ProbeHandler::HttpGet {
            host: Some("other".to_string()),
            path: "/health".to_string(),
            port: 8080,
        };

        assert_eq!(handler1, handler2);
        assert_ne!(handler1, handler3);
    }

    #[test]
    fn test_probe_clone() {
        let probe = Probe {
            handler: ProbeHandler::HttpGet {
                host: Some("localhost".to_string()),
                path: "/health".to_string(),
                port: 8080,
            },
            initial_delay: Duration::from_secs(5),
            period: Duration::from_secs(10),
            timeout: Duration::from_secs(2),
            failure_threshold: 3,
            success_threshold: 1,
        };

        let cloned = probe.clone();
        assert_eq!(probe.handler, cloned.handler);
        assert_eq!(probe.initial_delay, cloned.initial_delay);
        assert_eq!(probe.period, cloned.period);
    }
}
