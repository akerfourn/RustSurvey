use std::{future::Future, io, net::SocketAddr};

use tokio::net::TcpListener;

const MAX_PORT_ATTEMPTS: usize = 100;

pub async fn bind_listener(address: SocketAddr, automatic_port: bool) -> io::Result<TcpListener> {
    bind_with(address, automatic_port, |address| {
        TcpListener::bind(address)
    })
    .await
}

async fn bind_with<T, F, Fut>(
    mut address: SocketAddr,
    automatic_port: bool,
    mut bind: F,
) -> io::Result<T>
where
    F: FnMut(SocketAddr) -> Fut,
    Fut: Future<Output = io::Result<T>>,
{
    let mut attempts = 0;
    loop {
        match bind(address).await {
            Ok(listener) => return Ok(listener),
            Err(error) => {
                attempts += 1;
                if !automatic_port
                    || error.kind() != io::ErrorKind::AddrInUse
                    || attempts >= MAX_PORT_ATTEMPTS
                    || address.port() == u16::MAX
                {
                    return Err(error);
                }
                address.set_port(address.port() + 1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn address(port: u16) -> SocketAddr {
        ([127, 0, 0, 1], port).into()
    }

    #[tokio::test]
    async fn test_bind_should_try_following_ports_when_default_port_is_occupied() {
        let mut attempted = Vec::new();

        let result = bind_with(address(3000), true, |candidate| {
            attempted.push(candidate);
            async move {
                if candidate.port() < 3002 {
                    return Err(io::Error::new(io::ErrorKind::AddrInUse, "occupied"));
                }
                Ok(candidate)
            }
        })
        .await
        .unwrap();

        assert_eq!(attempted, [address(3000), address(3001), address(3002)]);
        assert_eq!(result, address(3002));
    }

    #[tokio::test]
    async fn test_bind_should_keep_explicit_port_when_address_is_configured() {
        let mut attempts = 0;

        let error = bind_with::<(), _, _>(address(4000), false, |_| {
            attempts += 1;
            async { Err(io::Error::new(io::ErrorKind::AddrInUse, "occupied")) }
        })
        .await
        .unwrap_err();

        assert_eq!(attempts, 1);
        assert_eq!(error.kind(), io::ErrorKind::AddrInUse);
        assert_eq!(error.to_string(), "occupied");
    }

    #[tokio::test]
    async fn test_bind_should_stop_immediately_when_failure_is_not_a_port_conflict() {
        let mut attempts = 0;

        let error = bind_with::<(), _, _>(address(3000), true, |_| {
            attempts += 1;
            async { Err(io::Error::new(io::ErrorKind::PermissionDenied, "denied")) }
        })
        .await
        .unwrap_err();

        assert_eq!(attempts, 1);
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
        assert_eq!(error.to_string(), "denied");
    }

    #[tokio::test]
    async fn test_bind_should_bound_search_when_every_port_is_occupied() {
        let mut attempts = 0;

        let error = bind_with::<(), _, _>(address(3000), true, |_| {
            attempts += 1;
            async { Err(io::Error::new(io::ErrorKind::AddrInUse, "occupied")) }
        })
        .await
        .unwrap_err();

        assert_eq!(attempts, MAX_PORT_ATTEMPTS);
        assert_eq!(error.kind(), io::ErrorKind::AddrInUse);
    }

    #[tokio::test]
    async fn test_bind_should_preserve_ipv6_and_stop_at_last_port_when_range_is_exhausted() {
        let mut attempted = Vec::new();
        let first = "[::1]:65534".parse().unwrap();

        let error = bind_with::<(), _, _>(first, true, |candidate| {
            attempted.push(candidate);
            async { Err(io::Error::new(io::ErrorKind::AddrInUse, "occupied")) }
        })
        .await
        .unwrap_err();

        assert_eq!(attempted, [first, "[::1]:65535".parse().unwrap()]);
        assert_eq!(error.kind(), io::ErrorKind::AddrInUse);
    }
}
