use anyhow::ensure;

fn parse_peer(address: &str) -> Result<Authority> {
    let address = match address.strip_prefix(':') {
        Some(port) => format!("localhost:{port}"),
        None => address.to_owned(),
    };
    let authority: Authority = address.parse()?;
    let host = authority.host();
    let understood = match authority.port_u16() {
        Some(port) => format!("{host}:{port}"),
        None => host.to_owned(),
    };
    ensure!(understood == address, "expected HOST, HOST:PORT, or :PORT");
    Ok(authority)
}
