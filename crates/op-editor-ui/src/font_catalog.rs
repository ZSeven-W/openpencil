//! Shared font-family catalog used by typography widgets and hosts.

/// TS `BUNDLED_FAMILIES` (use-system-fonts.ts:9-21).
pub const BUNDLED_FONT_FAMILIES: [&str; 11] = [
    "Inter",
    "Poppins",
    "Roboto",
    "Montserrat",
    "Open Sans",
    "Lato",
    "Raleway",
    "DM Sans",
    "Playfair Display",
    "Nunito",
    "Source Sans 3",
];

/// TS `FALLBACK_SYSTEM_FONTS` (use-system-fonts.ts:24-36).
pub const FALLBACK_SYSTEM_FONTS: [&str; 11] = [
    "Arial",
    "Helvetica",
    "Helvetica Neue",
    "Georgia",
    "Times New Roman",
    "Courier New",
    "Verdana",
    "Trebuchet MS",
    "Tahoma",
    "Impact",
    "Comic Sans MS",
];
