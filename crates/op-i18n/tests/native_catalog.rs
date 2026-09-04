use op_i18n::{translate, Locale};

#[test]
fn runtime_catalog_feature_keeps_native_thai_embedded() {
    let translated = translate(Locale::Th, "common.cancel");
    assert_eq!(translated, "ยกเลิก");
    assert_ne!(translated, "Cancel");
}
