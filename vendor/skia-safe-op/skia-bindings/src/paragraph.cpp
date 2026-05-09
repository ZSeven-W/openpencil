/// Skia skparagraph Module C Wrapper Functions

#include "bindings.h"

#include "modules/skparagraph/include/DartTypes.h"
#include "modules/skparagraph/include/FontCollection.h"
#include "modules/skparagraph/include/Metrics.h"
#include "modules/skparagraph/include/ParagraphCache.h"
#include "modules/skparagraph/include/Paragraph.h"
#include "modules/skparagraph/include/ParagraphBuilder.h"
#include "modules/skparagraph/include/ParagraphStyle.h"
#include "modules/skparagraph/include/TextShadow.h"
#include "modules/skparagraph/include/TextStyle.h"
#include "modules/skparagraph/include/TypefaceFontProvider.h"

#include "modules/skunicode/include/SkUnicode_icu.h"

// m84: needs definition of SkFontData
#include "src/core/SkFontDescriptor.h"

#include <optional>

using namespace skia::textlayout;

//
// FontArguments.h
//

extern "C" {
    void C_FontArguments_Construct(const SkFontArguments* fontArguments, FontArguments* uninitialized) {
        new (uninitialized) FontArguments(*fontArguments);
    }

    void C_FontArguments_CopyConstruct(FontArguments* uninitialized, const FontArguments* self) {
        new (uninitialized) FontArguments(*self);
    }

    void C_FontArguments_destruct(FontArguments* self) {
        self->~FontArguments();
    }

    bool C_FontArguments_Equals(const FontArguments* lhs, const FontArguments* rhs) {
        return *lhs == *rhs;
    }

    size_t C_FontArguments_hash(const FontArguments* self) {
        return std::hash<FontArguments>{}(*self);
    }

    SkTypeface* C_FontArguments_cloneTypeface(const FontArguments* self, SkTypeface* tf) {
        return self->CloneTypeface(sp(tf)).release();
    }
}

//
// FontCollection.h
//

extern "C" {
    FontCollection* C_FontCollection_new() {
        return new FontCollection();
    }

    void C_FontCollection_setAssetFontManager(FontCollection* self, const SkFontMgr* fontManager) {
        self->setAssetFontManager(spFromConst(fontManager));
    }

    void C_FontCollection_setDynamicFontManager(FontCollection* self, const SkFontMgr* fontManager) {
        self->setDynamicFontManager(spFromConst(fontManager));
    }

    void C_FontCollection_setTestFontManager(FontCollection* self, const SkFontMgr* fontManager) {
        self->setTestFontManager(spFromConst(fontManager));
    }

    void C_FontCollection_setDefaultFontManager(FontCollection* self, const SkFontMgr* fontManager) {
        self->setDefaultFontManager(spFromConst(fontManager));
    }

    void C_FontCollection_setDefaultFontManager2(FontCollection* self, const SkFontMgr* fontManager, const char* defaultFamilyName) {
        self->setDefaultFontManager(spFromConst(fontManager), defaultFamilyName);
    }

    void C_FontCollection_setDefaultFontManager3(FontCollection* self, const SkFontMgr* fontManager, const SkStrings* familyNames) {
        self->setDefaultFontManager(spFromConst(fontManager), familyNames->strings);
    }

    SkFontMgr* C_FontCollection_getFallbackManager(const FontCollection* self) {
        return self->getFallbackManager().release();
    }

    void C_FontCollection_findTypefaces(
        FontCollection* self, 
        const SkStrings* familyNames, 
        SkFontStyle fontStyle,
        const FontArguments* fontArguments,
        VecSink<sk_sp<SkTypeface>>* typefaces) {
        // TODO: Don't create a copy of `fontArguments`.
        auto fa = fontArguments ? std::optional(*fontArguments) : std::nullopt;
        auto tfs = self->findTypefaces(familyNames->strings, fontStyle, fa);
        typefaces->set(tfs);
    }

    SkTypeface* C_FontCollection_defaultFallback(FontCollection* self, SkUnichar unicode, const SkStrings* familyNames, SkFontStyle fontStyle, const SkString* locale, const FontArguments* arguments) {
        return self->defaultFallback(unicode, familyNames->strings, fontStyle, *locale, arguments ? std::make_optional(*arguments) : std::nullopt).release();
    }


    SkTypeface* C_FontCollection_defaultFallback2(FontCollection* self) {
        return self->defaultFallback().release();
    }

    SkTypeface* C_FontCollection_defaultEmojiFallback(FontCollection* self, SkUnichar emojiStart, SkFontStyle fontStyle, const SkString* locale) {
        return self->defaultEmojiFallback(emojiStart, fontStyle, *locale).release();
    }

    bool C_FontCollection_fontFallbackEnabled(const FontCollection* self) {
        return const_cast<FontCollection*>(self)->fontFallbackEnabled();
    }

    ParagraphCache* C_FontCollection_paragraphCache(FontCollection* self) {
        return self->getParagraphCache();
    }
}

