//! The self-contained share page `export_share.rs` writes.
//!
//! Like the slideshow template next door, everything is inline — no
//! stylesheet, script, font or image lives outside the file — so the page
//! works from a mail attachment, a chat download or a USB stick. Unlike
//! the slideshow it is a VIEWER, not a projector: a light surround, a
//! header saying what the work is and how it was made, one board at a
//! time with arrows (a deck reads as slides), and a primary "Make one
//! like this" action that downloads the embedded `.op`. Its chrome text
//! follows the recipient's browser language (`export_share_chrome`).

use op_util::xml_escape::escape_html;
use std::fmt::Write as _;

use crate::export_html_structured::css_num;
use crate::export_share_chrome::{ShareChrome, CHROME_ELEMENT_ID, CHROME_JS};

/// One board on the page.
pub struct ShareSlide {
    /// Authored board name — the board's accessible label.
    pub name: String,
    /// The page the board sits on, when the document has several.
    pub page: Option<String>,
    /// Board size in doc px.
    pub width: f32,
    pub height: f32,
    /// Board markup in board-local coordinates.
    pub body: String,
}

/// Element id of the embedded document. The download handler and the
/// tests both read it by this name.
pub const DOCUMENT_ELEMENT_ID: &str = "op-document";

/// Build the whole page. `document_json` is the embedded `.op`.
pub fn render_share_page(
    title: &str,
    slides: &[ShareSlide],
    document_json: &str,
    chrome: &ShareChrome,
) -> String {
    let labels = &chrome.author;
    let mut boards = String::new();
    for (i, slide) in slides.iter().enumerate() {
        let current = if i == 0 { " is-current" } else { "" };
        let page = slide
            .page
            .as_deref()
            .map(|page| format!(" data-page=\"{}\"", escape_html(page)))
            .unwrap_or_default();
        let _ = write!(
            boards,
            "\n<div class=\"slot{current}\" data-w=\"{w}\" data-h=\"{h}\"{page}>\
             <div class=\"slide\" role=\"group\" aria-label=\"{label}\" \
             style=\"width:{w}px;height:{h}px\">{body}</div></div>",
            label = escape_html(&slide.name),
            w = css_num(slide.width),
            h = css_num(slide.height),
            body = slide.body,
        );
    }
    let file_name = format!("{}.op", download_stem(title));
    let recipe_line = if labels.recipe_line.is_empty() {
        String::new()
    } else {
        format!(
            "<div class=\"recipe\">{}</div>",
            escape_html(&labels.recipe_line)
        )
    };
    format!(
        "<!doctype html>\n\
         <html lang=\"{lang}\">\n\
         <head>\n\
         <meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <meta name=\"generator\" content=\"OpenPencil\">\n\
         <title>{title}</title>\n\
         <style>{SHARE_CSS}</style>\n\
         </head>\n\
         <body>\n\
         <header id=\"bar\"><div class=\"who\"><div class=\"title\">{title}</div>{recipe_line}</div>\
         <button id=\"make\" type=\"button\" title=\"{hint}\" data-file=\"{file}\">{make}</button></header>\n\
         <main id=\"stage\">{boards}\n</main>\n\
         <nav id=\"nav\"><button id=\"prev\" type=\"button\" aria-label=\"{prev}\">&#8249;</button>\
         <span id=\"counter\">1 / {count}</span>\
         <button id=\"next\" type=\"button\" aria-label=\"{next}\">&#8250;</button></nav>\n\
         <footer id=\"foot\"><span class=\"made\">{made}</span><span class=\"hint\">{hint}</span></footer>\n\
         <script type=\"application/json\" id=\"{DOCUMENT_ELEMENT_ID}\">{doc}</script>\n\
         <script type=\"application/json\" id=\"{CHROME_ELEMENT_ID}\">{chrome}</script>\n\
         <script>{CHROME_JS}</script>\n\
         <script>{SHARE_JS}</script>\n\
         </body>\n\
         </html>\n",
        lang = labels.html_lang,
        title = escape_html(title),
        hint = escape_html(&labels.make_same_hint),
        file = escape_html(&file_name),
        make = escape_html(&labels.make_same),
        prev = escape_html(&labels.prev),
        next = escape_html(&labels.next),
        made = escape_html(&labels.made_with),
        count = slides.len(),
        doc = script_safe_json(document_json),
        chrome = script_safe_json(&chrome.table_json),
    )
}

/// JSON is inert inside `type="application/json"`, but `</script>` in a
/// string would still close the element. `<` only ever occurs inside JSON
/// strings, where its `\u` escape decodes to the same character, so the
/// rewrite is exact.
pub fn script_safe_json(json: &str) -> String {
    json.replace('<', "\\u003c")
}

/// The downloaded document's file stem: the title, filesystem-safe.
fn download_stem(title: &str) -> String {
    let stem: String = title
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '-'
            } else {
                c
            }
        })
        .take(60)
        .collect();
    let stem = stem.trim();
    if stem.is_empty() {
        "openpencil".to_string()
    } else {
        stem.to_string()
    }
}

