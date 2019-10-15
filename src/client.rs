use openssl::rsa::{Padding, Rsa};
use std::io::BufWriter;
use std::fs::File;
use std::io::Read;

use log::{debug, info, trace, warn};

use serde::Serialize;
use serde::Deserialize;

#[derive(Serialize)]
struct IDData {
    data: String,
}

impl IDData {
    fn new() -> IDData {
        IDData {
            data: String::from("MAC: 123"),
        }
    }
    fn fill(&mut self) {
        // TODO -- Only dummy data for now!
        // self.data
        //     .insert("MAC".to_string(), "123::345::678".to_string());
        // self.data
        //     .insert("serial_number".to_string(), "12345678".to_string());
    }
}

// TODO -- This needs to be serialized to bytes (Through serde(?))
#[derive(Serialize)]
struct AuthRequestBody {
    id_data: String,
    pubkey: String,
    tenant_token: Option<String>,
}


#[derive(Deserialize, Debug, Clone)]
pub struct Source {
    uri: String,
    expire: String,
}

// Artifact struct holds the information received from a
// GET /deployments/next HTTP response
// TODO -- What fields are optional here?
#[derive(Deserialize, Debug, Clone)]
pub struct Artifact {
    artifact_name: String,
    source: Source,
    device_types_compatible: Vec<String>,
    payload_types: Option<Vec<String>>,
}

// UpdateInfo holds the information received from a GET /deployments/next
// HTTP response
#[derive(Deserialize, Debug, Clone)]
pub struct UpdateInfo {
    id: String,
    artifact: Artifact,
}

#[derive(Debug)]
pub struct ClientError {
    error: Box<std::error::Error>,
}

impl From<std::io::Error> for ClientError {
    fn from(error: std::io::Error) -> Self {
        ClientError{error: Box::new(error)}
    }
}

impl From<reqwest::Error> for ClientError {
    fn from(error: reqwest::Error) -> Self {
        ClientError{error: Box::new(error)}
    }
}

pub struct Client {
    pub is_authorized: bool,
    address: String,
    private_key: Rsa<openssl::pkey::Private>,
    // public_key: Rsa<openssl::pkey::Public>,
    tenant_token: Option<String>,
    pub jwt_token: Option<String>,
    request_client: reqwest::Client,
    // Request signature, computed as
    // 'BASE64(SIGN(device_private_key, SHA256(request_body)))'.
    // Verified with the public key presented by the device.
    // signature: String,
}

