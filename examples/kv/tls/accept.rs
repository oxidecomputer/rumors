impl Tls {
    async fn accept(
        &self,
        tcp: TcpStream,
    ) -> Result<(
        impl AsyncBufRead + Unpin + Send,
        impl AsyncWrite + Unpin + Send,
    )> {
        tcp.set_nodelay(true)?;
        let (read, write) = tokio::io::split(self.acceptor.accept(tcp).await?);
        Ok((BufReader::new(read), write))
    }
}
