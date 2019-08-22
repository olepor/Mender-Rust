use openssl::rsa::{Padding, Rsa};
use http::{Request, Response};

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
            // TODO -- What is the server address?
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
            let request = Request::builder()
                .uri(uri)
                .header("Content-Type", "application/json")
                .header("Authorization", "Bearer ".to_owned() + "TODO -- Token")
                .header("X-MEN-Signature", "TODO -- req_signature")
                .body(())
                .unwrap();
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
