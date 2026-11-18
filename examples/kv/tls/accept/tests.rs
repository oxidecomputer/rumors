use anyhow::Result;

use crate::credentials::Credentials;
use crate::tls::Tls;

#[tokio::test]
async fn only_peers_holding_the_same_credentials_can_connect() -> Result<()> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?.to_string().parse()?;

    let can_connect = async |client: &Tls, server: &Tls| {
        let connect = client.connect(&address);
        let accept = async { server.accept(listener.accept().await?.0).await };
        let (connected, accepted) = tokio::join!(connect, accept);
        connected.is_ok() && accepted.is_ok()
    };

    let peers = [
        Tls::new(Credentials::generate()?)?,
        Tls::new(Credentials::generate()?)?,
    ];

    for (n, p) in peers.iter().enumerate() {
        for (m, q) in peers.iter().enumerate() {
            assert_eq!(n == m, can_connect(p, q).await);
        }
    }

    Ok(())
}
