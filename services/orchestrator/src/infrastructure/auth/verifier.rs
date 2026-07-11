use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}

pub struct JwtVerifier {
    decoding_key: DecodingKey,
    algorithm: Algorithm,
}

impl JwtVerifier {
    pub fn new(public_key_pem: &str) -> anyhow::Result<Self> {
        let pem_bytes = public_key_pem.as_bytes();

        let (decoding_key, algorithm) = if public_key_pem.contains("BEGIN RSA") {
            (
                DecodingKey::from_rsa_pem(pem_bytes)?,
                Algorithm::RS256,
            )
        } else {
            (
                DecodingKey::from_ed_pem(pem_bytes)?,
                Algorithm::EdDSA,
            )
        };

        Ok(Self {
            decoding_key,
            algorithm,
        })
    }

    pub fn verify(&self, token: &str) -> anyhow::Result<Claims> {
        let mut validation = Validation::new(self.algorithm);
        validation.validate_exp = true;

        let token_data = decode::<Claims>(token, &self.decoding_key, &validation)?;
        Ok(token_data.claims)
    }
}
