//! Structure-only interoperability tests for the keyless RFC 10082 PDF/CMS path.
//! Component signatures and the synthetic certificate are NOT cryptographically valid;
//! the external C++ SDK must supply/verify those before production.
#![cfg(feature = "external-composite")]

use cms::{content_info::ContentInfo, signed_data::SignedData};
use der::{
    asn1::{BitString, OctetString},
    Decode, DecodePem, Encode,
};
use underskrift::{
    composite::{extract_composite_verification_inputs, CompositeAlgorithm},
    core::parser,
    finalize_signature, prepare_composite_signature, CompositeSignerInfo, RemoteSigningOptions,
};
use x509_cert::Certificate;

fn synthetic_certificate(algorithm: CompositeAlgorithm) -> Vec<u8> {
    let pem = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/signer_cert.pem"
    ))
    .expect("signer certificate fixture");
    let mut cert = Certificate::from_pem(&pem).expect("fixture must be a valid certificate");
    let spki = &mut cert.tbs_certificate.subject_public_key_info;
    spki.algorithm.oid = algorithm.oid();
    spki.algorithm.parameters = None;
    let mldsa_pk_size = algorithm.mldsa_public_key_len();
    let trad_pk_size = match algorithm {
        CompositeAlgorithm::MlDsa44Ed25519 | CompositeAlgorithm::MlDsa65Ed25519 => 32,
        CompositeAlgorithm::MlDsa87Ed448 => 57,
        CompositeAlgorithm::MlDsa87EcdsaP384 => 97, // Uncompressed SEC1 P-384 point
    };
    spki.subject_public_key =
        BitString::from_bytes(&vec![0xA5; mldsa_pk_size + trad_pk_size]).expect("valid BIT STRING");
    // Mutating the fixture invalidates its certification signature intentionally.
    cert.to_der().unwrap()
}

fn mock_external_signature(algorithm: CompositeAlgorithm) -> Vec<u8> {
    let mut sig = vec![0xA5; algorithm.mldsa_signature_len()];
    match algorithm {
        CompositeAlgorithm::MlDsa44Ed25519 | CompositeAlgorithm::MlDsa65Ed25519 => {
            sig.extend_from_slice(&[0xB6; 64])
        }
        CompositeAlgorithm::MlDsa87Ed448 => sig.extend_from_slice(&[0xB6; 114]),
        CompositeAlgorithm::MlDsa87EcdsaP384 => {
            sig.extend_from_slice(&[0x30, 0x06, 0x02, 0x01, 0x01, 0x02, 0x01, 0x02])
        }
    }
    *sig.last_mut().expect("signature has bytes") = 0;
    sig
}

