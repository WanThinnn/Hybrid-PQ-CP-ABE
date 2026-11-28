//! C ABI facade for keyless Composite ML-DSA PDF/CMS signing.
#![allow(non_camel_case_types)]

use std::{
    cell::RefCell,
    ffi::{c_char, CString},
    panic::{catch_unwind, AssertUnwindSafe},
    ptr, slice,
};

use underskrift::{
    composite::{extract_composite_verification_inputs, CompositeAlgorithm},
    finalize_signature, prepare_composite_signature, CompositeSignerInfo, PreparedSignature,
    RemoteSigningOptions,
};

pub const PQPDF_OK: i32 = 0;
pub const PQPDF_INVALID_INPUT: i32 = -1;
pub const PQPDF_ERROR: i32 = -2;
pub const PQPDF_PANIC: i32 = -3;
pub const PQPDF_NOT_FOUND: i32 = -4;
const INVALID: i32 = PQPDF_INVALID_INPUT;
const ERROR: i32 = PQPDF_ERROR;
const PANIC: i32 = PQPDF_PANIC;
const NOT_FOUND: i32 = PQPDF_NOT_FOUND;
type Result<T> = std::result::Result<T, (i32, String)>;

thread_local! {
    static LAST_ERROR: RefCell<CString> =
        RefCell::new(CString::new("").expect("empty CString"));
}

fn diagnostic(message: &str) {
    LAST_ERROR.with(|error| {
        *error.borrow_mut() =
            CString::new(message.replace('\0', " ")).expect("sanitized diagnostic contains no NUL");
    });
}

fn operation(f: impl FnOnce() -> Result<()>) -> i32 {
    diagnostic("");
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(())) => 0,
        Ok(Err((code, message))) => {
            diagnostic(&message);
            code
        }
        Err(_) => {
            diagnostic("pqc-underskrift bridge panicked");
            PANIC
        }
    }
}

fn invalid(message: &str) -> (i32, String) {
    (INVALID, message.into())
}
fn failure(error: impl std::fmt::Display) -> (i32, String) {
    (ERROR, error.to_string())
}

fn algorithm(id: i32) -> Result<CompositeAlgorithm> {
    match id {
        0 => Ok(CompositeAlgorithm::MlDsa44Ed25519),
        1 => Ok(CompositeAlgorithm::MlDsa65Ed25519),
        2 => Ok(CompositeAlgorithm::MlDsa87Ed448),
        3 => Ok(CompositeAlgorithm::MlDsa87EcdsaP384),
        _ => Err(invalid("unknown Composite ML-DSA algorithm identifier")),
    }
}
fn algorithm_id(id: CompositeAlgorithm) -> i32 {
    match id {
        CompositeAlgorithm::MlDsa44Ed25519 => 0,
        CompositeAlgorithm::MlDsa65Ed25519 => 1,
        CompositeAlgorithm::MlDsa87Ed448 => 2,
        CompositeAlgorithm::MlDsa87EcdsaP384 => 3,
    }
}

/// # Safety
/// The pointer must reference at least `len` initialized bytes for the call.
unsafe fn bytes<'a>(data: *const u8, len: usize) -> Result<&'a [u8]> {
    if data.is_null() || len == 0 || len > isize::MAX as usize {
        return Err(invalid(
            "input pointer must be non-null and length positive",
        ));
    }
    Ok(slice::from_raw_parts(data, len))
}

/// # Safety
/// The caller must supply a writable empty pqpdf_buffer.
unsafe fn check_buffer(buffer: *mut pqpdf_buffer) -> Result<()> {
    if buffer.is_null() || !(*buffer).data.is_null() || (*buffer).len != 0 {
        return Err(invalid("output buffer must be initialized to {NULL, 0}"));
    }
    Ok(())
}
unsafe fn store(buffer: *mut pqpdf_buffer, bytes: Vec<u8>) {
    let boxed = bytes.into_boxed_slice();
    (*buffer).len = boxed.len();
    (*buffer).data = Box::into_raw(boxed) as *mut u8;
}

#[repr(C)]
#[derive(Debug)]
pub struct pqpdf_buffer {
    pub data: *mut u8,
    pub len: usize,
}

#[repr(C)]
#[derive(Debug)]
pub struct pqpdf_verification {
    pub algorithm: i32,
    pub covers_current_document: i32,
    pub attrs_hash: pqpdf_buffer,
    pub signature: pqpdf_buffer,
    pub certificate_der: pqpdf_buffer,
}

/// Opaque to C callers; only allocated and freed by this bridge.
pub struct pqpdf_prepared {
    inner: PreparedSignature,
    algorithm: CompositeAlgorithm,
}