//
// ParagraphCache.h
//

extern "C" {
    void C_ParagraphCache_destruct(ParagraphCache* self) {
        self->~ParagraphCache();
    }

    void C_ParagraphCache_turnOn(ParagraphCache* self, bool value) {
        self->turnOn(value);
    }

    int C_ParagraphCache_count(ParagraphCache* self) {
        return self->count();
    }
}

//
// ParagraphStyle.h
//

extern "C" {
    void C_StrutStyle_Construct(StrutStyle* uninitialized) {
        new(uninitialized) StrutStyle();
    }

    void C_StrutStyle_CopyConstruct(StrutStyle* uninitialized, const StrutStyle* other) {
        new(uninitialized) StrutStyle(*other);
    }

    void C_StrutStyle_destruct(StrutStyle* self) {
        self->~StrutStyle();
    }

    const SkString* C_StrutStyle_getFontFamilies(const StrutStyle* self, size_t* count) {
        auto& v = self->getFontFamilies();
        *count = v.size();
        return v.data();
    }

    void C_StrutStyle_setFontFamilies(StrutStyle* self, const SkString* data, size_t count) {
        self->setFontFamilies(std::vector<SkString>(data, data + count));
    }

    bool C_StrutStyle_equals(const StrutStyle* self, const StrutStyle* rhs) {
        return *self == *rhs;
    }
}

extern "C" {
    ParagraphStyle* C_ParagraphStyle_new() {
        return new ParagraphStyle();
    }

    ParagraphStyle* C_ParagraphStyle_newCopy(const ParagraphStyle* other) {
        return new ParagraphStyle(*other);
    }

    void C_ParagraphStyle_delete(ParagraphStyle* self) {
        delete self;
    }

    bool C_ParagraphStyle_Equals(const ParagraphStyle* left, const ParagraphStyle* right) {
        return *left == *right;
    }

    bool C_ParagraphStyle_ellipsized(const ParagraphStyle* self) {
        return self->ellipsized();
    }
}

//
// TextShadow.h
//

extern "C" {
    bool C_TextShadow_Equals(const TextShadow* self, const TextShadow* other) {
        return *self == *other;
    }
}

//
// Metrics.h
//

extern "C" {
    size_t C_LineMetrics_styleMetricsCount(const LineMetrics* self) {
        return self->fLineMetrics.size();
    }

    struct IndexedStyleMetrics {
        size_t index;
        StyleMetrics metrics;
    };

    void C_LineMetrics_getAllStyleMetrics(const LineMetrics* self, IndexedStyleMetrics* result) {
        auto begin = self->fLineMetrics.begin();
        auto end = self->fLineMetrics.end();

        for (auto it = begin; it != end; ++it) {
            *result++ = IndexedStyleMetrics { it->first, it->second };
        }
    }
}

//
// Paragraph.h
//

