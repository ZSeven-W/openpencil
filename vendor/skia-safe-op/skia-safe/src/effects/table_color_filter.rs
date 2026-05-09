use crate::{color_filters, ColorFilter};

impl ColorFilter {
    #[deprecated(since = "0.56.0", note = "Use color_filters::table()")]
    pub fn from_table(table: &[u8; 256]) -> Self {
        color_filters::table(table).unwrap()
    }

    #[deprecated(since = "0.56.0", note = "Use color_filters::table_argb()")]
    pub fn from_argb(
        table_a: Option<&[u8; 256]>,
        table_r: Option<&[u8; 256]>,
        table_g: Option<&[u8; 256]>,
        table_b: Option<&[u8; 256]>,
    ) -> Self {
        color_filters::table_argb(table_a, table_r, table_g, table_b).unwrap()
    }
}

#[deprecated(since = "0.56.0", note = "Use color_filters::table()")]
pub fn from_table(table: &[u8; 256]) -> ColorFilter {
    color_filters::table(table).unwrap()
}

#[deprecated(since = "0.56.0", note = "Use color_filters::table_argb()")]
pub fn from_argb(
    table_a: Option<&[u8; 256]>,
    table_r: Option<&[u8; 256]>,
    table_g: Option<&[u8; 256]>,
    table_b: Option<&[u8; 256]>,
) -> ColorFilter {
    color_filters::table_argb(table_a, table_r, table_g, table_b).unwrap()
}