#[test]
fn external_composite_pdf_cms_roundtrip_for_four_rfc_algorithms() {
    let pdf = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/sample.pdf"
    ))
    .unwrap();
    for algorithm in [
        CompositeAlgorithm::MlDsa44Ed25519,
        CompositeAlgorithm::MlDsa65Ed25519,
        CompositeAlgorithm::MlDsa87Ed448,
        CompositeAlgorithm::MlDsa87EcdsaP384,
    ] {
        let cert = synthetic_certificate(algorithm);
        let identity = CompositeSignerInfo {
            certificate_der: cert.clone(),
            chain_der: vec![cert],
            algorithm,
        };
        let prepared =
            prepare_composite_signature(&pdf, &identity, &RemoteSigningOptions::default())
                .expect("prepare CMS with composite X.509");
        assert_eq!(prepared.attrs_hash.len(), 64);
        let prehash = prepared.attrs_hash.clone();
        let signature = mock_external_signature(algorithm);
        let signed = finalize_signature(prepared, &signature).expect("embed CMS into PDF");
        assert!(signed.starts_with(b"%PDF"));
        assert!(signed.len() > pdf.len());
        let inputs = extract_composite_verification_inputs(&signed).expect("parse Composite CMS");
        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs[0].algorithm, algorithm);
        assert_eq!(inputs[0].attrs_hash, prehash);
        assert_eq!(inputs[0].signature, signature);
        assert!(inputs[0].covers_current_document);
        let doc = lopdf::Document::load_mem(&signed).unwrap();
        let signatures = parser::extract_signatures(&doc).unwrap();
        assert_eq!(signatures.len(), 1);
        let pdf_sig = &signatures[0];
        assert_eq!(pdf_sig.sub_filter, "ETSI.CAdES.detached");

        let content = ContentInfo::from_der(&pdf_sig.contents).expect("decode CMS ContentInfo");
        let signed_data = SignedData::from_der(&content.content.to_der().unwrap())
            .expect("decode CMS SignedData");
        let signer = signed_data.signer_infos.0.iter().next().unwrap();
        assert_eq!(signer.signature_algorithm.oid, algorithm.oid());
        assert!(signer.signature_algorithm.parameters.is_none());
        assert_eq!(signer.digest_alg.oid, algorithm.digest_oid());
        assert!(signer.digest_alg.parameters.is_none());
        assert_eq!(signer.signature.as_bytes(), signature.as_slice());

        let br = pdf_sig.byte_range;
        let content_digest =
            algorithm.digest_parts(&[&signed[br[0]..br[0] + br[1]], &signed[br[2]..br[2] + br[3]]]);
        let attrs = signer.signed_attrs.as_ref().unwrap();
        let md_attr = attrs
            .iter()
            .find(|attr| attr.oid.to_string() == "1.2.840.113549.1.9.4")
            .expect("CMS message-digest");
        let md_der = md_attr.values.iter().next().unwrap().to_der().unwrap();
        let digest = OctetString::from_der(&md_der).unwrap();
        assert_eq!(digest.as_bytes(), content_digest.as_slice());

        let attrs_der = attrs.to_der().unwrap();
        assert_eq!(prehash, algorithm.digest(&attrs_der));
    }
}

#[test]
fn reject_modified_signed_pdf_even_with_structurally_valid_pdf() {
    let pdf = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/sample.pdf"
    ))
    .unwrap();
    let algorithm = CompositeAlgorithm::MlDsa65Ed25519;
    let cert = synthetic_certificate(algorithm);
    let identity = CompositeSignerInfo {
        certificate_der: cert.clone(),
        chain_der: vec![cert],
        algorithm,
    };
    let options = RemoteSigningOptions {
        reason: Some("SecurityReview".to_string()),
        ..RemoteSigningOptions::default()
    };
    let prepared = prepare_composite_signature(&pdf, &identity, &options).unwrap();
    let signed = finalize_signature(prepared, &mock_external_signature(algorithm)).unwrap();
    let offset = signed
        .windows(b"SecurityReview".len())
        .position(|window| window == b"SecurityReview")
        .expect("reason is stored inside signed PDF bytes");
    let mut corrupted = signed.clone();
    corrupted[offset] = b'R';
    let err = extract_composite_verification_inputs(&corrupted).unwrap_err();
    assert!(format!("{err}").contains("content digest mismatch"));

    let mut later_revision = signed;
    later_revision.extend_from_slice(b"\n");
    let entries = extract_composite_verification_inputs(&later_revision).unwrap();
    assert_eq!(entries.len(), 1);
    assert!(!entries[0].covers_current_document);
}

#[test]
fn reject_mismatched_composite_certificate_before_signing() {
    let pdf = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/sample.pdf"
    ))
    .unwrap();
    let certificate_der = synthetic_certificate(CompositeAlgorithm::MlDsa44Ed25519);
    let identity = CompositeSignerInfo {
        certificate_der,
        chain_der: vec![],
        algorithm: CompositeAlgorithm::MlDsa65Ed25519,
    };
    assert!(
        prepare_composite_signature(&pdf, &identity, &RemoteSigningOptions::default()).is_err()
    );
}