extern "C" {
    void C_Paragraph_delete(Paragraph* self) {
        delete self;
    }
    
    void C_Paragraph_layout(Paragraph* self, SkScalar width) {
        self->layout(width);
    }

    void C_Paragraph_paint(Paragraph* self, SkCanvas* canvas, SkScalar x, SkScalar y) {
        self->paint(canvas, x, y);
    }

    void C_Paragraph_getRectsForRange(Paragraph *self, unsigned start, unsigned end, RectHeightStyle rectHeightStyle,
                                            RectWidthStyle rectWidthStyle, VecSink<TextBox>* textBoxes) {
        auto v = self->getRectsForRange(start, end, rectHeightStyle, rectWidthStyle);
        textBoxes->set(v);
    }

    void C_Paragraph_getRectsForPlaceholders(Paragraph* self, VecSink<TextBox>* result) {
        auto v = self->getRectsForPlaceholders();
        result->set(v);
    }

    void C_Paragraph_getGlyphPositionAtCoordinate(Paragraph* self, SkScalar x, SkScalar y, PositionWithAffinity* position) {
        *position = self->getGlyphPositionAtCoordinate(x, y);
    }

    void C_Paragraph_getWordBoundary(Paragraph* self, unsigned offset, size_t range[2]) {
        auto sk_range = self->getWordBoundary(offset);
        range[0] = sk_range.start;
        range[1] = sk_range.end;
    }

    void C_Paragraph_getLineMetrics(Paragraph* self, VecSink<LineMetrics>* result) {
        std::vector<LineMetrics> vec;
        self->getLineMetrics(vec);
        result->set(vec);
    }

    size_t C_Paragraph_lineNumber(Paragraph* self) {
        return self->lineNumber();
    }

    void C_Paragraph_markDirty(Paragraph* self) {
        self->markDirty();
    }

    int32_t C_Paragraph_unresolvedGlyphs(Paragraph* self) {
        return self->unresolvedGlyphs();
    }

    void C_Paragraph_unresolvedCodepoints(Paragraph* self, VecSink<SkUnichar>* result) {
        auto set = self->unresolvedCodepoints();
        std::vector<SkUnichar> vec(set.begin(), set.end());
        result->set(vec);
    }

    void C_Paragraph_visit(Paragraph* self, void* ctx, void (*visit)(void *, size_t, const Paragraph::VisitorInfo *)) {
        auto visitFn = [ctx,visit](int i, const Paragraph::VisitorInfo *info_)
        {
            visit(ctx, i, info_);
        };
        self->visit(visitFn);
    }

    void C_Paragraph_extendedVisit(Paragraph* self, void* ctx, void (*visit)(void *, size_t, const Paragraph::ExtendedVisitorInfo *)) {
        auto visitFn = [ctx,visit](int i, const Paragraph::ExtendedVisitorInfo *info_)
        {
            visit(ctx, i, info_);
        };
        self->extendedVisit(visitFn);
    }

    int C_Paragraph_getPath(Paragraph* self, int lineNumber, SkPath* path) {
        return self->getPath(lineNumber, path);
    }

    void C_Paragraph_GetPath(SkTextBlob* textBlob, SkPath* uninitialized) {
        new (uninitialized) SkPath(Paragraph::GetPath(textBlob));
    }

    bool C_Paragraph_containsEmoji(Paragraph* self, SkTextBlob* textBlob) {
        return self->containsEmoji(textBlob);
    }

    bool C_Paragraph_containsColorFontOrBitmap(Paragraph* self, SkTextBlob* textBlob) {
        return self->containsColorFontOrBitmap(textBlob);
    }

    int C_Paragraph_getLineNumberAt(const Paragraph* self, TextIndex codeUnitIndex) {
        return self->getLineNumberAt(codeUnitIndex);
    }

    int C_Paragraph_getLineNumberAtUTF16Offset(Paragraph* self, size_t codeUnitIndex) {
        return self->getLineNumberAtUTF16Offset(codeUnitIndex);
    }

    void C_Paragraph_getLineMetricsAt(const Paragraph* self, size_t lineNumber, Sink<LineMetrics>* lineMetrics) {
        LineMetrics lm;
        if (self->getLineMetricsAt(lineNumber, &lm)) {
            lineMetrics->set(lm);
        }
    }

    void C_Paragraph_getActualTextRange(const Paragraph* self, size_t lineNumber, bool includeSpaces, size_t r[2]) {
        auto range = self->getActualTextRange(lineNumber, includeSpaces);
        r[0] = range.start;
        r[1] = range.end;
    }

    void C_Paragraph_getGlyphClusterAt(const Paragraph* self, TextIndex codeUnitIndex, Sink<Paragraph::GlyphClusterInfo>* r) {
        Paragraph::GlyphClusterInfo gci;
        // Most likely const, implementation does not seem to mutate Paragraph.
        if (const_cast<Paragraph*>(self)->getGlyphClusterAt(codeUnitIndex, &gci)) {
            r->set(gci);
        }
    }

    void C_Paragraph_getClosestGlyphClusterAt(const Paragraph* self, SkScalar dx, SkScalar dy, Sink<Paragraph::GlyphClusterInfo>* r) {
        Paragraph::GlyphClusterInfo gci;
        // Most likely const, implementation does not seem to mutate Paragraph.
        if (const_cast<Paragraph*>(self)->getClosestGlyphClusterAt(dx, dy, &gci)) {
            r->set(gci);
        }
    }

    bool C_Paragraph_getGlyphInfoAtUTF16Offset(Paragraph* self, size_t codeUnitIndex, Paragraph::GlyphInfo* uninitialized) {
        Paragraph::GlyphInfo gi;
        if (self->getGlyphInfoAtUTF16Offset(codeUnitIndex, &gi)) {
            new (uninitialized) Paragraph::GlyphInfo(gi);
            return true;
        }
        return false;
    }

    bool C_Paragraph_getClosestUTF16GlyphInfoAt(Paragraph* self, SkScalar dx, SkScalar dy, Paragraph::GlyphInfo* uninitialized) {
        Paragraph::GlyphInfo gi;
        if (self->getClosestUTF16GlyphInfoAt(dx, dy, &gi)) {
            new (uninitialized) Paragraph::GlyphInfo(gi);
            return true;
        }
        return false;
    }

    void C_Paragraph_getFontAt(const Paragraph* self, TextIndex codeUnitIndex, SkFont* uninitialized) {
        new (uninitialized) SkFont(self->getFontAt(codeUnitIndex));
    }

    void C_Paragraph_getFontAtUTF16Offset(Paragraph* self, size_t codeUnitIndex, SkFont* uninitialized) {
        new (uninitialized) SkFont(self->getFontAtUTF16Offset(codeUnitIndex));
    }

    void C_Paragraph_getFonts(const Paragraph* self, VecSink<Paragraph::FontInfo>* r) {
        auto fonts = self->getFonts();
        r->set(fonts);
    }
}

