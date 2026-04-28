use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}

pub struct JwtVerifier {
    decoding_key: DecodingKey,
}

impl JwtVerifier {
    pub fn new(public_key_pem: &str) -> anyhow::Result<Self> {
        let decoding_key = DecodingKey::from_rsa_pem(public_key_pem.as_bytes())?;
        Ok(Self { decoding_key })
    }

    pub fn verify(&self, token: &str) -> anyhow::Result<Claims> {
        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_exp = true;
        
        // Note: You can add issuer/audience validation here if needed
        // validation.set_issuer(&["my-auth-server"]);
        
        let token_data = decode::<Claims>(token, &self.decoding_key, &validation)?;
        Ok(token_data.claims)
    }
}
