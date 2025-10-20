// Copyright (c) Microsoft Corporation. All rights reserved.
use vercode::{Vercode, deserialize, serialize};

#[test]
fn socketaddr_v4() {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

    let mut buf = vec![0u8; 256];
    let serialized = serialize(&addr, &mut buf);
    let decoded: SocketAddr = deserialize(serialized).unwrap();

    assert_eq!(decoded, addr);
    assert_eq!(serialized.len(), 7); // 1 byte discriminant + 4 bytes IP + 2 bytes port
}

#[test]
fn socketaddr_v6() {
    use std::net::{IpAddr, Ipv6Addr, SocketAddr};

    let addr = SocketAddr::new(
        IpAddr::V6(Ipv6Addr::new(0x2001, 0x0db8, 0, 0, 0, 0, 0, 1)),
        8080,
    );

    let mut buf = vec![0u8; 256];
    let serialized = serialize(&addr, &mut buf);
    let decoded: SocketAddr = deserialize(serialized).unwrap();

    assert_eq!(decoded, addr);
    // 1 byte discriminant + 16 bytes IP + 2 bytes port + 4 bytes flowinfo + 4 bytes scope_id = 27 bytes
    assert_eq!(serialized.len(), 27);
}

#[test]
fn socketaddr_v6_with_flowinfo_and_scope() {
    use std::net::{Ipv6Addr, SocketAddrV6};

    let addr = SocketAddrV6::new(
        Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1),
        8080,
        12345, // flowinfo
        6,     // scope_id
    );
    let socket_addr = std::net::SocketAddr::V6(addr);

    let mut buf = vec![0u8; 256];
    let serialized = serialize(&socket_addr, &mut buf);
    let decoded: std::net::SocketAddr = deserialize(serialized).unwrap();

    assert_eq!(decoded, socket_addr);

    // Verify the flowinfo and scope_id were preserved
    if let std::net::SocketAddr::V6(decoded_v6) = decoded {
        assert_eq!(decoded_v6.flowinfo(), 12345);
        assert_eq!(decoded_v6.scope_id(), 6);
    } else {
        panic!("Expected V6 address");
    }
}

#[test]
fn socketaddr_localhost_variants() {
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

    // IPv4 localhost
    let v4_localhost = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 3000);
    let mut buf = vec![0u8; 256];
    let serialized = serialize(&v4_localhost, &mut buf);
    let decoded: SocketAddr = deserialize(serialized).unwrap();
    assert_eq!(decoded, v4_localhost);

    // IPv6 localhost
    let v6_localhost = SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), 3000);
    let mut buf = vec![0u8; 256];
    let serialized = serialize(&v6_localhost, &mut buf);
    let decoded: SocketAddr = deserialize(serialized).unwrap();
    assert_eq!(decoded, v6_localhost);
}

#[test]
fn socketaddr_in_struct() {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    #[derive(Vercode, Debug, PartialEq, Clone)]
    struct Server {
        name: String,
        address: SocketAddr,
        port: u16,
    }

    let server = Server {
        name: "web-server".to_string(),
        address: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)), 8080),
        port: 8080,
    };

    let mut buf = vec![0u8; 256];
    let serialized = serialize(&server, &mut buf);
    let decoded: Server = deserialize(serialized).unwrap();

    assert_eq!(decoded, server);
    assert_eq!(decoded.name, "web-server");
}

#[test]
fn socketaddr_vec() {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    let addrs = vec![
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080),
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 8081),
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 8082),
    ];

    let mut buf = vec![0u8; 512];
    let serialized = serialize(&addrs, &mut buf);
    let decoded: Vec<SocketAddr> = deserialize(serialized).unwrap();

    assert_eq!(decoded.len(), 3);
    assert_eq!(decoded, addrs);
}
