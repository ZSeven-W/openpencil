use crate::{
    apply::validate_document_with_limits, canonical_document_hash, finite::validate_finite,
    typed_images, ApplyLimits, Snapshot, SnapshotError,
};

/// Verify an atomic snapshot before it is installed into editor state.
pub fn verify_snapshot(snapshot: &Snapshot, limits: &ApplyLimits) -> Result<(), SnapshotError> {
    validate_document_with_limits(&snapshot.document, limits)?;
    validate_finite(&snapshot.document).map_err(|_| SnapshotError::NonFiniteNumber)?;
    if typed_images::document_has_external_image_ref(&snapshot.document) {
        return Err(SnapshotError::ExternalImageReference);
    }
    let actual = canonical_document_hash(&snapshot.document)?;
    if actual != snapshot.doc_hash {
        return Err(SnapshotError::HashMismatch {
            expected: snapshot.doc_hash,
            actual,
        });
    }
    Ok(())
}