/// Light surround; the board is fitted with `transform: scale()` so no
/// element inside is re-laid-out (same reasoning as the slideshow). A
/// board much taller than wide (a long web page, an infographic) fits
/// its width and scrolls instead of shrinking to a sliver.
const SHARE_CSS: &str = "\
*{box-sizing:border-box}\
html,body{margin:0;height:100%;background:#f4f4f5;color:#18181b;\
font:14px/1.4 ui-sans-serif,system-ui,-apple-system,'Segoe UI',Roboto,'PingFang SC',sans-serif}\
body{display:flex;flex-direction:column}\
#bar{display:flex;align-items:center;gap:16px;padding:12px 20px;background:#fff;\
border-bottom:1px solid #e4e4e7}\
.who{flex:1;min-width:0}\
.title{font-weight:600;font-size:16px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}\
.recipe{color:#71717a;font-size:12px;margin-top:2px}\
#make{flex:none;border:0;border-radius:10px;padding:10px 18px;background:#2563eb;color:#fff;\
font:600 14px/1 inherit;cursor:pointer}\
#make:hover{background:#1d4ed8}\
#stage{flex:1;position:relative;overflow:hidden}\
.slot{position:absolute;inset:0;display:none;overflow:auto;padding:24px}\
.slot.is-current{display:block}\
.slot .fit{margin:0 auto;position:relative;box-shadow:0 8px 30px rgba(0,0,0,.12);background:#fff}\
.slide{position:absolute;left:0;top:0;transform-origin:0 0;\
transform:scale(var(--scale,1));overflow:hidden;background:#fff}\
.slide .n{position:absolute;box-sizing:border-box}\
.slide .k{pointer-events:none}\
#nav{display:flex;align-items:center;justify-content:center;gap:14px;padding:8px}\
#nav button{width:36px;height:36px;border-radius:50%;border:1px solid #d4d4d8;background:#fff;\
font-size:20px;line-height:1;cursor:pointer;color:#18181b}\
#nav button:disabled{opacity:.35;cursor:default}\
#counter{min-width:64px;text-align:center;color:#52525b;font-variant-numeric:tabular-nums}\
#foot{display:flex;gap:12px;justify-content:space-between;padding:8px 20px 12px;color:#a1a1aa;\
font-size:12px}\
#foot .hint{text-align:right}\
@media (max-width:640px){#bar{padding:10px 12px;gap:10px}#make{padding:9px 12px;white-space:nowrap}\
#foot .hint{display:none}.slot{padding:12px}}";

/// Navigation, fit and the Make-one-like-this download. Arrow keys,
/// PageUp / PageDown, Home / End and swipes move between boards.
const SHARE_JS: &str = r#"
(function () {
  var slots = Array.prototype.slice.call(document.querySelectorAll('.slot'));
  var counter = document.getElementById('counter');
  var prev = document.getElementById('prev');
  var next = document.getElementById('next');
  var stage = document.getElementById('stage');
  var index = 0;
  slots.forEach(function (slot) {
    var fit = document.createElement('div');
    fit.className = 'fit';
    fit.appendChild(slot.firstElementChild);
    slot.appendChild(fit);
  });
  function fit() {
    var sw = stage.clientWidth - 48, sh = stage.clientHeight - 48;
    slots.forEach(function (slot) {
      var w = parseFloat(slot.getAttribute('data-w')) || 1;
      var h = parseFloat(slot.getAttribute('data-h')) || 1;
      var tall = h / w > 2;
      var scale = tall ? Math.min(sw / w, 1) : Math.min(sw / w, sh / h);
      var box = slot.firstElementChild;
      box.style.width = (w * scale) + 'px';
      box.style.height = (h * scale) + 'px';
      box.firstElementChild.style.setProperty('--scale', scale);
      if (!tall) { box.style.marginTop = Math.max(0, (sh - h * scale) / 2) + 'px'; }
    });
  }
  function show(target) {
    target = Math.max(0, Math.min(slots.length - 1, target));
    slots[index].classList.remove('is-current');
    slots[target].classList.add('is-current');
    index = target;
    counter.textContent = (index + 1) + ' / ' + slots.length;
    prev.disabled = index === 0;
    next.disabled = index === slots.length - 1;
  }
  prev.addEventListener('click', function () { show(index - 1); });
  next.addEventListener('click', function () { show(index + 1); });
  document.addEventListener('keydown', function (event) {
    switch (event.key) {
      case 'ArrowRight': case 'PageDown': show(index + 1); break;
      case 'ArrowLeft': case 'PageUp': show(index - 1); break;
      case 'Home': show(0); break;
      case 'End': show(slots.length - 1); break;
      default: return;
    }
    event.preventDefault();
  });
  var touchX = null;
  stage.addEventListener('touchstart', function (e) { touchX = e.touches[0].clientX; }, { passive: true });
  stage.addEventListener('touchend', function (e) {
    if (touchX === null) { return; }
    var dx = e.changedTouches[0].clientX - touchX;
    touchX = null;
    if (Math.abs(dx) > 50) { show(dx < 0 ? index + 1 : index - 1); }
  });
  document.getElementById('make').addEventListener('click', function () {
    var button = this;
    var text = document.getElementById('op-document').textContent;
    var url = URL.createObjectURL(new Blob([text], { type: 'application/octet-stream' }));
    var link = document.createElement('a');
    link.href = url;
    link.download = button.getAttribute('data-file') || 'openpencil.op';
    document.body.appendChild(link);
    link.click();
    link.remove();
    setTimeout(function () { URL.revokeObjectURL(url); }, 4000);
  });
  window.addEventListener('resize', fit);
  fit();
  show(0);
})();
"#;
