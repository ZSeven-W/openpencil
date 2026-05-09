#include "bindings.h"

#include "include/core/SkFontMgr.h"
#include "modules/skshaper/include/SkShaper.h"
#include "modules/skshaper/include/SkShaper_harfbuzz.h"
#include "modules/skshaper/include/SkShaper_skunicode.h"
#include "modules/skunicode/include/SkUnicode_icu.h"
#ifdef SK_SHAPER_CORETEXT_AVAILABLE
#include "modules/skshaper/include/SkShaper_coretext.h"
#endif

#if defined(_WIN32)
#include "third_party/icu/SkLoadICU.h"
#endif

#ifdef SK_SHAPER_CORETEXT_AVAILABLE
extern "C" SkShaper* C_SkShaper_MakeCoreText(SkShapers::CT::LineBreakMode lineBreakMode) {
    return SkShapers::CT::CoreText(lineBreakMode).release();
}
#endif

extern "C" SkShaper* C_SkShaper_Make(SkFontMgr* fontMgr) {
    return SkShaper::Make(sk_sp<SkFontMgr>(fontMgr)).release();
}

extern "C" void C_SkShaper_delete(SkShaper* self) {
    delete self;
}

extern "C" void C_SkShaper_RunIterator_delete(SkShaper::RunIterator* self) {
    delete self;
}

extern "C" void C_SkShaper_RunIterator_consume(SkShaper::RunIterator* self)  {
    self->consume();
}

extern "C" size_t C_SkShaper_RunIterator_endOfCurrentRun(const SkShaper::RunIterator* self) {
    return self->endOfCurrentRun();
}

extern "C" bool C_SkShaper_RunIterator_atEnd(const SkShaper::RunIterator* self) {
    return self->atEnd();
}

extern "C" const SkFont* C_SkShaper_FontRunIterator_currentFont(const SkShaper::FontRunIterator* self) {
    return &self->currentFont();
}

extern "C" SkShaper::FontRunIterator* C_SkShaper_MakeFontMgrRunIterator(const char* utf8, size_t utf8Bytes, const SkFont* font, SkFontMgr* fallback) {
    return SkShaper::MakeFontMgrRunIterator(utf8, utf8Bytes, *font, sk_sp<SkFontMgr>(fallback)).release();
}

extern "C" SkShaper::FontRunIterator* C_SkShaper_TrivialFontRunIterator_new(const SkFont& font, size_t utf8Bytes) {
    return new SkShaper::TrivialFontRunIterator(font, utf8Bytes);
}

extern "C" uint8_t C_SkShaper_BiDiRunIterator_currentLevel(const SkShaper::BiDiRunIterator* self) {
    return self->currentLevel();
}

extern "C" SkShaper::BiDiRunIterator* C_SkShaper_MakeBidiRunIterator(const char* utf8, size_t utf8Bytes, uint8_t bidiLevel) {
    return SkShaper::MakeBiDiRunIterator(utf8, utf8Bytes, bidiLevel).release();
}

extern "C" SkShaper::BiDiRunIterator* C_SkShaper_MakeIcuBidiRunIterator(const char* utf8, size_t utf8Bytes, uint8_t bidiLevel) {
    return SkShaper::MakeIcuBiDiRunIterator(utf8, utf8Bytes, bidiLevel).release();
}

extern "C" SkFourByteTag C_SkShaper_ScriptRunIterator_currentScript(const SkShaper::ScriptRunIterator* self) {
    return self->currentScript();
}

extern "C" SkShaper::ScriptRunIterator* C_SkShaper_MakeScriptRunIterator(const char* utf8, size_t utf8Bytes, SkFourByteTag script) {
    return SkShaper::MakeScriptRunIterator(utf8, utf8Bytes, script).release();
}

extern "C" SkShaper::ScriptRunIterator* C_SkShaper_MakeHbIcuScriptRunIterator(const char* utf8, size_t utf8Bytes) {
    return SkShaper::MakeHbIcuScriptRunIterator(utf8, utf8Bytes).release();
}

