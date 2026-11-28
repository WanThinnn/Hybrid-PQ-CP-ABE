//! RFC 10073/10082 Composite ML-DSA metadata for *external* CMS signing.
//! No ML-DSA, EdDSA, ECDSA, RSA or private-key implementation is linked here.
//! The caller's cryptographic SDK signs the RFC 10073 message representative.
use crate::error::PdfSignError;
use const_oid::ObjectIdentifier;
use der::{Decode, Tagged};
use x509_cert::Certificate;

/// Supported LAMPS Composite ML-DSA algorithms (RFC 10073 section 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositeAlgorithm {
    MlDsa44Ed25519,
    MlDsa65Ed25519,
    MlDsa87Ed448,
    MlDsa87EcdsaP384,
}
impl CompositeAlgorithm {
    /// The same OID identifies the signature and the public-key algorithm.
    pub const fn oid(self) -> ObjectIdentifier {
        match self {
            Self::MlDsa44Ed25519 => ObjectIdentifier::new_unwrap("1.3.6.1.5.5.7.6.39"),
            Self::MlDsa65Ed25519 => ObjectIdentifier::new_unwrap("1.3.6.1.5.5.7.6.48"),
            Self::MlDsa87Ed448 => ObjectIdentifier::new_unwrap("1.3.6.1.5.5.7.6.51"),
            Self::MlDsa87EcdsaP384 => ObjectIdentifier::new_unwrap("1.3.6.1.5.5.7.6.49"),
        }
    }
    pub const fn digest_oid(self) -> ObjectIdentifier {
        match self {
            Self::MlDsa87Ed448 => ObjectIdentifier::new_unwrap("2.16.840.1.101.3.4.2.12"),
            _ => ObjectIdentifier::new_unwrap("2.16.840.1.101.3.4.2.3"),
        }
    }
    /// RFC 10073 signature label. C++ uses it when constructing M' and as ML-DSA context.
    pub const fn label(self) -> &'static str {
        match self {
            Self::MlDsa44Ed25519 => "COMPSIG-MLDSA44-Ed25519-SHA512",
            Self::MlDsa65Ed25519 => "COMPSIG-MLDSA65-Ed25519-SHA512",
            Self::MlDsa87Ed448 => "COMPSIG-MLDSA87-Ed448-SHAKE256",
            Self::MlDsa87EcdsaP384 => "COMPSIG-MLDSA87-ECDSA-P384-SHA512",
        }
    }
    pub const fn mldsa_signature_len(self) -> usize {
        match self {
            Self::MlDsa44Ed25519 => 2420,
            Self::MlDsa65Ed25519 => 3309,
            Self::MlDsa87Ed448 | Self::MlDsa87EcdsaP384 => 4627,
        }
    }
    pub const fn mldsa_public_key_len(self) -> usize {
        match self {
            Self::MlDsa44Ed25519 => 1312,
            Self::MlDsa65Ed25519 => 1952,
            Self::MlDsa87Ed448 | Self::MlDsa87EcdsaP384 => 2592,
        }
    }
    /// SHA-512 or SHAKE256 with exactly 64 output bytes, as prescribed by RFC 10082.
    pub fn digest_parts(self, parts: &[&[u8]]) -> Vec<u8> {
        if self == Self::MlDsa87Ed448 {
            use sha3::digest::{ExtendableOutput, Update, XofReader};
            let mut h = sha3::Shake256::default();
            for part in parts {
                h.update(part);
            }
            let mut out = vec![0u8; 64];
            h.finalize_xof().read(&mut out);
            out
        } else {
            use sha2::Digest;
            let mut h = sha2::Sha512::default();
            for part in parts {
                h.update(part);
            }
            h.finalize().to_vec()
        }
    }
    pub fn digest(self, data: &[u8]) -> Vec<u8> {
        self.digest_parts(&[data])
    }
    /// Structural check only. The C++ SDK must perform both component verifications.
    pub fn validate_signature(self, signature: &[u8]) -> Result<(), PdfSignError> {
        let prefix_len = self.mldsa_signature_len();
        if signature.len() < prefix_len {
            return Err(PdfSignError::Configuration(
                "truncated Composite ML-DSA signature".into(),
            ));
        }
        let trad = &signature[prefix_len..];
        let correct = match self {
            Self::MlDsa44Ed25519 | Self::MlDsa65Ed25519 => trad.len() == 64,
            Self::MlDsa87Ed448 => trad.len() == 114,
            Self::MlDsa87EcdsaP384 => {
                // ECDSA signatures in this composite are DER-encoded ECDSA-Sig-Value.
                matches!(der::Any::from_der(trad), Ok(value) if value.tag() == der::Tag::Sequence)
            }
        };
        if !correct {
            return Err(PdfSignError::Configuration(
                "invalid traditional signature component encoding".into(),
            ));
        }
        Ok(())
    }
    /// The certificate SPKI must identify the same Composite algorithm, and
    /// MUST NOT encode AlgorithmIdentifier parameters (RFC 10073/10082).
    pub fn validate_certificate(self, der: &[u8]) -> Result<(), PdfSignError> {
        let certificate = Certificate::from_der(der)
            .map_err(|e| PdfSignError::Configuration(format!("invalid X.509 certificate: {e}")))?;
        let spki = &certificate.tbs_certificate.subject_public_key_info;
        if spki.algorithm.oid != self.oid() || spki.algorithm.parameters.is_some() {
            return Err(PdfSignError::Configuration(
                "certificate SPKI Composite OID/parameters mismatch".into(),
            ));
        }
        let raw = spki.subject_public_key.as_bytes().ok_or_else(|| {
            PdfSignError::Configuration("unaligned composite public key BIT STRING".into())
        })?;
        let minimum = self.mldsa_public_key_len()
            + match self {
                Self::MlDsa44Ed25519 | Self::MlDsa65Ed25519 => 32,
                Self::MlDsa87Ed448 => 57,
                Self::MlDsa87EcdsaP384 => 1,
            };
        if raw.len() < minimum
            || (matches!(
                self,
                Self::MlDsa44Ed25519 | Self::MlDsa65Ed25519 | Self::MlDsa87Ed448
            ) && raw.len() != minimum)
        {
            return Err(PdfSignError::Configuration(
                "invalid Composite SPKI public-key length".into(),
            ));
        }
        Ok(())
    }
}