impl Client {
    pub fn new() -> Client {
        use std::fs::File;
        use std::io::Read;
        // read the server certificate
        let mut buf = Vec::new();
        File::open("/etc/mender/server.crt").unwrap().read_to_end(&mut buf).unwrap();

        // create a certificate
        let cert = reqwest::Certificate::from_pem(&buf).unwrap();

        let request_client = reqwest::Client::builder()
            .add_root_certificate(cert)
            .build().unwrap();
        if let Ok(mut file) = File::open("./dummies/private-key-rsa.key") {
            debug!("Reading in the private key...");
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
                jwt_token: None,
                request_client: request_client,
            }
        } else {
            debug!("Generating rsa private key of length 3072 bits");
            let rsa = Self::generate_private_key();
            Client {
                is_authorized: false,
                address: String::from("https://docker.mender.io"),
                private_key: rsa,
                tenant_token: None,
                jwt_token: None,
                request_client: request_client,
            }
        }
    }
    fn generate_private_key() -> Rsa<openssl::pkey::Private> {
        Rsa::generate(3072).unwrap()
    }

    pub fn authorize(&self) -> Result<reqwest::Response, ClientError> {
        debug!("The client is trying to authorize...");
        // Do authorization
        // Authorization API can be found at:
        // https://docs.mender.io/2.0/apis/device-apis/device-authentication
        let protocol = "https://";
        let host = "docker.mender.io"; // TODO -- For testing purposes
        let basepath = "/api/devices/v1";
        let request = "/authentication/auth_requests";
        let uri = protocol.to_owned() + host + basepath + request;
        // Create the AuthRequest body
        let pem_pub_key = String::from_utf8(self.private_key.public_key_to_pem().unwrap()).unwrap();
        let id_data = r#"{"MAC": "123"}"#;
        let auth_req = AuthRequestBody {
            id_data: id_data.to_string(),
            pubkey: pem_pub_key,
            tenant_token: None, // TODO -- This needs to be handled
        };
        // serialize the request to json
        let auth_req_str = serde_json::to_string(&auth_req)
            .expect("Failed to serialize the authorization request to json");
        debug!("auth_req_data_str: {}", auth_req_str);
        // Sign using PKCS#1
        let sig = self.sign_request(auth_req_str.as_bytes());
        // Base64 encode the signature
        let sig_base64 = base64::encode(&sig[..384]);

        Ok(self.request_client
            .post(&uri)
            .header("Content-Type", "application/json")
            .header("X-MEN-Signature", sig_base64)
            .body(auth_req_str)
            // .body(auth_req_str.as_bytes())
            .send()?)
    }

    fn sign_request(&self, request: &[u8]) -> [u8; 3072] {
        // Sign the authorization request with the private(?) key
        let rsa_key = self.private_key.clone();
        let keypair = openssl::pkey::PKey::from_rsa(rsa_key).unwrap();
        let mut signer =
            openssl::sign::Signer::new(openssl::hash::MessageDigest::sha256(), &keypair)
                .expect("Failed to create the signer");
        signer
            .set_rsa_padding(Padding::PKCS1)
            .expect("Failed to set the signer padding");
        signer.update(request).expect("Failed to sign the request");
        let mut sig = [0; 3072];
        let len = signer.sign(&mut sig).expect("Failed to sign the payload");
        println!("encypted length: {}", len);
        sig
    }

    fn shasum256_request(&self, request: &[u8]) -> [u8; 32] {
        let mut hasher = openssl::sha::Sha256::new();
        hasher.update(request);
        let hash = hasher.finish();
        hash
    }

    // Host : docker.mender.io
    // BasePath : /api/devices/v1/inventory
    // Schemes : HTTPS
    // Paths
    // PATCH /device/attributes
    pub fn send_inventory(&self) -> Result<reqwest::Response, reqwest::Error> {
        debug!("Client: Sending inventory...");
        let request_client = reqwest::Client::new();
        self.request_client
            .patch("https://docker.mender.io/api/devices/v1/inventory/device/attributes")
            .bearer_auth(self.jwt_token.as_ref().unwrap())
            .json(&[
                InventoryAttribute {
                    name: "Mac".to_string(),
                    value: "123:456:789:123".to_string(),
                },
                InventoryAttribute {
                    name: "device_type".to_string(),
                    value: "qemux86-64".to_string(),
                },
                InventoryAttribute {
                    name: "artifact_name".to_string(),
                    value: "foobar".to_string(),
                },
            ])
            .send()
    }

    // Host : docker.mender.io
    // BasePath : /api/devices/v1/deployments
    // Schemes : HTTPS
    // GET /device/deployments/next
    pub fn check_for_update(&self) -> Result<reqwest::Response, reqwest::Error> {
        debug!("Client: Checking for update...");
        self.request_client
            .get("https://docker.mender.io/api/devices/v1/deployments/device/deployments/next")
            .bearer_auth(self.jwt_token.as_ref().unwrap())
            .query(&[("device_type", "qemux86-64"), ("artifact_name", "foobar")])
            .send()
    }

    pub fn download_update(&self, update_info: UpdateInfo) {
        debug!("Client: Downloading the update...");
        let request_client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .unwrap();
        let mut resp = request_client
            .get(&update_info.artifact.source.uri)
            .send().expect("Failed to GET the update");
        // TODO -- Later this should be streamed directly to the artifact.
        {
            let f = std::fs::File::open("foo.txt").expect("Failed to open the file");
            let mut writer = BufWriter::new(f);
            resp.copy_to(&mut writer);
        }

        let mut f = std::fs::File::open("foo.txt").expect("Failed to open the file");

        let mut ma = mender_artifact::MenderArtifact::new(&mut f);
        let mut payloads = ma.parse("todo");

        let mut entry = payloads.unwrap().next().unwrap().unwrap();
        // Check that the entry base path name is the same as the one we are expecting
        let path = entry.header().path().expect("Failed to get the header");
        if !path.starts_with("data") {
            eprintln!("No data found in artifact");
        }

        // TODO -- Open the actual other partition (Should probably be a seperate module)
        let other_partition = std::fs::File::open("/dev/null").expect("Failed to open '/dev/null'"); // TODO - Write to other partition
        let mut buf_writer = BufWriter::new(other_partition);
        if let Ok(r) = std::io::copy(&mut entry, &mut buf_writer) {
            debug!("Wrote: {:?} bytes to the other partition", r);
        } else {
            warn!("Failed to write to the other partition");
        }


    }
}

impl Client {
    pub fn is_authorized(&self) -> bool {
        return self.is_authorized;
    }
}

#[derive(Serialize)]
struct InventoryAttribute {
    #[serde(rename(deserialize = "name"))]
    name: String,
    #[serde(rename(deserialize = "value"))]
    value: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_authorization() {
        let client = Client::new();
        // assert_eq!(client.authorize(), true);
    }

    #[test]
    fn test_sha256sum() {
        let expected_res = [
            195, 171, 143, 241, 55, 32, 232, 173, 144, 71, 221, 57, 70, 107, 60, 137, 116, 229,
            146, 194, 250, 56, 61, 74, 57, 96, 113, 76, 174, 240, 196, 242,
        ];
        let client = Client::new();
        let hash = client.shasum256_request("foobar".as_bytes());
        assert_eq!(hash, expected_res);
    }