extern "C" const char* C_SkShaper_LanguageRunIterator_currentLanguage(const SkShaper::LanguageRunIterator* self) {
    return self->currentLanguage();
}

extern "C" SkShaper::LanguageRunIterator* C_SkShaper_MakeStdLanguageRunIterator(const char* utf8, size_t utf8Bytes) {
    return SkShaper::MakeStdLanguageRunIterator(utf8, utf8Bytes).release();
}

extern "C" SkShaper::LanguageRunIterator* C_SkShaper_TrivialLanguageRunIterator_new(const char* language, size_t utf8Bytes) {
    return new SkShaper::TrivialLanguageRunIterator(language, utf8Bytes);
}

extern "C" void C_SkShaper_RunHandler_delete(SkShaper::RunHandler* self) {
    delete self;
}

namespace RunHandler {
    extern "C" typedef void (*BeginLine)(TraitObject);
    extern "C" typedef void (*RunInfo)(TraitObject, const SkShaper::RunHandler::RunInfo*);
    extern "C" typedef void (*CommitRunInfo)(TraitObject);
    extern "C" typedef SkShaper::RunHandler::Buffer (*RunBuffer)(TraitObject, const SkShaper::RunHandler::RunInfo*);
    extern "C" typedef void (*CommitRunBuffer)(TraitObject, const SkShaper::RunHandler::RunInfo*);
    extern "C" typedef void (*CommitLine)(TraitObject);
}

class RustRunHandler: SkShaper::RunHandler {

public:
    struct Param {
        TraitObject trait;
        ::RunHandler::BeginLine beginLine;
        ::RunHandler::RunInfo runInfo;
        ::RunHandler::CommitRunInfo commitRunInfo;
        ::RunHandler::RunBuffer runBuffer;
        ::RunHandler::CommitRunBuffer commitRunBuffer;
        ::RunHandler::CommitLine commitLine;
    };

    explicit RustRunHandler(const Param& param)
    :_param(param){
    }


private:
    void beginLine() override {
        _param.beginLine(_param.trait);
    }

    void runInfo(const RunInfo &info) override {
        _param.runInfo(_param.trait, &info);
    }

    void commitRunInfo() override {
        _param.commitRunInfo(_param.trait);
    }

    Buffer runBuffer(const RunInfo &info) override {
        return _param.runBuffer(_param.trait, &info);
    }

    void commitRunBuffer(const RunInfo &info) override {
        _param.commitRunBuffer(_param.trait, &info);
    }

    void commitLine() override {
        _param.commitLine(_param.trait);
    }

private:
    Param _param;
};

extern "C" void C_RustRunHandler_construct(RustRunHandler* uninitialized, const RustRunHandler::Param* param) {
    new(uninitialized)RustRunHandler(*param);
}

extern "C" void
C_SkShaper_shape(const SkShaper *self, const char *utf8, size_t utf8Bytes, const SkFont *srcFont, bool leftToRight,
                 SkScalar width, SkShaper::RunHandler *runHandler) {
    self->shape(utf8, utf8Bytes, *srcFont, leftToRight, width, runHandler);
}

extern "C" void
C_SkShaper_shape2(const SkShaper *self, const char *utf8, size_t utf8Bytes, SkShaper::FontRunIterator *fontRunIterator,
                  SkShaper::BiDiRunIterator *bidiRunIterator,
                  SkShaper::ScriptRunIterator *scriptRunIterator,
                  SkShaper::LanguageRunIterator *languageRunIterator, SkScalar width,
                  SkShaper::RunHandler *runHandler) {
    self->shape(utf8, utf8Bytes, *fontRunIterator, *bidiRunIterator, *scriptRunIterator, *languageRunIterator, width,
                runHandler);
}

