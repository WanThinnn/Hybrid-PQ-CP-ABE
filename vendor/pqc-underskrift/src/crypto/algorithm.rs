//! Minimal algorithm descriptors for builds using an external cryptographic SDK.
//! The full legacy signing/validation stack remains available behind
//! `legacy-crypto`; this module avoids compiling tsp-ltv and its RSA/ECC backends.
use const_oid::ObjectIdentifier;
use digest::DynDigest;

pub const OID_RSASSA_PSS: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.10");
pub const OID_ED25519: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.3.101.112");
pub const OID_ECDSA_WITH_SHA512: ObjectIdentifier =
    ObjectIdentifier::new_unwrap("1.2.840.10045.4.3.4");
pub const OID_SHA3_256: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.16.840.1.101.3.4.2.8");
pub const OID_SHA3_384: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.16.840.1.101.3.4.2.9");
pub const OID_SHA3_512: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.16.840.1.101.3.4.2.10");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SignatureAlgorithm {
    RsaPkcs1v15,
    RsaPss,
    EcdsaP256,
    EcdsaP384,
    Ed25519,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DigestAlgorithm {
    Sha256,
    Sha384,
    Sha512,
    Sha3_256,
    Sha3_384,
    Sha3_512,
}
impl Default for DigestAlgorithm {
    fn default() -> Self {
        Self::Sha256
    }
}
impl DigestAlgorithm {
    pub fn oid(self) -> ObjectIdentifier {
        match self {
            Self::Sha256 => ObjectIdentifier::new_unwrap("2.16.840.1.101.3.4.2.1"),
            Self::Sha384 => ObjectIdentifier::new_unwrap("2.16.840.1.101.3.4.2.2"),
            Self::Sha512 => ObjectIdentifier::new_unwrap("2.16.840.1.101.3.4.2.3"),
            Self::Sha3_256 => OID_SHA3_256,
            Self::Sha3_384 => OID_SHA3_384,
            Self::Sha3_512 => OID_SHA3_512,
        }
    }
    pub fn is_sha3(self) -> bool {
        matches!(self, Self::Sha3_256 | Self::Sha3_384 | Self::Sha3_512)
    }
    pub fn new_hasher(self) -> Box<dyn DynDigest + Send> {
        match self {
            Self::Sha256 => Box::new(sha2::Sha256::default()),
            Self::Sha384 => Box::new(sha2::Sha384::default()),
            Self::Sha512 => Box::new(sha2::Sha512::default()),
            Self::Sha3_256 => Box::new(sha3::Sha3_256::default()),
            Self::Sha3_384 => Box::new(sha3::Sha3_384::default()),
            Self::Sha3_512 => Box::new(sha3::Sha3_512::default()),
        }
    }
    pub fn digest(self, input: &[u8]) -> Vec<u8> {
        let mut hasher = self.new_hasher();
        hasher.update(input);
        hasher.finalize().to_vec()
    }
}

/// Optional policy gate for ordinary signature algorithms.
#[derive(Debug, Clone, Default)]
pub struct AlgorithmRegistry {
    allowed: std::collections::HashSet<(SignatureAlgorithm, DigestAlgorithm)>,
}
impl AlgorithmRegistry {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn standard() -> Self {
        let mut out = Self::new();
        for alg in [
            SignatureAlgorithm::RsaPkcs1v15,
            SignatureAlgorithm::RsaPss,
            SignatureAlgorithm::EcdsaP256,
            SignatureAlgorithm::EcdsaP384,
            SignatureAlgorithm::Ed25519,
        ] {
            for hash in [
                DigestAlgorithm::Sha256,
                DigestAlgorithm::Sha384,
                DigestAlgorithm::Sha512,
                DigestAlgorithm::Sha3_256,
                DigestAlgorithm::Sha3_384,
                DigestAlgorithm::Sha3_512,
            ] {
                out.allowed.insert((alg, hash));
            }
        }
        out
    }
    pub fn validate(&self, alg: SignatureAlgorithm, digest: DigestAlgorithm) -> Result<(), String> {
        if self.allowed.contains(&(alg, digest)) {
            Ok(())
        } else {
            Err(format!(
                "algorithm combination not allowed: {alg:?} + {digest:?}"
            ))
        }
    }
}
