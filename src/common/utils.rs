pub fn parse_host(host: &str, default_port: u16) -> anyhow::Result<(&str, u16)> {
    if let Some((host, port)) = host.split_once(":") {
        let port: u16 = port.parse()?;
        Ok((host, port))
    } else {
        Ok((host, default_port))
    }
}