extern "C" void
C_SkShaper_shape3(const SkShaper *self, const char *utf8, size_t utf8Bytes, SkShaper::FontRunIterator *fontRunIterator,
                  SkShaper::BiDiRunIterator *bidiRunIterator,
                  SkShaper::ScriptRunIterator *scriptRunIterator,
                  SkShaper::LanguageRunIterator *languageRunIterator,
                  const SkShaper::Feature *features, size_t featuresSize,
                  SkScalar width,
                  SkShaper::RunHandler *runHandler) {
    self->shape(utf8, utf8Bytes, *fontRunIterator, *bidiRunIterator, *scriptRunIterator, *languageRunIterator, features,
                featuresSize, width,
                runHandler);
}

extern "C" void C_SkTextBlobBuilderRunHandler_construct(SkTextBlobBuilderRunHandler* uninitialized, const char* utf8Text, const SkPoint* offset) {
    new(uninitialized)SkTextBlobBuilderRunHandler(utf8Text, *offset);
}

extern "C" SkTextBlob* C_SkTextBlobBuilderRunHandler_makeBlob(SkTextBlobBuilderRunHandler* self) {
    return self->makeBlob().release();
}

extern "C" SkPoint C_SkTextBlobBuilderRunHandler_endPoint(SkTextBlobBuilderRunHandler* self) {
    return self->endPoint();
}

// SkShapers::Primitive

extern "C" SkShaper* C_SkShapers_Primitive_PrimitiveText() {
    return SkShapers::Primitive::PrimitiveText().release();
}

extern "C" SkShaper::BiDiRunIterator* C_SkShapers_Primitive_TrivialBidiRunIterator_new(uint8_t bidiLevel, size_t utf8Bytes) {
    // m125: This function is not yet implemented.
    // return SkShapers::Primitive::TrivialBiDiRunIterator(bidiLevel, utf8Bytes).release();
    return new SkShaper::TrivialBiDiRunIterator(bidiLevel, utf8Bytes);
}

extern "C" SkShaper::ScriptRunIterator* C_SkShapers_Primitive_TrivialScriptRunIterator_new(uint8_t bidiLevel, size_t utf8Bytes) {
    // m125: This function is not yet implemented.
    // return SkShapers::Primitive::TrivialScriptRunIterator(bidiLevel, utf8Bytes).release();
    return new SkShaper::TrivialScriptRunIterator(bidiLevel, utf8Bytes);
}

// SkShapers::HB

extern "C" SkShaper* C_SkShapers_HB_ShaperDrivenWrapper(SkFontMgr* fontMgr) {
    auto unicode = SkUnicodes::ICU::Make();
    if (!unicode) {
        return nullptr;
    }
    return SkShapers::HB::ShaperDrivenWrapper(std::move(unicode), sk_sp<SkFontMgr>(fontMgr)).release();
}

extern "C" SkShaper* C_SkShapers_HB_ShapeThenWrap(SkFontMgr* fontMgr) {
    auto unicode = SkUnicodes::ICU::Make();
    if (!unicode) {
        return nullptr;
    }
    return SkShapers::HB::ShapeThenWrap(std::move(unicode), sk_sp<SkFontMgr>(fontMgr)).release();
}

extern "C" SkShaper* C_SkShapers_HB_ShapeDontWrapOrReorder(SkFontMgr* fontMgr) {
    auto unicode = SkUnicodes::ICU::Make();
    if (!unicode) {
        return nullptr;
    }
    return SkShapers::HB::ShapeDontWrapOrReorder(std::move(unicode), sk_sp<SkFontMgr>(fontMgr)).release();
}

// SkShapers::unicode

extern "C" SkShaper::BiDiRunIterator* C_SkShapers_unicode_BidiRunIterator(const char* utf8, size_t utf8Bytes, uint8_t bidiLevel) {
    auto unicode = SkUnicodes::ICU::Make();
    if (!unicode) {
        return nullptr;
    }
    return SkShapers::unicode::BidiRunIterator(std::move(unicode), utf8, utf8Bytes, bidiLevel).release();
}
