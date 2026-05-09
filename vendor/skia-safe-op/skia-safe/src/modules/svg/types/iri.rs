use std::fmt;

use crate::{
    interop::{self, AsStr},
    prelude::*,
};
use skia_bindings as sb;

pub type IriKind = sb::SkSVGIRI_Type;
pub type IriFuncKind = sb::SkSVGFuncIRI_Type;

pub type Iri = Handle<sb::SkSVGIRI>;

impl NativeDrop for sb::SkSVGIRI {
    fn drop(&mut self) {
        unsafe { sb::C_SkSVGIRI_destruct(self) }
    }
}

impl Default for Iri {
    fn default() -> Self {
        Self::construct(|uninitialized| unsafe { sb::C_SkSVGIRI_Construct(uninitialized) })
    }
}

impl fmt::Debug for Iri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SvgIri")
            .field("data", &self.data())
            .finish()
    }
}

impl Iri {
    pub fn data(&self) -> &str {
        self.native().fIRI.as_str()
    }

    pub fn new<T: AsRef<str>>(value: T, kind: IriKind) -> Self {
        Self::construct(|uninitialized| unsafe {
            let iri = interop::String::from_str(value.as_ref());

            sb::C_SkSVGIRI_Construct1(uninitialized, kind, iri.native())
        })
    }
}

pub type IriFunc = Handle<sb::SkSVGFuncIRI>;

impl NativeDrop for sb::SkSVGFuncIRI {
    fn drop(&mut self) {
        unsafe { sb::C_SkSVGFuncIRI_destruct(self) }
    }
}

impl fmt::Debug for IriFunc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SvgIriFunc")
            .field("kind", &self.kind())
            .field("iri", &self.iri())
            .finish()
    }
}

impl IriFunc {
    pub fn iri(&self) -> Option<&Iri> {
        let func = self.native();

        if matches!(func.fType, IriFuncKind::IRI) {
            Some(Iri::from_native_ref(&self.native().fIRI))
        } else {
            None
        }
    }

    pub fn kind(&self) -> IriFuncKind {
        self.native().fType
    }

    pub fn from_iri(value: Iri) -> Self {
        Self::construct(|uninitialized| unsafe {
            sb::C_SkSVGFuncIRI_Construct1(uninitialized, value.native())
        })
    }

    pub fn none() -> Self {
        Self::construct(|uninitialized| unsafe { sb::C_SkSVGFuncIRI_Construct(uninitialized) })
    }
}
