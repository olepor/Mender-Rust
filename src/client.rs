use http::{Request, Response};
use openssl::rsa::{Padding, Rsa};
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
    fn fill(& mut self) {
        // TODO -- Only dummy data for now!
        self.data
            .insert("MAC".to_string(), "123::345::678".to_string());
        self.data
            .insert("serial_number".to_string(), "12345678".to_string());
    }
}

// TODO -- This needs to be serialized to bytes
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
    tenant_token: String,
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
                tenant_token: String::from("footoken"),
            }
        } else {
            let rsa = Self::generate_private_key();
            Client {
                is_authorized: false,
                address: String::from("https://docker.mender.io"),
                private_key: rsa,
                tenant_token: String::from("footoken"),
            }
        }
    }
    fn generate_private_key() -> Rsa<openssl::pkey::Private> {
        Rsa::generate(2048).unwrap()
        // TODO -- Write the key to file!
        // self.public_key = Rsa::from(rsa);
        // let data = b"foobar";
        // let mut buf = vec![0; rsa.size() as usize];
        // let encrypted_len = rsa.public_encrypt(data, &mut buf, Padding::PKCS1).unwrap();
    }

    pub fn authorize(&self) -> bool {
        if !self.is_authorized {
            // Do authorization
            // Authorization API can be found at:
            // https://docs.mender.io/2.0/apis/device-apis/device-authentication
            // HOST: docker.mender.io
            // Current implementation API: HOST/authentication/auth_requests
            // Submit an authentication request:
            // POST /auth_requests
            let protocol = "https://";
            let host = "docker.mender.io";
            let basepath = "/api/devices/v1/authentication";
            let request = "/authentication/auth_requests";
            let uri = protocol.to_owned() + host + basepath + request;
            // hreq.Header.Add("Content-Type", "application/json")
            //     hreq.Header.Add("Authorization", fmt.Sprintf("Bearer %s", req.Token))
            //     hreq.Header.Add("X-MEN-Signature", base64.StdEncoding.EncodeToString(req.Signature))
            // Create the AuthRequest body
            let n = self.private_key.n().clone().to_owned().unwrap();
            let e = self.private_key.e().clone().to_owned().unwrap();
            let pub_key = Rsa::from_public_components(n, e).unwrap();
            let auth_req = AuthRequestBody{
                id_data: IDData::new(),
                pubkey: String::from("pub-key -- TODO"), //pub_key.to_string(),
                tenant_token: None, // TODO -- This needs to be handled
            };
            let mut request: Request<&str> = Request::builder()
                .method("POST")
                .uri(uri)
                .header("Content-Type", "application/json")
                .header("Authorization", "Bearer ".to_owned() + "TODO -- Token")
                .header("X-MEN-Signature", "TODO -- req_signature")
                .body("AuthRequest -- TODO")
                .unwrap();
            // First do a shasum256 of the request
            use openssl::hash::{hash, MessageDigest};
            let sha256_sum = hash(MessageDigest::sha256(), request.body().as_bytes()).unwrap();
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
