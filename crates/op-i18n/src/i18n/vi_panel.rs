//! Overflow-shard strings for this locale.
//!
//! The main table sits at the repo's 800-line file cap, so `vi_git`
//! falls through here for the `imagePanel.*` popover keys and the
//! `providerProbe.*` keys the Antigravity / Grok Build CLI probes emit.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "imagePanel.searchPlaceholder" => "Tìm hình ảnh…",
        "imagePanel.searching" => "Đang tìm…",
        "imagePanel.noResults" => "Không có kết quả",
        "imagePanel.searchPrompt" => "Tìm kiếm hình ảnh",
        "imagePanel.sourceNotice" => {
            "Hình ảnh từ {{source}}. Giấy phép tự do — hãy kiểm tra giấy phép trước khi dùng."
        }
        "imagePanel.genNotConfigured" => "Chưa cấu hình tạo hình ảnh",
        "imagePanel.openSettings" => "Mở cài đặt",
        "imagePanel.promptPlaceholder" => "Mô tả hình ảnh…",
        "providerProbe.connectedViaCli" => "Đã kết nối qua CLI {{name}}",
        "providerProbe.cliExitedWithError" => "CLI {{name}} đã thoát với lỗi",
        "providerProbe.cliNoVersionOutput" => "CLI {{name}} không xuất thông tin phiên bản",
        "providerProbe.modelQueryFailed" => {
            "Truy vấn mô hình {{name}} thất bại hoặc hết thời gian chờ"
        }
        "providerProbe.modelQueryFailedRunLogin" => {
            "Truy vấn mô hình {{name}} thất bại. Hãy chạy {{command}} một lần để xác thực."
        }
        "providerProbe.modelQueryNeedsAuth" => {
            "Truy vấn mô hình {{name}} cần xác thực. Hãy chạy {{command}} một lần để đăng nhập."
        }
        "providerProbe.unrecognizedModelCatalog" => {
            "{{name}} đã trả về danh mục mô hình không nhận dạng được"
        }
        _ => return super::vi_collab::lookup(key),
    })
}