/// Data for the C++ Composite-ML-DSA.VerifyPreHashed operation.
/// This function checks PDF byte ranges, CMS fields, hashes, and SPKI metadata,
/// but intentionally does NOT claim the signature or certificate chain is trusted.
#[derive(Debug, Clone)]
pub struct CompositeVerificationInput {
    pub algorithm: CompositeAlgorithm,
    pub attrs_hash: Vec<u8>,
    pub signature: Vec<u8>,
    pub certificate_der: Vec<u8>,
    /// False if another PDF revision was appended after this signature.
    pub covers_current_document: bool,
}

impl CompositeAlgorithm {
    pub fn from_oid(oid: ObjectIdentifier) -> Option<Self> {
        [
            Self::MlDsa44Ed25519,
            Self::MlDsa65Ed25519,
            Self::MlDsa87Ed448,
            Self::MlDsa87EcdsaP384,
        ]
        .into_iter()
        .find(|algorithm| algorithm.oid() == oid)
    }
}

/// Parse and validate CMS structure for all Composite ML-DSA PDF signatures.
/// The returned pre-hash and signature MUST still be cryptographically verified
/// by the external Crypto++/liboqs SDK, with RFC 10073 application ctx = "".
pub fn extract_composite_verification_inputs(
    pdf: &[u8],
) -> Result<Vec<CompositeVerificationInput>, PdfSignError> {
    use cms::{
        cert::CertificateChoices,
        content_info::ContentInfo,
        signed_data::{SignedData, SignerIdentifier},
    };
    use der::{asn1::OctetString, Encode};
    let document = lopdf::Document::load_mem(pdf).map_err(crate::error::CoreError::Lopdf)?;
    let signatures = crate::core::parser::extract_signatures(&document)?;
    let mut results = Vec::new();
    for sig in signatures {
        let [offset1, len1, offset2, len2] = sig.byte_range;
        let end1 = offset1.checked_add(len1);
        let end2 = offset2.checked_add(len2);
        if offset1 != 0
            || end1.is_none()
            || end2.is_none()
            || end1.unwrap() > offset2
            || end2.unwrap() > pdf.len()
        {
            return Err(PdfSignError::Configuration(
                "invalid PDF signature ByteRange".into(),
            ));
        }
        let ci = ContentInfo::from_der(&sig.contents)
            .map_err(|e| PdfSignError::Configuration(format!("invalid CMS ContentInfo: {e}")))?;
        if ci.content_type != const_oid::db::rfc5911::ID_SIGNED_DATA {
            return Err(PdfSignError::Configuration("CMS is not SignedData".into()));
        }
        let sd = SignedData::from_der(
            &ci.content
                .to_der()
                .map_err(|e| PdfSignError::Configuration(format!("invalid CMS DER: {e}")))?,
        )
        .map_err(|e| PdfSignError::Configuration(format!("invalid CMS SignedData: {e}")))?;
        let mut signers = sd.signer_infos.0.iter();
        let signer = signers
            .next()
            .ok_or_else(|| PdfSignError::Configuration("missing CMS SignerInfo".into()))?;
        if signers.next().is_some() {
            return Err(PdfSignError::Configuration(
                "multiple CMS signers not supported".into(),
            ));
        }
        let algorithm = match CompositeAlgorithm::from_oid(signer.signature_algorithm.oid) {
            Some(algorithm) => algorithm,
            None => continue, // Ordinary signatures are outside this external Composite verifier.
        };
        if signer.signature_algorithm.parameters.is_some()
            || signer.digest_alg.oid != algorithm.digest_oid()
            || signer.digest_alg.parameters.is_some()
        {
            return Err(PdfSignError::Configuration(
                "RFC 10082 AlgorithmIdentifier mismatch".into(),
            ));
        }
        let signature = signer.signature.as_bytes().to_vec();
        algorithm.validate_signature(&signature)?;
        let cert = sd
            .certificates
            .as_ref()
            .and_then(|set| {
                set.0.iter().find_map(|item| match item {
                    CertificateChoices::Certificate(cert) => {
                        let matches = match &signer.sid {
                            SignerIdentifier::IssuerAndSerialNumber(sid) => {
                                sid.issuer == cert.tbs_certificate.issuer
                                    && sid.serial_number == cert.tbs_certificate.serial_number
                            }
                            _ => false,
                        };
                        if matches {
                            Some(cert)
                        } else {
                            None
                        }
                    }
                    _ => None,
                })
            })
            .ok_or_else(|| {
                PdfSignError::Configuration("matching X.509 signer certificate missing".into())
            })?;
        let certificate_der = cert
            .to_der()
            .map_err(|e| PdfSignError::Configuration(format!("invalid X.509 DER: {e}")))?;
        algorithm.validate_certificate(&certificate_der)?;
        let signed_attrs = signer
            .signed_attrs
            .as_ref()
            .ok_or_else(|| PdfSignError::Configuration("CMS signedAttrs required".into()))?;
        let pdf_digest =
            algorithm.digest_parts(&[&pdf[offset1..end1.unwrap()], &pdf[offset2..end2.unwrap()]]);
        let md_attr = signed_attrs
            .iter()
            .find(|a| a.oid == const_oid::db::rfc5911::ID_MESSAGE_DIGEST)
            .ok_or_else(|| PdfSignError::Configuration("missing CMS message-digest".into()))?;
        let mut values = md_attr.values.iter();
        let md_der = values
            .next()
            .ok_or_else(|| PdfSignError::Configuration("empty CMS message-digest".into()))?
            .to_der()
            .map_err(|e| PdfSignError::Configuration(format!("invalid CMS digest DER: {e}")))?;
        if values.next().is_some() {
            return Err(PdfSignError::Configuration(
                "multiple CMS message-digest values".into(),
            ));
        }
        let md = OctetString::from_der(&md_der).map_err(|e| {
            PdfSignError::Configuration(format!("invalid CMS message-digest encoding: {e}"))
        })?;
        if md.as_bytes() != pdf_digest.as_slice() {
            return Err(PdfSignError::Configuration(
                "PDF content digest mismatch".into(),
            ));
        }
        let attrs_der = signed_attrs.to_der().map_err(|e| {
            PdfSignError::Configuration(format!("invalid CMS signedAttrs DER: {e}"))
        })?;
        results.push(CompositeVerificationInput {
            algorithm,
            attrs_hash: algorithm.digest(&attrs_der),
            signature,
            certificate_der,
            covers_current_document: end2.unwrap() == pdf.len(),
        });
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mappings_follow_rfc_10073_10082() {
        let mapping = [
            (
                CompositeAlgorithm::MlDsa44Ed25519,
                "1.3.6.1.5.5.7.6.39",
                "2.16.840.1.101.3.4.2.3",
            ),
            (
                CompositeAlgorithm::MlDsa65Ed25519,
                "1.3.6.1.5.5.7.6.48",
                "2.16.840.1.101.3.4.2.3",
            ),
            (
                CompositeAlgorithm::MlDsa87Ed448,
                "1.3.6.1.5.5.7.6.51",
                "2.16.840.1.101.3.4.2.12",
            ),
            (
                CompositeAlgorithm::MlDsa87EcdsaP384,
                "1.3.6.1.5.5.7.6.49",
                "2.16.840.1.101.3.4.2.3",
            ),
        ];
        for (alg, oid, digest) in mapping {
            assert_eq!(alg.oid().to_string(), oid);
            assert_eq!(alg.digest_oid().to_string(), digest);
            assert_eq!(alg.digest(b"sample").len(), 64);
        }
        assert_ne!(
            CompositeAlgorithm::MlDsa87Ed448.digest(b"sample"),
            CompositeAlgorithm::MlDsa65Ed25519.digest(b"sample")
        );
    }
    #[test]
    fn rejects_malformed_composite_signatures() {
        let alg = CompositeAlgorithm::MlDsa65Ed25519;
        assert!(alg.validate_signature(&vec![0; 3309 + 64]).is_ok());
        assert!(alg.validate_signature(&vec![0; 3309]).is_err());
        assert!(alg.validate_signature(&vec![0; 3309 + 65]).is_err());
    }
}
