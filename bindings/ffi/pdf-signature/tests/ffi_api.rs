use der::{asn1::BitString, DecodePem, Encode};
use pdf_signature_ffi::*;
use std::ptr;
use x509_cert::Certificate;

fn fake_certificate(alg: i32) -> Vec<u8> {
    let pem = include_bytes!("../../../../vendor/pqc-underskrift/tests/fixtures/signer_cert.pem");
    let mut cert = Certificate::from_pem(pem).unwrap();
    let (oid, pq, classic) = match alg {
        0 => ("1.3.6.1.5.5.7.6.39", 1312, 32),
        1 => ("1.3.6.1.5.5.7.6.48", 1952, 32),
        2 => ("1.3.6.1.5.5.7.6.51", 2592, 57),
        3 => ("1.3.6.1.5.5.7.6.49", 2592, 97),
        _ => panic!("unsupported"),
    };
    cert.tbs_certificate.subject_public_key_info.algorithm.oid = oid.parse().unwrap();
    cert.tbs_certificate
        .subject_public_key_info
        .algorithm
        .parameters = None;
    cert.tbs_certificate
        .subject_public_key_info
        .subject_public_key = BitString::from_bytes(&vec![0x42; pq + classic]).unwrap();
    cert.to_der().unwrap()
}

fn signature(alg: i32) -> Vec<u8> {
    let (len, classic) = match alg {
        0 => (2420, vec![3u8; 64]),
        1 => (3309, vec![3u8; 64]),
        2 => (4627, vec![3u8; 114]),
        3 => (4627, vec![0x30, 0x06, 0x02, 0x01, 0x01, 0x02, 0x01, 0x02]),
        _ => panic!("unsupported"),
    };
    let mut bytes = vec![0x55; len];
    bytes.extend(classic);
    bytes
}
fn empty_buffer() -> pqpdf_buffer {
    pqpdf_buffer {
        data: ptr::null_mut(),
        len: 0,
    }
}
fn empty_verification() -> pqpdf_verification {
    pqpdf_verification {
        algorithm: 0,
        covers_current_document: 0,
        attrs_hash: empty_buffer(),
        signature: empty_buffer(),
        certificate_der: empty_buffer(),
    }
}
unsafe fn copy(b: &pqpdf_buffer) -> Vec<u8> {
    std::slice::from_raw_parts(b.data, b.len).to_vec()
}
#[test]
fn ffi_prepare_finalize_extract_every_composite_algorithm() {
    let pdf = include_bytes!("../../../../vendor/pqc-underskrift/tests/fixtures/sample.pdf");
    for alg in 0..4 {
        let cert = fake_certificate(alg);
        let mut state: *mut pqpdf_prepared = ptr::null_mut();
        let mut hash = empty_buffer();
        let status = unsafe {
            pqpdf_prepare(
                pdf.as_ptr(),
                pdf.len(),
                cert.as_ptr(),
                cert.len(),
                ptr::null(),
                ptr::null(),
                0,
                alg,
                &mut state,
                &mut hash,
            )
        };
        assert_eq!(status, PQPDF_OK, "prepare failed for algorithm {alg}");
        assert!(!state.is_null());
        let digest = unsafe { copy(&hash) };
        assert_eq!(digest.len(), 64);
        unsafe { pqpdf_buffer_free(&mut hash) };
        assert!(hash.data.is_null());
        let raw_sig = signature(alg);
        let mut signed = empty_buffer();
        assert_eq!(
            unsafe { pqpdf_finalize(&mut state, raw_sig.as_ptr(), raw_sig.len(), &mut signed) },
            PQPDF_OK,
        );
        assert!(state.is_null());
        let signed_bytes = unsafe { copy(&signed) };
        assert!(signed_bytes.len() > pdf.len());
        let mut extracted = empty_verification();
        let extract_status = unsafe { pqpdf_extract(signed.data, signed.len, 0, &mut extracted) };
        assert_eq!(extract_status, PQPDF_OK, "algorithm {alg}: {}", unsafe {
            std::ffi::CStr::from_ptr(pqpdf_last_error()).to_string_lossy()
        });
        assert_eq!(extracted.algorithm, alg);
        assert_eq!(extracted.covers_current_document, 1);
        assert_eq!(unsafe { copy(&extracted.attrs_hash) }, digest);
        assert_eq!(unsafe { copy(&extracted.signature) }, raw_sig);
        assert_eq!(unsafe { copy(&extracted.certificate_der) }, cert);
        unsafe {
            pqpdf_verification_free(&mut extracted);
            pqpdf_buffer_free(&mut signed);
        }
        assert!(extracted.signature.data.is_null());
        assert!(signed.data.is_null());
    }
}

