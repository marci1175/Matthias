use std::io::Error;

pub fn ipv4_get() -> Result<String, Error> {
    // Send an HTTP GET request to a service that returns your public IPv4 address
    let response = reqwest::blocking::get("https://ipv4.icanhazip.com/");
    // Check if the request was successful
    if response.is_ok() {
        let public_ipv4 = response.unwrap().text();

        Ok(public_ipv4.unwrap())
    } else {
        Err(Error::new(
            std::io::ErrorKind::ConnectionRefused,
            "Failed to fetch ip address",
        ))
    }
}
pub fn ipv6_get() -> Result<String, Error> {
    // Send an HTTP GET request to a service that returns your public IPv4 address
    let response = reqwest::blocking::get("https://ipv6.icanhazip.com/");
    // Check if the request was successful
    if response.is_ok() {
        let public_ipv4 = response.unwrap().text();

        Ok(public_ipv4.unwrap())
    } else {
        Err(Error::new(
            std::io::ErrorKind::ConnectionRefused,
            "Failed to fetch ip address",
        ))
    }
}
