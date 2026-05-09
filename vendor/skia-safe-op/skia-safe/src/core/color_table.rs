use std::fmt;

use skia_bindings::{self as sb, SkColorTable, SkRefCnt, SkRefCntBase};

use crate::prelude::*;

/// [`ColorTable`] holds the lookup tables for each channel (ARGB) used to define the filter behavior
/// of `SkColorFilters::Table`, and provides a way to share the table data between client code and
/// the returned [`crate::ColorFilter`]. Once created, an [`ColorTable`] is immutable.
pub type ColorTable = RCHandle<SkColorTable>;
unsafe_send_sync!(ColorTable);
require_base_type!(SkColorTable, SkRefCnt);

impl NativeRefCountedBase for SkColorTable {
    type Base = SkRefCntBase;
}

impl fmt::Debug for ColorTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ColorTable").finish()
    }
}

impl ColorTable {
    /// Creates a new [`ColorTable`] with 'table' used for all four channels. The table is copied
    /// into the [`ColorTable`].
    pub fn new(table: &[u8; 256]) -> Self {
        Self::new_per_channel(table, table, table, table).unwrap()
    }

    // Creates a new SkColorTable with the per-channel lookup tables. Each non-null table is copied
    // into the SkColorTable. Null parameters are interpreted as the identity table.
    pub fn new_per_channel<'a>(
        table_a: impl Into<Option<&'a [u8; 256]>>,
        table_r: impl Into<Option<&'a [u8; 256]>>,
        table_g: impl Into<Option<&'a [u8; 256]>>,
        table_b: impl Into<Option<&'a [u8; 256]>>,
    ) -> Option<Self> {
        let table = unsafe {
            sb::C_SkColorTable_Make(
                table_a.into().map(|t| t.as_ref()).as_ptr_or_null(),
                table_r.into().map(|t| t.as_ref()).as_ptr_or_null(),
                table_g.into().map(|t| t.as_ref()).as_ptr_or_null(),
                table_b.into().map(|t| t.as_ref()).as_ptr_or_null(),
            )
        };
        ColorTable::from_ptr(table)
    }

    pub fn alpha_table(&self) -> &[u8; 256] {
        self.get_table(0)
    }

    pub fn red_table(&self) -> &[u8; 256] {
        self.get_table(1)
    }

    pub fn green_table(&self) -> &[u8; 256] {
        self.get_table(2)
    }

    pub fn blue_table(&self) -> &[u8; 256] {
        self.get_table(3)
    }

    fn get_table(&self, i: usize) -> &[u8; 256] {
        unsafe {
            let ptr: *const u8 = sb::C_SkColorTable_getTable(self.native(), i);
            &*(ptr as *const [u8; 256])
        }
    }
}
