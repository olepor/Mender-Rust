use openssl::rsa::{Padding, Rsa};
// use reqwest::Client;
use std::collections::HashMap;

use serde::Serialize;

#[derive(Serialize)]
struct IDData {
    data: HashMap<String, String>,
}

impl IDData {
    fn new() -> IDData {
        IDData {
            data: HashMap::new(),
        }
    }
    fn fill(&mut self) {
        // TODO -- Only dummy data for now!
        self.data
            .insert("MAC".to_string(), "123::345::678".to_string());
        self.data
            .insert("serial_number".to_string(), "12345678".to_string());
    }
}

// TODO -- This needs to be serialized to bytes (Through serde(?))
#[derive(Serialize)]
struct AuthRequestBody {
    id_data: IDData,
    pubkey: String,
    tenant_token: Option<String>,
}

pub struct Client {
    is_authorized: bool,
    address: String,
    private_key: Rsa<openssl::pkey::Private>,
    // public_key: Rsa<openssl::pkey::Public>,
    tenant_token: Option<String>,
    // Request signature, computed as
    // 'BASE64(SIGN(device_private_key, SHA256(request_body)))'.
    // Verified with the public key presented by the device.
    // signature: String,
}

impl Client {
    pub fn new() -> Client {
        use std::fs::File;
        use std::io::Read;
        if let Ok(mut file) = File::open("./dummies/id_rsa") {
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)
                .expect("Failed to read from the pem file (id_rsa)");
            let rsa = Rsa::private_key_from_pem(buffer.as_slice())
                .expect("Failed to extract the private key from pem file");
            Client {
                is_authorized: false,
                address: String::from("https://docker.mender.io"),
                private_key: rsa,
                tenant_token: None,
            }
        } else {
            let rsa = Self::generate_private_key();
            Client {
                is_authorized: false,
                address: String::from("https://docker.mender.io"),
                private_key: rsa,
                tenant_token: None,
            }
        }
    }
    fn generate_private_key() -> Rsa<openssl::pkey::Private> {
        Rsa::generate(2048).unwrap()
    }

    pub fn authorize(&self) -> bool {
        if !self.is_authorized {
            // Do authorization
            // Authorization API can be found at:
            // https://docs.mender.io/2.0/apis/device-apis/device-authentication
            let protocol = "https://";
            let host = "docker.mender.io";
            let basepath = "/api/devices/v1/authentication";
            let request = "/authentication/auth_requests";
            let uri = protocol.to_owned() + host + basepath + request;
            // Create the AuthRequest body
            let pem_pub_key =
                String::from_utf8(self.private_key.public_key_to_pem_pkcs1().unwrap()).unwrap();
            let auth_req = AuthRequestBody {
                id_data: IDData::new(),
                pubkey: pem_pub_key,
                tenant_token: None, // TODO -- This needs to be handled
            };
            // serialize the request to json
            let auth_req_str = serde_json::to_string(&auth_req)
                .expect("Failed to serialize the authorization request to json");
            // First do a shasum256 of the request
            use openssl::hash::{hash, MessageDigest};
            let request_sha256_sum =
                hash(MessageDigest::sha256(), auth_req_str.as_bytes()).unwrap();
            // Sign the authorization request with the private(?) key
            let mut sig = [0; 2048];
            let encrypted_len = self
                .private_key
                .private_encrypt(&request_sha256_sum, &mut sig, Padding::PKCS1)
                .expect("Failed to sign the request body");
            println!("Encrypted length: {}", encrypted_len);
            // Base64 encode the signature
            let sig_base64 = base64::encode(&sig[..]);
            let request_client = reqwest::Client::new();
            let res = request_client
                .post(&uri)
                .header("Content-Type", "application/json")
                // .header("Authorization", "Bearer ".to_owned() + "TODO -- Token") Not supported yet
                .header("X-MEN-Signature", sig_base64)
                .body(auth_req_str)
                .send()
                .expect("Failed to POST the authorization request");
            println!("Response from authorization request: {:?}", res);
            true
        } else {
            false
        }
    }
}

impl Client {
    pub fn is_authorized(&self) -> bool {
        return self.is_authorized;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_authorization() {
        let client = Client::new();
        assert_eq!(client.authorize(), true);
    }
}
