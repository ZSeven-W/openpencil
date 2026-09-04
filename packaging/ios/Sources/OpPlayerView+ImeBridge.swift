import UIKit

/// The UITextViewDelegate half of the IME bridge. The conduit text view
/// itself (`ImeConduitTextView`, including its `deleteBackward` override)
/// lives in OpPlayerView.swift; this extension owns the composition and
/// insertion forwarding, and enforces the conduit's empty invariant:
/// outside a composition the conduit holds no text, so plain backspace
/// always reaches `deleteBackward` on an empty view and is forwarded to
/// the engine. Before the drain calls below existed, every composition
/// commit accumulated in the conduit; once non-empty, Chinese IMEs'
/// backspace deleted that invisible text instead of reaching the engine —
/// the on-device "typed text cannot be deleted" defect.
extension OpPlayerView {
    /// The engine's IME focus flipped; show or hide the system keyboard.
    func imeFocusChanged(_ focused: Bool) {
        NSLog("[IME] imeFocusChanged(%d) firstResponder=%d", focused ? 1 : 0,
              imeTextView.isFirstResponder ? 1 : 0)
        if focused {
            if !imeTextView.isFirstResponder {
                imeTextView.becomeFirstResponder()
            }
        } else {
            // The engine has already released the input owner. End any
            // UIKit-side marked range before this conduit is reused so a
            // cancelled Chinese candidate cannot become the next field's
            // commit payload.
            wasComposing = false
            imeTextView.unmarkText()
            drainImeConduit(imeTextView)
            if imeTextView.isFirstResponder {
                imeTextView.resignFirstResponder()
            }
        }
    }

    func textViewDidChange(_ textView: UITextView) {
        guard host.editorMode else { return }
        guard host.editorImeFocusedNow() else {
            // UIKit may finish an in-flight marked range synchronously while
            // resigning first responder. The engine has already blurred, so
            // discard that conduit-only residue instead of sending it to the
            // next focused editor field.
            wasComposing = false
            drainImeConduit(textView)
            return
        }
        let marked = markedText(of: textView)
        if let marked {
            // Composition update: forward the marked text + caret.
            wasComposing = true
            let markedRange = textView.markedTextRange
            let selection = textView.selectedTextRange
            var selectedBytes = 0..<0
            if let markedRange, let selection {
                // UITextInput positions are UTF-16 code-unit offsets. The
                // Rust ABI deliberately takes UTF-8 byte offsets, so convert
                // relative to the marked string rather than forwarding the
                // UIKit numbers unchanged (which breaks after the first CJK
                // scalar).
                let start = textView.offset(from: markedRange.start, to: selection.start)
                let end = textView.offset(from: markedRange.start, to: selection.end)
                selectedBytes = ImeSelectionOffsets.utf8Range(
                    in: marked,
                    utf16Start: start,
                    utf16End: end
                )
            }
            host.editorImePreedit(marked, selection: selectedBytes)
        } else if wasComposing {
            // The composition ended. The conduit is empty outside a
            // composition (drained below and in shouldChangeTextIn), so the
            // full conduit text IS the commit payload — the candidate the
            // user chose, which can differ from the last preedit string
            // ("si" → "四"). An empty conduit means the composition was
            // cancelled; the empty commit clears the engine's preedit.
            wasComposing = false
            host.editorImeCommit(textView.text ?? "")
            drainImeConduit(textView)
        } else if !(textView.text ?? "").isEmpty {
            // Safety net: text landed without a composition and without
            // being forwarded from shouldChangeTextIn (some IMEs and
            // dictation insert directly). Forward it, then restore the
            // empty invariant so backspace keeps reaching the engine.
            host.editorText(textView.text)
            drainImeConduit(textView)
        }
        host.requestImmediateFrame()
    }

    func textView(
        _ textView: UITextView,
        shouldChangeTextIn range: NSRange,
        replacementText text: String
    ) -> Bool {
        guard host.editorMode else { return true }
        guard host.editorImeFocusedNow() else { return true }
        // While composing, the system owns the edit (preedit updates flow
        // through textViewDidChange).
        if markedText(of: textView) != nil {
            return true
        }
        if text == "\n" {
            host.editorKey(Int32(OpKey_Enter))
            host.requestImmediateFrame()
            return false
        }
        if range.length == 0 {
            if text.isEmpty {
                // Backspace on the empty conduit. Because the conduit is
                // kept empty outside a composition, the caret always sits
                // at offset 0 with nothing to delete, and iOS (observed on
                // 26.x, simulator and device) delivers backspace as this
                // zero-length-range, empty-replacement delegate call and
                // never invokes `deleteBackward`. Forwarding it as plain
                // insertion made every backspace a no-op `editorText("")`
                // — the "typed text cannot be deleted" on-device defect.
                host.editorKey(Int32(OpKey_Backspace))
            } else {
                // Let UIKit apply the insertion first. A Chinese IME's first
                // pinyin character reaches this delegate BEFORE it installs a
                // marked range; vetoing it here prevents composition from ever
                // starting. `textViewDidChange` then sees either the new
                // marked range (preedit) or a plain insertion, forwards it
                // exactly once, and drains the conduit.
                return true
            }
        } else {
            // Deletion fallback for IME paths that edit through the
            // delegate rather than UIKeyInput. With the empty invariant
            // holding, this branch is normally unreachable outside a
            // composition (there is nothing to delete).
            let atEnd = range.location + range.length >= (textView.text as NSString).length
            host.editorKey(Int32(atEnd ? OpKey_Backspace : OpKey_Delete))
        }
        host.requestImmediateFrame()
        return false
    }

    /// Restore the conduit's empty invariant. The engine owns the real
    /// text; any character left in the conduit makes the next backspace
    /// delete invisible conduit text instead of reaching the engine. Only
    /// drains while no composition is active (clearing under a marked
    /// range would yank the preedit out from under the IME); programmatic
    /// `text` writes do not re-enter the delegate and leave
    /// first-responder status untouched.
    private func drainImeConduit(_ textView: UITextView) {
        guard textView.markedTextRange == nil else { return }
        if !(textView.text ?? "").isEmpty {
            textView.text = ""
        }
    }

    private func markedText(of textView: UITextView) -> String? {
        guard let range = textView.markedTextRange else { return nil }
        return textView.text(in: range)
    }
}