//
// ParagraphBuilder.h
//

extern "C" {
    void C_ParagraphBuilder_delete(ParagraphBuilder* self) {
        delete self;
    }

    void C_ParagraphBuilder_pushStyle(ParagraphBuilder* self, const TextStyle* style) {
        self->pushStyle(*style);
    }

    void C_ParagraphBuilder_pop(ParagraphBuilder* self) {
        self->pop();
    }

    void C_ParagraphBuilder_peekStyle(ParagraphBuilder* self, TextStyle* style) {
        *style = self->peekStyle();
    }

    void C_ParagraphBuilder_addText(ParagraphBuilder* self, const char* text, size_t len) {
        self->addText(text, len);
    }

    void C_ParagraphBuilder_addPlaceholder(ParagraphBuilder* self, const PlaceholderStyle* placeholderStyle) {
        self->addPlaceholder(*placeholderStyle);
    }

    Paragraph* C_ParagraphBuilder_Build(ParagraphBuilder* self) {
        return self->Build().release();
    }

    void C_ParagraphBuilder_getText(ParagraphBuilder* self, char** text, size_t* len) {
        auto span = self->getText();
        *text = span.data();
        *len = span.size();
    }

    ParagraphStyle* C_ParagraphBuilder_getParagraphStyle(const ParagraphBuilder* self) {
        return new ParagraphStyle(self->getParagraphStyle());
    }

    void C_ParagraphBuilder_Reset(ParagraphBuilder* self) {
        return self->Reset();
    }

    ParagraphBuilder* C_ParagraphBuilder_make(const ParagraphStyle* style, const FontCollection* fontCollection) {
        auto unicode = SkUnicodes::ICU::Make();
        if (!unicode) {
            return nullptr;
        }

        return ParagraphBuilder::make(*style, spFromConst(fontCollection), unicode).release();
    }
}

//
// TextStyle.h
//

