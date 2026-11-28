#ifndef HYBRID_PDF_SIGNATURE_FFI_H
#define HYBRID_PDF_SIGNATURE_FFI_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

enum {
    PQPDF_OK = 0,
    PQPDF_INVALID_INPUT = -1,
    PQPDF_ERROR = -2,
    PQPDF_PANIC = -3,
    PQPDF_NOT_FOUND = -4
};
enum {
    PQPDF_MLDSA44_ED25519 = 0,
    PQPDF_MLDSA65_ED25519 = 1,
    PQPDF_MLDSA87_ED448 = 2,
    PQPDF_MLDSA87_ECDSA_P384 = 3
};

typedef struct pqpdf_prepared pqpdf_prepared;

typedef struct {
    uint8_t *data;
    size_t len;
} pqpdf_buffer;

typedef struct {
    int32_t algorithm;
    int32_t covers_current_document;
    pqpdf_buffer attrs_hash;
    pqpdf_buffer signature;
    pqpdf_buffer certificate_der;
} pqpdf_verification;

/*
 * C ABI for the keyless PDF/CMS engine. No private key ever enters this bridge.
 *
 * All input pointers must be valid for the stated lengths and remain alive
 * throughout their call; nonzero lengths require non-NULL pointers.
 * Outputs must be distinct, initialized to NULL/0; do not alias input memory.
 *
 * Algorithm identifiers correspond to RFC 10073 and RFC 10082.
 * attrs_hash is PH(DER(SignedAttrs)) (64 bytes). The C++ SDK must perform the
 * RFC 10073 SignPreHashed/VerifyPreHashed operation with ctx="" and correct label.
 * Do NOT pass this hash directly to generic OQS_SIG_sign: it is not Composite.
 *
 * Certificate input is DER X.509 whose SPKI declares the selected Composite OID.
 * chain is an optional array of DER certificates; pass NULL and chain_count=0
 * for no chain. chain_lens must have chain_count elements.
 *
 * The returned opaque prepare state must not be shared, copied, or used after
 * finalize or free. Finalize consumes it and nulls *state even on failure.
 * On invalid input, *state is left unchanged unless finalization begins.
 * Call pqpdf_prepared_free(&state) to abandon preparation.
 * Free all output bytes with pqpdf_buffer_free, never free/delete.
 * pqpdf_last_error() returns a thread-local string until the next bridge call
 * on the same thread. Exported functions catch Rust panics.
 */

/* Prepare a PDF for a SINGLE invisible CMS Composite signature. */
int32_t pqpdf_prepare(
    const uint8_t *pdf, size_t pdf_len,
    const uint8_t *certificate_der, size_t certificate_len,
    const uint8_t *const *chain, const size_t *chain_lens, size_t chain_count,
    int32_t algorithm,
    pqpdf_prepared **state,
    pqpdf_buffer *attrs_hash);

/* Consumes and clears *state. signature = raw RFC 10073 Composite signature. */
int32_t pqpdf_finalize(
    pqpdf_prepared **state,
    const uint8_t *signature, size_t signature_len,
    pqpdf_buffer *signed_pdf);

/* Extract structurally validated Composite signing inputs for SDK verification.
 * index is zero-based among Composite CMS signatures (not all PDF signatures).
 * A return of PQPDF_OK means only structural/digest checks passed, NOT valid
 * cryptography, key ownership, PKIX trust, revocation or timestamp status. */
int32_t pqpdf_extract(
    const uint8_t *signed_pdf, size_t pdf_len,
    size_t index, pqpdf_verification *result);

void pqpdf_prepared_free(pqpdf_prepared **state);
void pqpdf_buffer_free(pqpdf_buffer *buffer);
void pqpdf_verification_free(pqpdf_verification *verification);
const char *pqpdf_last_error(void);

#ifdef __cplusplus
}
#endif
#endif