#[test]
fn invalid_arguments_and_abandoning_handle() {
    let pdf = include_bytes!("../../../../vendor/pqc-underskrift/tests/fixtures/sample.pdf");
    let cert = fake_certificate(1);
    let mut state: *mut pqpdf_prepared = ptr::null_mut();
    let mut attrs_hash = empty_buffer();
    assert_eq!(
        unsafe {
            pqpdf_prepare(
                pdf.as_ptr(),
                pdf.len(),
                cert.as_ptr(),
                cert.len(),
                ptr::null(),
                ptr::null(),
                0,
                13,
                &mut state,
                &mut attrs_hash,
            )
        },
        PQPDF_INVALID_INPUT
    );
    assert!(state.is_null());
    assert_eq!(
        unsafe {
            pqpdf_prepare(
                pdf.as_ptr(),
                pdf.len(),
                cert.as_ptr(),
                cert.len(),
                ptr::null(),
                ptr::null(),
                0,
                1,
                &mut state,
                &mut attrs_hash,
            )
        },
        PQPDF_OK
    );
    let invalid_sig = [0u8; 2];
    let mut out = empty_buffer();
    assert_eq!(
        unsafe {
            pqpdf_finalize(
                &mut state,
                invalid_sig.as_ptr(),
                invalid_sig.len(),
                &mut out,
            )
        },
        PQPDF_ERROR
    );
    // Invalid signature does not consume the opaque handle, so caller can retry.
    assert!(!state.is_null());
    unsafe {
        pqpdf_prepared_free(&mut state);
        pqpdf_buffer_free(&mut attrs_hash);
    }
    assert!(state.is_null());
    assert_eq!(
        unsafe { pqpdf_extract(pdf.as_ptr(), pdf.len(), 0, &mut empty_verification()) },
        PQPDF_NOT_FOUND
    );
}

#[test]
fn tampering_with_pdf_is_rejected_before_crypto_verify() {
    let pdf = include_bytes!("../../../../vendor/pqc-underskrift/tests/fixtures/sample.pdf");
    let cert = fake_certificate(1);
    let mut state: *mut pqpdf_prepared = ptr::null_mut();
    let mut attrs_hash = empty_buffer();
    assert_eq!(
        unsafe {
            pqpdf_prepare(
                pdf.as_ptr(),
                pdf.len(),
                cert.as_ptr(),
                cert.len(),
                ptr::null(),
                ptr::null(),
                0,
                1,
                &mut state,
                &mut attrs_hash,
            )
        },
        PQPDF_OK
    );
    let raw = signature(1);
    let mut signed = empty_buffer();
    assert_eq!(
        unsafe { pqpdf_finalize(&mut state, raw.as_ptr(), raw.len(), &mut signed) },
        PQPDF_OK
    );
    let mut changed = unsafe { copy(&signed) };
    // Change a byte in the original PDF comment; PDF remains parsable.
    changed[5] ^= 1;
    let mut result = empty_verification();
    assert_eq!(
        unsafe { pqpdf_extract(changed.as_ptr(), changed.len(), 0, &mut result) },
        PQPDF_ERROR
    );
    unsafe {
        pqpdf_buffer_free(&mut attrs_hash);
        pqpdf_buffer_free(&mut signed)
    };
}
