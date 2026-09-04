//! Reverse lookup from the first-party lucide d-string catalogue.

use super::icons::Icon;

/// Return the canonical lucide name for one catalogue path d-string.
///
/// The comparison uses the core semantic canonicalizer, so a model may use
/// commas, arbitrary whitespace, relative commands, or implicit line groups
/// without losing a catalogue hit.
pub fn lucide_name_for_path_d(d: &str) -> Option<&'static str> {
    let canonical = op_editor_core::icon_path_normalize::canonicalize_path_d(d)?;
    for &(name, icon) in FIRST_PARTY_ICONS {
        if icon.paths().iter().any(|path| {
            op_editor_core::icon_path_normalize::canonicalize_path_d(path).as_deref()
                == Some(canonical.as_str())
        }) {
            return Some(name);
        }
    }
    None
}

// Keep this list explicit: Icon is also the editor-chrome enum, and not every
// chrome-only/custom glyph is a runtime lucide catalogue entry.
const FIRST_PARTY_ICONS: &[(&str, Icon)] = &[
    ("mouse-pointer-2", Icon::Cursor),
    ("square", Icon::Square),
    ("square-round-corner", Icon::SquareRoundCorner),
    ("chevron-down", Icon::ChevronDown),
    ("chevron-right", Icon::ChevronRight),
    ("type", Icon::Type),
    ("frame", Icon::Frame),
    ("hand", Icon::Hand),
    ("undo-2", Icon::Undo),
    ("redo-2", Icon::Redo),
    ("braces", Icon::Braces),
    ("book-open", Icon::BookOpen),
    ("library", Icon::Library),
    ("plus", Icon::Plus),
    ("minus", Icon::Minus),
    ("search", Icon::Search),
    ("sun", Icon::Sun),
    ("moon", Icon::Moon),
    ("globe", Icon::Globe),
    ("maximize", Icon::Maximize),
    ("minimize-2", Icon::Minimize),
    ("hash", Icon::Hash),
    ("panel-left", Icon::PanelLeft),
    ("folder-open", Icon::FolderOpen),
    ("git-branch", Icon::GitBranch),
    ("history", Icon::History),
    ("file-plus", Icon::FilePlus),
    ("git-fork", Icon::GitFork),
    ("sparkles", Icon::Sparkles),
    ("wand-sparkles", Icon::Wand2),
    ("brain", Icon::Brain),
    ("x", Icon::Close),
    ("trash-2", Icon::Trash),
    ("copy", Icon::Copy),
    ("pencil", Icon::Pencil),
    ("pen", Icon::Pen),
    ("arrow-up", Icon::ArrowUp),
    ("arrow-down", Icon::ArrowDown),
    ("chevron-up", Icon::ChevronUp),
    ("message-square", Icon::MessageSquare),
    ("layout-grid", Icon::LayoutGrid),
    ("rows-3", Icon::Rows3),
    ("columns-3", Icon::Columns3),
    ("rotate-cw", Icon::RotateCw),
    ("diamond", Icon::Diamond),
    ("component", Icon::Component),
    ("unlink", Icon::Unlink),
    ("check", Icon::Check),
    ("arrow-up-right", Icon::ArrowUpRight),
    ("circle", Icon::Circle),
    ("triangle", Icon::Triangle),
    ("pen-tool", Icon::PenTool),
    ("image-plus", Icon::ImagePlus),
    ("eye", Icon::Eye),
    ("eye-off", Icon::EyeOff),
    ("lock", Icon::Lock),
    ("lock-open", Icon::LockOpen),
    ("github", Icon::Github),
    ("bot", Icon::Bot),
    ("terminal", Icon::Terminal),
    ("image", Icon::Image),
    ("settings", Icon::Settings),
    ("wrench", Icon::Wrench),
    ("save", Icon::Save),
    ("download", Icon::Download),
    ("upload", Icon::Upload),
    ("file-text", Icon::FileText),
    ("file-search", Icon::FileSearch),
    ("file-down", Icon::FileDown),
    ("user-x", Icon::UserX),
    ("key", Icon::Key),
    ("log-out", Icon::LogOut),
    ("mail", Icon::Mail),
    ("smartphone", Icon::Smartphone),
    ("chrome", Icon::Chrome),
    ("user", Icon::User),
    ("clock", Icon::Clock),
    ("calendar", Icon::Calendar),
    ("star", Icon::Star),
    ("heart", Icon::Heart),
    ("home", Icon::Home),
    ("bell", Icon::Bell),
    ("play", Icon::Play),
    ("map-pin", Icon::MapPin),
    ("phone", Icon::Phone),
    ("camera", Icon::Camera),
    ("video", Icon::Video),
    ("music", Icon::Music),
    ("share", Icon::Share),
    ("info", Icon::Info),
    ("alert-circle", Icon::AlertCircle),
    ("help-circle", Icon::HelpCircle),
    ("chevron-left", Icon::ChevronLeft),
    ("more-vertical", Icon::MoreVertical),
    ("more-horizontal", Icon::MoreHorizontal),
    ("milestone", Icon::Milestone),
    ("trending-up", Icon::TrendingUp),
    ("trending-down", Icon::TrendingDown),
    ("compass", Icon::Compass),
    ("refresh-cw", Icon::RefreshCw),
    ("layout-dashboard", Icon::LayoutDashboard),
    ("users", Icon::Users),
    ("package", Icon::Package),
    ("zap", Icon::Zap),
    ("sliders-horizontal", Icon::SlidersHorizontal),
    ("activity", Icon::Activity),
    ("loader", Icon::Loader),
    ("focus", Icon::Focus),
    ("chart-line", Icon::ChartLine),
    ("settings-2", Icon::Settings2),
    ("arrow-right", Icon::ArrowRight),
    ("arrow-left", Icon::ArrowLeft),
    ("check-circle", Icon::CheckCircle),
    ("alert-triangle", Icon::AlertTriangle),
    ("alert-octagon", Icon::AlertOctagon),
    ("sticky-note", Icon::StickyNote),
    ("bar-chart-2", Icon::BarChart2),
    ("bold", Icon::Bold),
    ("italic", Icon::Italic),
    ("underline", Icon::Underline),
    ("strikethrough", Icon::Strikethrough),
    ("shopping-cart", Icon::ShoppingCart),
    ("shopping-bag", Icon::ShoppingBag),
    ("send", Icon::Send),
    ("paperclip", Icon::Paperclip),
    ("message-circle", Icon::MessageCircle),
    ("rocket", Icon::Rocket),
    ("menu", Icon::Menu),
    ("credit-card", Icon::CreditCard),
    ("x-circle", Icon::XCircle),
    ("align-left", Icon::AlignLeft),
    ("align-center-vertical", Icon::AlignCenterH),
    ("align-right", Icon::AlignRight),
    ("align-start-horizontal", Icon::AlignTop),
    ("align-center-horizontal", Icon::AlignCenterV),
    ("align-end-horizontal", Icon::AlignBottom),
    ("align-horizontal-distribute-center", Icon::DistributeH),
    ("align-vertical-distribute-center", Icon::DistributeV),
    ("squares-unite", Icon::SquaresUnite),
    ("squares-subtract", Icon::SquaresSubtract),
    ("squares-intersect", Icon::SquaresIntersect),
    ("squares-exclude", Icon::SquaresExclude),
    ("palette", Icon::Palette),
    ("layers", Icon::Layers),
];

#[cfg(test)]
mod tests {
    use super::lucide_name_for_path_d;

    #[test]
    fn reverse_lookup_accepts_equivalent_chevron_spellings() {
        assert_eq!(
            lucide_name_for_path_d("M 6,9 l 6,6 6,-6"),
            Some("chevron-down")
        );
    }
}