#[no_mangle]
pub unsafe extern "C" fn pqpdf_prepare(
    pdf: *const u8,
    pdf_len: usize,
    certificate: *const u8,
    certificate_len: usize,
    chain: *const *const u8,
    chain_lens: *const usize,
    chain_count: usize,
    algorithm_code: i32,
    state: *mut *mut pqpdf_prepared,
    attrs_hash: *mut pqpdf_buffer,
) -> i32 {
    operation(|| {
        if state.is_null() || !(*state).is_null() {
            return Err(invalid("state must point to a NULL handle"));
        }
        check_buffer(attrs_hash)?;
        let alg = algorithm(algorithm_code)?;
        let mut cert_chain = Vec::new();
        if chain_count > 0 {
            if chain.is_null()
                || chain_lens.is_null()
                || chain_count > isize::MAX as usize / std::mem::size_of::<usize>()
            {
                return Err(invalid("invalid certificate chain array"));
            }
            for i in 0..chain_count {
                let member = *chain.add(i);
                let length = *chain_lens.add(i);
                cert_chain.push(bytes(member, length)?.to_vec());
            }
        }
        let leaf = bytes(certificate, certificate_len)?.to_vec();
        // The upstream CMS builder encodes certificates from chain_der only.
        // Ensure that the signer certificate is always embedded, even when
        // the caller supplies no chain or supplies only intermediate CAs.
        if !cert_chain.iter().any(|entry| entry == &leaf) {
            cert_chain.insert(0, leaf.clone());
        }
        let signer = CompositeSignerInfo {
            certificate_der: leaf,
            chain_der: cert_chain,
            algorithm: alg,
        };
        let prepared = prepare_composite_signature(
            bytes(pdf, pdf_len)?,
            &signer,
            &RemoteSigningOptions::default(),
        )
        .map_err(failure)?;
        let hash = prepared.attrs_hash.clone();
        let handle = Box::new(pqpdf_prepared {
            inner: prepared,
            algorithm: alg,
        });
        store(attrs_hash, hash);
        *state = Box::into_raw(handle);
        Ok(())
    })
}

/// Consume the handle once, even if CMS serialization fails.
#[no_mangle]
pub unsafe extern "C" fn pqpdf_finalize(
    state: *mut *mut pqpdf_prepared,
    signature: *const u8,
    signature_len: usize,
    signed_pdf: *mut pqpdf_buffer,
) -> i32 {
    operation(|| {
        if state.is_null() || (*state).is_null() {
            return Err(invalid("NULL or already consumed prepare state"));
        }
        check_buffer(signed_pdf)?;
        let signature = bytes(signature, signature_len)?;
        let handle = *state;
        (*handle)
            .algorithm
            .validate_signature(signature)
            .map_err(failure)?;
        *state = ptr::null_mut();
        let prepared = Box::from_raw(handle);
        let pdf = finalize_signature(prepared.inner, signature).map_err(failure)?;
        store(signed_pdf, pdf);
        Ok(())
    })
}

/// Extract certificate, raw signature, and pre-hash for external verification.
#[no_mangle]
pub unsafe extern "C" fn pqpdf_extract(
    pdf: *const u8,
    pdf_len: usize,
    index: usize,
    out: *mut pqpdf_verification,
) -> i32 {
    operation(|| {
        if out.is_null() {
            return Err(invalid("verification output must not be NULL"));
        }
        if !(*out).attrs_hash.data.is_null()
            || (*out).attrs_hash.len != 0
            || !(*out).signature.data.is_null()
            || (*out).signature.len != 0
            || !(*out).certificate_der.data.is_null()
            || (*out).certificate_der.len != 0
        {
            return Err(invalid("verification output must be zero-initialized"));
        }
        let inputs =
            extract_composite_verification_inputs(bytes(pdf, pdf_len)?).map_err(failure)?;
        let selected = inputs
            .get(index)
            .ok_or_else(|| (NOT_FOUND, "Composite signature index not found".into()))?;
        (*out).algorithm = algorithm_id(selected.algorithm);
        (*out).covers_current_document = i32::from(selected.covers_current_document);
        store(&mut (*out).attrs_hash, selected.attrs_hash.clone());
        store(&mut (*out).signature, selected.signature.clone());
        store(
            &mut (*out).certificate_der,
            selected.certificate_der.clone(),
        );
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn pqpdf_prepared_free(state: *mut *mut pqpdf_prepared) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if !state.is_null() && !(*state).is_null() {
            let owned = *state;
            *state = ptr::null_mut();
            drop(Box::from_raw(owned));
        }
    }));
}

#[no_mangle]
pub unsafe extern "C" fn pqpdf_buffer_free(buffer: *mut pqpdf_buffer) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if !buffer.is_null() {
            let len = (*buffer).len;
            let ptr = (*buffer).data;
            (*buffer).len = 0;
            (*buffer).data = ptr::null_mut();
            if !ptr.is_null() {
                drop(Box::from_raw(ptr::slice_from_raw_parts_mut(ptr, len)));
            }
        }
    }));
}

#[no_mangle]
pub unsafe extern "C" fn pqpdf_verification_free(result: *mut pqpdf_verification) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if !result.is_null() {
            pqpdf_buffer_free(&mut (*result).attrs_hash);
            pqpdf_buffer_free(&mut (*result).signature);
            pqpdf_buffer_free(&mut (*result).certificate_der);
            (*result).algorithm = 0;
            (*result).covers_current_document = 0;
        }
    }));
}

#[no_mangle]
pub extern "C" fn pqpdf_last_error() -> *const c_char {
    LAST_ERROR.with(|error| error.borrow().as_ptr())
}