    #[test]
    fn test_request_signing() {
        let expected_res = [
            126, 104, 142, 98, 199, 130, 244, 118, 134, 48, 157, 85, 137, 66, 103, 47, 255, 22,
            174, 41, 39, 230, 249, 80, 68, 11, 71, 31, 117, 172, 141, 236, 101, 188, 145, 242, 1,
            219, 115, 208, 208, 109, 27, 12, 61, 145, 36, 99, 191, 244, 230, 90, 131, 33, 57, 33,
            252, 217, 181, 5, 93, 125, 250, 18, 66, 53, 79, 95, 212, 18, 205, 180, 147, 129, 111,
            243, 102, 78, 77, 164, 200, 200, 170, 170, 215, 198, 241, 50, 243, 119, 63, 115, 45,
            134, 223, 86, 224, 130, 121, 255, 229, 98, 210, 20, 168, 2, 154, 236, 113, 247, 21, 69,
            200, 25, 131, 18, 84, 108, 88, 56, 211, 35, 83, 123, 235, 72, 114, 43, 8, 81, 0, 135,
            131, 18, 72, 85, 16, 131, 186, 170, 208, 24, 199, 78, 87, 19, 248, 61, 86, 184, 16, 0,
            230, 255, 74, 139, 5, 207, 208, 94, 179, 222, 93, 209, 84, 138, 133, 50, 229, 39, 45,
            20, 56, 94, 218, 92, 99, 135, 194, 134, 213, 34, 225, 241, 41, 93, 110, 2, 198, 177,
            170, 151, 240, 123, 206, 45, 120, 111, 70, 37, 59, 186, 76, 172, 101, 40, 237, 140, 49,
            57, 17, 102, 29, 243, 125, 21, 223, 220, 234, 94, 121, 11, 72, 56, 226, 119, 177, 254,
            3, 249, 144, 62, 28, 149, 64, 36, 233, 51, 91, 50, 126, 23, 19, 35, 45, 66, 78, 109,
            53, 96, 151, 1, 175, 148, 53, 91, 82, 217, 238, 68, 209, 248, 59, 177, 223, 3, 99, 168,
            22, 195, 165, 164, 39, 182, 49, 171, 85, 84, 128, 10, 53, 142, 132, 89, 137, 224, 34,
            231, 205, 139, 32, 116, 53, 54, 24, 36, 169, 238, 58, 5, 51, 205, 43, 175, 25, 62, 57,
            14, 200, 164, 72, 174, 152, 165, 68, 103, 180, 47, 82, 194, 138, 102, 105, 214, 247,
            83, 67, 183, 183, 206, 236, 6, 103, 127, 131, 2, 96, 41, 53, 188, 111, 74, 74, 5, 239,
            127, 75, 245, 46, 182, 210, 9, 45, 108, 209, 56, 160, 78, 52, 217, 143, 70, 253, 130,
            107, 71, 219, 230, 188, 184, 38, 62, 91, 124, 81, 163, 211, 37, 162, 87, 19, 23, 214,
            102,
        ];
        let client = Client::new();
        let res = client.sign_request("foobar".as_bytes());
        assert_eq!(res[..384], expected_res[..384]);
        // openssl::rsa::Rsa::dec
    }

    #[test]
    fn test_parse_update_response() {
        let resp_str = r#"{"id":"f4a7b80c-1dd7-415f-a020-834a8c9ce875","artifact":{"artifact_name":"mender-demo-artifact-2.0.0","source":{"uri":"https://s3.docker.mender.io:9000/mender-artifact-storage/5bdd442a-fd0c-4df2-8c0a-317af5eb99fa?X-Amz-Algorithm=AWS4-HMAC-SHA256u0026X-Amz-Content-Sha256=UNSIGNED-PAYLOADu0026X-Amz-Credential=minio%2F20190908%2Fus-east-1%2Fs3%2Faws4_requestu0026X-Amz-Date=20190908T160442Zu0026X-Amz-Expires=86400u0026X-Amz-SignedHeaders=hostu0026response-content-type=application%2Fvnd.mender-artifactu0026X-Amz-Signature=e8898a461ba7dd4e7be85600f00bd36ef17e9102867c82860ba00844e39ee99e","expire":"2019-09-08T16:04:42.6058009Z"},"device_types_compatible":["beaglebone","beaglebone-yocto","beaglebone-yocto-grub","generic-armv6","generic-x86_64","qemux86-64","raspberrypi0w","raspberrypi0-wifi","raspberrypi3"]}}"#;

        let parsed_resp: UpdateInfo = serde_json::from_str(resp_str).expect("Failed to parse update info...");

    }
}
