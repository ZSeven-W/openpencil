//! Short Chinese catalogue names for people comparing design directions.
//! These describe the pinned style, not an assessment of generated quality.

use op_editor_core::Locale;

pub(super) fn label(id: &str, locale: Locale) -> Option<&'static str> {
    let traditional = match locale {
        Locale::ZhCn => false,
        Locale::ZhTw => true,
        _ => return None,
    };
    let (simplified, traditional_label) = match id {
        "agency-editorial-light" => ("清爽杂志", "清爽雜誌"),
        "ai-product-dark" => ("深色科技", "深色科技"),
        "arcade-neon-dark" => ("霓虹街机", "霓虹街機"),
        "banxin-rule" => ("版心书页", "版心書頁"),
        "bauhaus-geometric-light" => ("包豪斯几何", "包豪斯幾何"),
        "broadsheet-lime-light" => ("青柠大报", "青檸大報"),
        "brutalist-luxury-dark" => ("粗粝奢华", "粗糲奢華"),
        "butter-serif-light" => ("奶油衬线", "奶油襯線"),
        "clean-blue-mobile-light" => ("清爽蓝白", "清爽藍白"),
        "corporate-blue-light" => ("商务蓝白", "商務藍白"),
        "creative-bold-light" => ("大胆创意", "大膽創意"),
        "crypto-dark-bold" => ("深色数字金融", "深色數位金融"),
        "cyber-gradient-dark" => ("赛博渐变", "賽博漸層"),
        "dark-bold-mobile" => ("深色大字", "深色大字"),
        "dashboard-analytics-dark" => ("深色数据面板", "深色數據面板"),
        "developer-terminal-dark" => ("开发者终端", "開發者終端"),
        "dossier-linen" => ("亚麻档案", "亞麻檔案"),
        "ecommerce-modern-light" => ("现代电商", "現代電商"),
        "editorial-orange-light" => ("橙色杂志", "橙色雜誌"),
        "editorial-serif-light" => ("衬线杂志", "襯線雜誌"),
        "education-friendly-light" => ("亲和教育", "親和教育"),
        "elegant-luxury-dark" => ("优雅奢华", "優雅奢華"),
        "enterprise-slate-dark" => ("深灰商务", "深灰商務"),
        "finance-clean-mobile-light" => ("清爽金融", "清爽金融"),
        "fintech-dark-blue-light" => ("深蓝金融", "深藍金融"),
        "gaming-electric-dark" => ("电玩电光", "電玩電光"),
        "gridpaper-graphite" => ("石墨方格", "石墨方格"),
        "health-minimal-mobile-dark" => ("深色健康", "深色健康"),
        "healthcare-trust-light" => ("清爽医疗", "清爽醫療"),
        "highlighter-notebook-light" => ("荧光手账", "螢光手帳"),
        "industrial-mobile-dark" => ("深色工业", "深色工業"),
        "industrial-neon-dark" => ("工业霓虹", "工業霓虹"),
        "japanese-swiss-light" => ("日式网格", "日式網格"),
        "leadprint-vermilion-light" => ("朱红印刷", "朱紅印刷"),
        "ledger-tick" => ("账簿刻度", "帳簿刻度"),
        "luxury-brand-dark" => ("深色品牌", "深色品牌"),
        "luxury-fashion-mobile-dark" => ("奢华时尚", "奢華時尚"),
        "midnight-minimal-dark" => ("午夜极简", "午夜極簡"),
        "mingsha-mineral-dark" => ("鸣沙矿物", "鳴沙礦物"),
        "minimal-playful-light" => ("简约趣味", "簡約趣味"),
        "monochrome-expressive-light" => ("黑白表现", "黑白表現"),
        "music-dark-mobile" => ("深色音乐", "深色音樂"),
        "neon-purple-mobile-dark" => ("紫色霓虹", "紫色霓虹"),
        "noir-elegant-dark" => ("优雅黑色", "優雅黑色"),
        "nonprofit-warm-light" => ("温暖公益", "溫暖公益"),
        "nordic-frost-light" => ("北欧冰霜", "北歐冰霜"),
        "pastel-soft-mobile-light" => ("柔和粉彩", "柔和粉彩"),
        "portfolio-minimal-light" => ("极简作品集", "極簡作品集"),
        "retro-warm-light" => ("暖色复古", "暖色復古"),
        "saas-clean-light" => ("清爽产品", "清爽產品"),
        "saas-modern-light" => ("现代产品", "現代產品"),
        "scandinavian-minimal-light" => ("北欧极简", "北歐極簡"),
        "social-vibrant-mobile-light" => ("活力社交", "活力社交"),
        "sounding-navy" => ("海军蓝测深", "海軍藍測深"),
        "startup-gradient-dark" => ("深色渐变", "深色漸層"),
        "tech-developer-dark" => ("开发者科技", "開發者科技"),
        "terminal-minimal-dark" => ("极简终端", "極簡終端"),
        "tidemark-slate" => ("岩灰潮线", "岩灰潮線"),
        "travel-warm-mobile-light" => ("暖色旅行", "暖色旅行"),
        "warm-food-mobile-light" => ("暖色餐饮", "暖色餐飲"),
        "wellness-green-mobile-light" => ("健康绿意", "健康綠意"),
        "wellness-organic-light" => ("自然有机", "自然有機"),
        "zen-paper-light" => ("禅意纸感", "禪意紙感"),
        _ => return None,
    };
    Some(if traditional {
        traditional_label
    } else {
        simplified
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_shipped_guide_has_a_short_distinguishable_chinese_name() {
        for locale in [Locale::ZhCn, Locale::ZhTw] {
            let mut names = std::collections::BTreeSet::new();
            for guide in op_ai_skills::style_guide::style_guide_registry() {
                let name = label(&guide.name, locale).expect("catalogue display label");
                assert!(name.chars().count() <= 8);
                assert!(names.insert(name), "ambiguous label: {name}");
            }
        }
        assert_eq!(label("user:my-brand", Locale::ZhCn), None);
        assert_eq!(label("highlighter-notebook-light", Locale::EnUs), None);
    }
}
