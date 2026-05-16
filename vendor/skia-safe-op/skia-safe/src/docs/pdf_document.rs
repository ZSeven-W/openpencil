pub mod pdf {
    use std::{ffi::CString, fmt, io, marker::PhantomData, mem, ptr};

    use skia_bindings::{
        self as sb, SkPDF_AttributeList, SkPDF_DateTime, SkPDF_Metadata, SkPDF_StructureElementNode,
    };

    use crate::{
        interop::{AsStr, RustWStream, SetStr},
        prelude::*,
        scalar, Canvas, Document, MILESTONE,
    };

    #[repr(transparent)]
    pub struct AttributeList<'a>(Handle<SkPDF_AttributeList>, PhantomData<&'a ()>);
    unsafe_send_sync!(AttributeList<'_>);

    impl NativeDrop for SkPDF_AttributeList {
        fn drop(&mut self) {
            unsafe { sb::C_SkPDF_AttributeList_destruct(self) }
        }
    }

    impl Default for AttributeList<'_> {
        fn default() -> Self {
            Self(
                Handle::from_native_c(unsafe { SkPDF_AttributeList::new() }),
                PhantomData,
            )
        }
    }

    impl fmt::Debug for AttributeList<'_> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("AttributeList").finish()
        }
    }

    /// Attributes for nodes in the PDF tree.
    ///
    /// Each attribute must have an owner (e.g. "Layout", "List", "Table", etc)
    /// and an attribute name (e.g. "BBox", "RowSpan", etc.) from PDF32000_2008 14.8.5,
    /// and then a value of the proper type according to the spec.
    ///
    /// Parameters of type [`CString`] will not be copied and the pointers must remain valid until
    /// [`crate::Document::close`] or [`crate::Document::abort`] is called.
    /// Names are expected to be constants and are taken as [`CString`].
    /// Parameters not taken as [`CString`] will be copied.
    impl<'a> AttributeList<'a> {
        pub fn append_int(
            &mut self,
            owner: &'a CString,
            name: &'a CString,
            value: i32,
        ) -> &mut Self {
            unsafe {
                self.0
                    .native_mut()
                    .appendInt(owner.as_ptr(), name.as_ptr(), value)
            }
            self
        }

        pub fn append_float(
            &mut self,
            owner: &'a CString,
            name: &'a CString,
            value: f32,
        ) -> &mut Self {
            unsafe {
                self.0
                    .native_mut()
                    .appendFloat(owner.as_ptr(), name.as_ptr(), value)
            }
            self
        }

        pub fn append_float_array(
            &mut self,
            owner: &'a CString,
            name: &'a CString,
            value: &[f32],
        ) -> &mut Self {
            unsafe {
                sb::C_SkPDF_AttributeList_appendFloatArray(
                    self.0.native_mut(),
                    owner.as_ptr(),
                    name.as_ptr(),
                    value.as_ptr(),
                    value.len(),
                )
            }
            self
        }

        pub fn append_name(
            &mut self,
            owner: &'a CString,
            name: &'a CString,
            value: &'a CString,
        ) -> &mut Self {
            unsafe {
                sb::C_SkPDF_AttributeList_appendName(
                    self.0.native_mut(),
                    owner.as_ptr(),
                    name.as_ptr(),
                    value.as_ptr(),
                )
            }
            self
        }

        pub fn append_text_string(
            &mut self,
            owner: &'a CString,
            name: &'a CString,
            value: &'a CString,
        ) -> &mut Self {
            unsafe {
                sb::C_SkPDF_AttributeList_appendTextString(
                    self.0.native_mut(),
                    owner.as_ptr(),
                    name.as_ptr(),
                    value.as_ptr(),
                )
            }
            self
        }

        pub fn append_node_id_array(
            &mut self,
            owner: &'a CString,
            name: &'a CString,
            node_ids: &[i32],
        ) -> &mut Self {
            unsafe {
                sb::C_SkPDF_AttributeList_appendNodeIdArray(
                    self.0.native_mut(),
                    owner.as_ptr(),
                    name.as_ptr(),
                    node_ids.as_ptr(),
                    node_ids.len(),
                )
            }
            self
        }
    }

    #[repr(transparent)]
    pub struct StructureElementNode<'a>(
        ptr::NonNull<SkPDF_StructureElementNode>,
        PhantomData<&'a ()>,
    );

    impl NativeAccess for StructureElementNode<'_> {
        type Native = SkPDF_StructureElementNode;

        fn native(&self) -> &SkPDF_StructureElementNode {
            unsafe { self.0.as_ref() }
        }
        fn native_mut(&mut self) -> &mut SkPDF_StructureElementNode {
            unsafe { self.0.as_mut() }
        }
    }

    impl Drop for StructureElementNode<'_> {
        fn drop(&mut self) {
            unsafe { sb::C_SkPDF_StructureElementNode_delete(self.native_mut()) }
        }
    }

    impl Default for StructureElementNode<'_> {
        fn default() -> Self {
            Self(
                ptr::NonNull::new(unsafe { sb::C_SkPDF_StructureElementNode_new() }).unwrap(),
                PhantomData,
            )
        }
    }

    impl fmt::Debug for StructureElementNode<'_> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("StructureElementNode")
                .field("type_string", &self.type_string())
                .field("child_vector", &self.child_vector())
                .field("node_id", &self.node_id())
                .field("attributes", &self.attributes())
                .field("alt", &self.alt())
                .field("lang", &self.lang())
                .finish()
        }
    }

    /// A node in a PDF structure tree, giving a semantic representation
    /// of the content.  Each node ID is associated with content
    /// by passing the [`crate::Canvas`] and node ID to [`set_node_id()`] when drawing.
    /// NodeIDs should be unique within each tree.
    impl<'a> StructureElementNode<'a> {
        pub fn new(type_string: impl AsRef<str>) -> Self {
            let mut node = Self::default();
            node.set_type_string(type_string);
            node
        }

        pub fn set_type_string(&mut self, type_string: impl AsRef<str>) -> &mut Self {
            self.native_mut().fTypeString.set_str(type_string);
            self
        }

        pub fn type_string(&self) -> &str {
            self.native().fTypeString.as_str()
        }

        pub fn set_child_vector(
            &mut self,
            mut child_vector: Vec<StructureElementNode>,
        ) -> &mut Self {
            // strategy is to move them out by setting them to nullptr (drop() will handle a nullptr on the rust side)
            unsafe {
                sb::C_SkPDF_StructureElementNode_setChildVector(
                    self.native_mut(),
                    child_vector.as_mut_ptr() as _,
                    child_vector.len(),
                )
            }
            self
        }

        pub fn append_child(&mut self, node: StructureElementNode) -> &mut Self {
            unsafe {
                sb::C_SkPDF_StructElementNode_appendChild(self.native_mut(), node.0.as_ptr());
            }
            mem::forget(node);
            self
        }

        pub fn child_vector(&self) -> &[StructureElementNode] {
            let mut ptr = ptr::null();
            unsafe {
                let len = sb::C_SkPDF_StructureElementNode_getChildVector(self.native(), &mut ptr);
                safer::from_raw_parts(ptr as _, len)
            }
        }

        pub fn set_node_id(&mut self, node_id: i32) -> &mut Self {
            self.native_mut().fNodeId = node_id;
            self
        }

        pub fn node_id(&self) -> i32 {
            self.native().fNodeId
        }

        pub fn attributes(&self) -> &AttributeList<'a> {
            unsafe { transmute_ref(Handle::from_native_ref(&self.native().fAttributes)) }
        }

        pub fn attributes_mut(&mut self) -> &mut AttributeList<'a> {
            unsafe {
                transmute_ref_mut(Handle::from_native_ref_mut(
                    &mut self.native_mut().fAttributes,
                ))
            }
        }

        pub fn set_alt(&mut self, alt: impl AsRef<str>) -> &mut Self {
            self.native_mut().fAlt.set_str(alt);
            self
        }

        pub fn alt(&self) -> &str {
            self.native().fAlt.as_str()
        }

        pub fn set_lang(&mut self, lang: impl AsRef<str>) -> &mut Self {
            self.native_mut().fLang.set_str(lang);
            self
        }

        pub fn lang(&self) -> &str {
            self.native().fLang.as_str()
        }
    }

    #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
    #[repr(C)]
    pub struct DateTime {
        /// The number of minutes that this is ahead of or behind UTC.
        pub time_zone_minutes: i16,
        /// e.g. 2005
        pub year: u16,
        /// 1..12
        pub month: u8,
        /// 0..6, 0==Sunday
        pub day_of_week: u8,
        /// 1..31
        pub day: u8,
        /// 0..23
        pub hour: u8,
        /// 0..59
        pub minute: u8,
        /// 0..59
        pub second: u8,
    }

    native_transmutable!(SkPDF_DateTime, DateTime);

    /// Optional metadata to be passed into the PDF factory function.
    #[derive(Debug)]
    pub struct Metadata<'a> {
        /// The document's title.
        pub title: String,
        /// The name of the person who created the document.
        pub author: String,
        /// The subject of the document.
        pub subject: String,
        /// Keywords associated with the document. Commas may be used to delineate keywords within
        /// the string.
        pub keywords: String,
        /// If the document was converted to PDF from another format, the name of the conforming
        /// product that created the original document from which it was converted.
        pub creator: String,
        /// The product that is converting this document to PDF.
        pub producer: String,
        /// The date and time the document was created. The zero default value represents an
        /// unknown/unset time.
        pub creation: Option<DateTime>,
        /// The date and time the document was most recently modified. The zero default value
        /// represents an unknown/unset time.
        pub modified: Option<DateTime>,
        /// The natural language of the text in the PDF. If `lang` is empty, the root
        /// [`StructureElementNode`]'s `lang` will be used (if not empty). Text not in this
        /// language should be marked with [`StructureElementNode`]'s `lang`.
        pub lang: String,
        /// The DPI (pixels-per-inch) at which features without native PDF support will be
        /// rasterized (e.g. draw image with perspective, draw text with perspective, ...) A larger
        /// DPI would create a PDF that reflects the original intent with better fidelity, but it
        /// can make for larger PDF files too, which would use more memory while rendering, and it
        /// would be slower to be processed or sent online or to printer.
        pub raster_dpi: Option<scalar>,
        /// If `true`, include XMP metadata, a document UUID, and sRGB output intent information.
        /// This adds length to the document and makes it non-reproducible, but are necessary
        /// features for PDF/A-2b conformance.
        pub pdf_a: bool,
        /// Encoding quality controls the trade-off between size and quality. By default this is
        /// set to 101 percent, which corresponds to lossless encoding. If this value is set to a
        /// value <= 100, and the image is opaque, it will be encoded (using JPEG) with that
        /// quality setting.
        pub encoding_quality: Option<i32>,

        /// An optional tree of structured document tags that provide a semantic representation of
        /// the content. The caller should retain ownership.
        pub structure_element_tree_root: Option<StructureElementNode<'a>>,

        pub outline: Outline,

        /// PDF streams may be compressed to save space. Use this to specify the desired
        /// compression vs time tradeoff.
        pub compression_level: CompressionLevel,
    }

    impl Default for Metadata<'_> {
        fn default() -> Self {
            Self {
                title: Default::default(),
                author: Default::default(),
                subject: Default::default(),
                keywords: Default::default(),
                creator: Default::default(),
                producer: format!("Skia/PDF m{MILESTONE}"),
                creation: Default::default(),
                modified: Default::default(),
                lang: Default::default(),
                raster_dpi: Default::default(),
                pdf_a: Default::default(),
                encoding_quality: Default::default(),
                structure_element_tree_root: None,
                outline: Outline::None,
                compression_level: Default::default(),
            }
        }
    }

    pub type Outline = skia_bindings::SkPDF_Metadata_Outline;
    variant_name!(Outline::StructureElements);

    pub type CompressionLevel = skia_bindings::SkPDF_Metadata_CompressionLevel;
    variant_name!(CompressionLevel::HighButSlow);

    /// Create a PDF-backed document, writing the results into a writer.
    ///
    /// PDF pages are sized in point units. 1 pt == 1/72 inch == 127/360 mm.
    ///
    /// - `writer` - A PDF document will be written to this writer. The document may write
    ///   to the writer at anytime during its lifetime, until either [`crate::Document::close`] is
    ///   called or the document is deleted.
    /// - `metadata` - a PDF metadata object. Some fields may be left empty.
    ///
    /// Returns `None` if there is an error, otherwise a newly created PDF-backed [`Document`].
    pub fn new_document<'a>(
        writer: &'a mut impl io::Write,
        // We need to make the metadata alive as long as the document, because of `structure_element_tree_root`.
        metadata: Option<&'a Metadata<'a>>,
    ) -> Document<'a> {
        let mut md = InternalMetadata::default();
        if let Some(metadata) = metadata {
            let internal = md.native_mut();
            internal.fTitle.set_str(&metadata.title);
            internal.fAuthor.set_str(&metadata.author);
            internal.fSubject.set_str(&metadata.subject);
            internal.fKeywords.set_str(&metadata.keywords);
            internal.fCreator.set_str(&metadata.creator);
            internal.fProducer.set_str(&metadata.producer);
            if let Some(creation) = metadata.creation {
                internal.fCreation = creation.into_native();
            }
            if let Some(modified) = metadata.modified {
                internal.fModified = modified.into_native();
            }
            internal.fLang.set_str(&metadata.lang);
            if let Some(raster_dpi) = metadata.raster_dpi {
                internal.fRasterDPI = raster_dpi;
            }
            internal.fPDFA = metadata.pdf_a;
            if let Some(encoding_quality) = metadata.encoding_quality {
                internal.fEncodingQuality = encoding_quality
            }
            if let Some(structure_element_tree) = &metadata.structure_element_tree_root {
                internal.fStructureElementTreeRoot = structure_element_tree.0.as_ptr();
            }
            internal.fOutline = metadata.outline;
            internal.fCompressionLevel = metadata.compression_level
        }

        // We enable harfbuzz font sub-setting in PDF documents if textlayout is enabled.
        #[cfg(all(feature = "textlayout", feature = "embed-icudtl"))]
        crate::icu::init();

        let mut stream = RustWStream::new(writer);
        let document = RCHandle::from_ptr(unsafe {
            sb::C_SkPDF_MakeDocument(stream.stream_mut(), md.native())
        })
        .unwrap();

        Document::new(stream, document)
    }

    //
    // Helper for constructing the internal metadata struct and setting associated strings.
    //

    type InternalMetadata = Handle<SkPDF_Metadata>;

    impl NativeDrop for SkPDF_Metadata {
        fn drop(&mut self) {
            unsafe { sb::C_SkPDF_Metadata_destruct(self) }
        }
    }

    impl Default for Handle<SkPDF_Metadata> {
        fn default() -> Self {
            Self::construct(|pdf_md| unsafe { sb::C_SkPDF_Metadata_Construct(pdf_md) })
        }
    }

    pub mod node_id {
        pub const NOTHING: i32 = 0;
        pub const OTHER_ARTIFACT: i32 = -1;
        pub const PAGINATION_ARTIFACT: i32 = -2;
        pub const PAGINATION_HEADER_ARTIFACT: i32 = -3;
        pub const PAGINATION_FOOTER_ARTIFACT: i32 = -4;
        pub const PAGINATION_WATERMARK_ARTIFACT: i32 = -5;
        pub const LAYOUT_ARTIFACT: i32 = -6;
        pub const PAGE_ARTIFACT: i32 = -7;
        pub const BACKGROUND_ARTIFACT: i32 = -8;
    }

    /// Associate a node ID with subsequent drawing commands in a [`Canvas`].
    ///
    /// The same node ID can appear in a [`StructureElementNode`] in order to associate
    /// a document's structure element tree with its content.
    ///
    /// A node ID of zero indicates no node ID. Negative node IDs are reserved.
    ///
    /// - `canvas` - The canvas used to draw to the PDF.
    /// - `node_id` - The node ID for subsequent drawing commands.
    pub fn set_node_id(canvas: &Canvas, node_id: i32) {
        unsafe {
            sb::C_SkPDF_SetNodeId(canvas.native_mut(), node_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::CString;

    use crate::{
        pdf::{self, Metadata, StructureElementNode},
        Color, Paint, Rect,
    };

    #[test]
    fn create_attribute_list() {
        let mut _al = pdf::AttributeList::default();
        let owner = CString::new("Owner").unwrap();
        let name = CString::new("Name").unwrap();
        _al.append_float_array(&owner, &name, &[1.0, 2.0, 3.0]);
    }

    #[test]
    fn structure_element_node_child_vector() {
        let mut root = StructureElementNode::new("root");
        root.append_child(StructureElementNode::new("nested"));
        root.append_child(StructureElementNode::new("nested2"));
        let v = root.child_vector();
        assert_eq!(v[0].type_string(), "nested");
        assert_eq!(v[1].type_string(), "nested2");
    }

    #[test]
    fn generate_pdf_with_structure_and_attributes() {
        // String storage - must outlive the PDF document
        let layout_owner = CString::new("Layout").unwrap();
        let bbox_name = CString::new("BBox").unwrap();
        let table_owner = CString::new("Table").unwrap();
        let colspan_name = CString::new("ColSpan").unwrap();
        let rowspan_name = CString::new("RowSpan").unwrap();

        // Create structure tree with attributes
        let mut root = StructureElementNode::new("Document");
        root.set_node_id(1);

        // Add a paragraph element with bounding box attribute
        let mut paragraph = StructureElementNode::new("P");
        paragraph.set_node_id(2);
        paragraph.attributes_mut().append_float_array(
            &layout_owner,
            &bbox_name,
            &[10.0, 10.0, 200.0, 50.0],
        );

        // Add a table element with column/row span attributes
        let mut table = StructureElementNode::new("Table");
        table.set_node_id(3);
        table
            .attributes_mut()
            .append_int(&table_owner, &colspan_name, 2)
            .append_int(&table_owner, &rowspan_name, 3);

        root.append_child(paragraph);
        root.append_child(table);

        // Create metadata with structure tree
        let metadata = Metadata {
            title: "Test Document with Structure".to_string(),
            author: "Rust Skia Test".to_string(),
            structure_element_tree_root: Some(root),
            ..Default::default()
        };

        // Generate PDF
        let mut output = Vec::new();
        let document = pdf::new_document(&mut output, Some(&metadata));

        // Draw content on first page
        let mut page = document.begin_page((200, 200), None);
        let canvas = page.canvas();

        // Mark the paragraph content area
        pdf::set_node_id(canvas, 2);
        let mut paint = Paint::default();
        paint.set_color(Color::from_rgb(100, 150, 200));
        canvas.draw_rect(Rect::from_xywh(10.0, 10.0, 190.0, 40.0), &paint);

        // Mark the table content area
        pdf::set_node_id(canvas, 3);
        paint.set_color(Color::from_rgb(200, 150, 100));
        canvas.draw_rect(Rect::from_xywh(10.0, 60.0, 190.0, 130.0), &paint);

        let document = page.end_page();
        document.close();

        // Verify PDF was generated
        assert!(!output.is_empty(), "PDF output should not be empty");

        // Basic PDF format validation
        assert!(
            output.starts_with(b"%PDF-"),
            "Output should start with PDF header"
        );
        assert!(
            output.windows(5).any(|w| w == b"%%EOF"),
            "Output should contain EOF marker"
        );

        // Check for structure-related content in the PDF
        let pdf_str = String::from_utf8_lossy(&output);
        assert!(
            pdf_str.contains("/StructTreeRoot") || pdf_str.contains("Document"),
            "PDF should contain structure tree references"
        );
    }
}
