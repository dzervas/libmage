extern crate heck;
extern crate url;

pub mod key;

use heck::CamelCase;
use url::Url;

pub struct Address {
    pub transport: String,
    pub listen: bool,
    pub host: String,
    pub port: u16,
}

impl Address {
    pub fn parse(address: String) -> Address {
        let url = Url::parse(address.as_str()).unwrap();
        let scheme_parts = url.scheme().split("+").collect::<Vec<&str>>();
        let transport = scheme_parts.get(0).unwrap().to_camel_case();
        let listen = match scheme_parts.get(1) {
            Some(d) => d == &"listen",
            None => false,
        };
        let host = url.host_str().unwrap().to_string();
        let port = url.port().unwrap();

        Address {
            transport,
            listen,
            host,
            port
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let mut parser = Address::parse("tcp://127.0.0.1:4444".to_string());

        assert_eq!(parser.transport, "Tcp");
        assert_eq!(parser.listen, false);
        assert_eq!(parser.host, "127.0.0.1");
        assert_eq!(parser.port, 4444);

        parser = Address::parse("https+listen://google.com:11223".to_string());

        assert_eq!(parser.transport, "Https");
        assert_eq!(parser.listen, true);
        assert_eq!(parser.host, "google.com");
        assert_eq!(parser.port, 11223);
    }
}