extern "C" {
    void C_TextStyle_Types(const Block*, const Placeholder*) {}

    void C_FontFeature_CopyConstruct(FontFeature* uninitialized, const FontFeature* other) {
        new(uninitialized) FontFeature(*other);
    }

    void C_FontFeature_destruct(FontFeature* self) {
        self->~FontFeature();
    }

    void C_TextStyle_Construct(TextStyle* uninitialized) {
        new(uninitialized) TextStyle();
    }

    void C_TextStyle_CopyConstruct(TextStyle* uninitialized, const TextStyle* other) {
        new(uninitialized) TextStyle(*other);
    }

    void C_TextStyle_cloneForPlaceholder(const TextStyle* self, TextStyle* uninitialized) {
        // m102: We assume that they just forgot to mark `TextStyle::cloneForPlaceholder` as const.
        new (uninitialized) TextStyle(const_cast<TextStyle*>(self)->cloneForPlaceholder());
    }

    void C_TextStyle_destruct(TextStyle* self) {
        self->~TextStyle();
    }

    void C_TextStyle_getForeground(const TextStyle* self, SkPaint* uninitialized) {
        new (uninitialized) SkPaint(self->getForeground());
    }

    void C_TextStyle_setForegroundPaint(TextStyle* self, const SkPaint* paint) {
        self->setForegroundPaint(*paint);
    }

    void C_TextStyle_getBackground(const TextStyle* self, SkPaint* uninitialized) {
        new (uninitialized) SkPaint(self->getBackground());
    }

    void C_TextStyle_setBackgroundPaint(TextStyle* self, const SkPaint* paint) {
        self->setBackgroundColor(*paint);
    }

    const TextShadow* C_TextStyle_getShadows(const std::vector<TextShadow>* self, size_t* len_ref) {
        auto len = self->size();
        *len_ref = len;
        return len ? self->data() : nullptr;
    }

    void C_TextStyle_addShadow(TextStyle* self, const TextShadow* shadow) {
        self->addShadow(*shadow);
    }

    void C_TextStyle_resetShadows(TextStyle* self) {
        self->resetShadows();
    }

    const FontFeature* C_TextStyle_getFontFeatures(const std::vector<FontFeature>* self, size_t* len_ref) {
        auto size = self->size();
        *len_ref = size;
        return size ? self->data() : nullptr;
    }

    void C_TextStyle_addFontFeature(TextStyle* self, const SkString* fontFeature, int value) {
        self->addFontFeature(*fontFeature, value);
    }

    void C_TextStyle_resetFontFeatures(TextStyle* self) {
        self->resetFontFeatures();
    }

    const FontArguments* C_TextStyle_getFontArguments(const TextStyle* self) {
        auto& fontArguments = self->getFontArguments();
        return fontArguments ? &*fontArguments : nullptr;
    }

    void C_TextStyle_setFontArguments(TextStyle* self, const SkFontArguments* arguments) {
        self->setFontArguments(arguments ? std::optional(*arguments) : std::nullopt);
    }

    const SkString* C_TextStyle_getFontFamilies(const TextStyle* self, size_t* count) {
        auto& v = self->getFontFamilies();
        *count = v.size();
        return v.data();
    }

    void C_TextStyle_setFontFamilies(TextStyle* self, const SkString* data, size_t count) {
        self->setFontFamilies(std::vector<SkString>(data, data + count));
    }

    void C_TextStyle_setTypeface(TextStyle* self, SkTypeface* typeface) {
        self->setTypeface(sk_sp<SkTypeface>(typeface));
    }
}

//
// TypefaceFontProvider
//

extern "C" {
    TypefaceFontStyleSet* C_TypefaceFontStyleSet_new(const SkString* family_name) {
        return new TypefaceFontStyleSet(*family_name);
    }

    void C_TypefaceFontStyleSet_appendTypeface(TypefaceFontStyleSet* self, SkTypeface* typeface) {
        self->appendTypeface(sk_sp<SkTypeface>(typeface));
    }
}

extern "C" {
    TypefaceFontProvider* C_TypefaceFontProvider_new() {
        return new TypefaceFontProvider();
    }

    size_t C_TypefaceFontProvider_registerTypeface(TypefaceFontProvider* self, SkTypeface* typeface, const SkString* alias) {
        if (alias) {
            return self->registerTypeface(sk_sp<SkTypeface>(typeface), *alias);
        }
        return self->registerTypeface(sk_sp<SkTypeface>(typeface));
    }
}
