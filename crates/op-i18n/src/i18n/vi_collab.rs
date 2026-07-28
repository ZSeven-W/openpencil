//! Chuỗi giao diện cộng tác.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "collab.topbar.collaborate" => "Cộng tác",
        "collab.topbar.starting" => "Đang bắt đầu cộng tác…",
        "collab.topbar.joining" => "Đang tham gia…",
        "collab.topbar.authenticating" => "Đang xác thực…",
        "collab.topbar.connected" => "Đã kết nối",
        "collab.topbar.reconnecting" => "Đang kết nối lại…",
        "collab.topbar.readOnly" => "Chỉ đọc",
        "collab.topbar.ended" => "Phiên đã kết thúc",
        "collab.topbar.participants" => "{{count}} người tham gia",
        "collab.topbar.unavailable" => "Bản dựng này chưa hỗ trợ cộng tác",
        "collab.action.start" => "Bắt đầu phiên",
        "collab.action.join" => "Tham gia phiên",
        "collab.action.leave" => "Rời phiên",
        "collab.action.retry" => "Thử lại",
        "collab.action.cancel" => "Hủy",
        "collab.action.connect" => "Kết nối",
        "collab.action.discardPending" => "Bỏ chỉnh sửa đang chờ",
        "collab.action.saveAsFork" => "Lưu thành bản phân nhánh",
        "collab.action.approveEditor" => "Duyệt quyền chỉnh sửa",
        "collab.action.approveViewer" => "Duyệt quyền xem",
        "collab.action.rejectAdmission" => "Từ chối",
        "collab.admission.request" => "Một người tham gia đã xác thực đang yêu cầu quyền truy cập.",
        "collab.join.title" => "Tham gia phiên cộng tác",
        "collab.join.discovering" => "Đang tìm phiên trong mạng cục bộ…",
        "collab.join.noSessions" => "Không tìm thấy phiên cục bộ",
        "collab.join.address" => "Địa chỉ IP và cổng",
        "collab.join.addressPlaceholder" => "192.168.1.8:43120",
        "collab.join.authenticating" => "Đang xác minh phiên bảo mật…",
        "collab.join.incompatible" => "Phiên này dùng phiên bản không tương thích",
        "collab.join.signInRequired" => "Đăng nhập để bắt đầu hoặc tham gia phiên",
        "collab.session.title" => "Cộng tác",
        "collab.session.name" => "Phiên: {{name}}",
        "collab.session.shareAddress" => "Địa chỉ chia sẻ",
        "collab.session.role.owner" => "Chủ sở hữu",
        "collab.session.role.editor" => "Người chỉnh sửa",
        "collab.session.role.viewer" => "Người xem",
        "collab.session.pending" => "Đang chờ chủ sở hữu xác nhận chỉnh sửa của bạn…",
        "collab.status.disconnectedReadOnly" => {
            "Mất kết nối. Chỉnh sửa tạm dừng trong khi kết nối lại."
        }
        "collab.status.ticketExpired" => "Đăng nhập cộng tác đã hết hạn. Hãy đăng nhập lại.",
        "collab.status.ownerLeft" => {
            "Chủ sở hữu đã rời nên phiên kết thúc. Bạn có thể lưu một bản riêng."
        }
        "collab.status.epochChanged" => {
            "Chủ sở hữu đã bắt đầu phiên mới. Chỉnh sửa đang chờ chưa được gửi."
        }
        "collab.status.undoConflict" => {
            "Không thể hoàn tác vì cùng trường đã được người khác sửa sau đó."
        }
        "collab.status.unsupportedEdit" => "Chỉnh sửa này chưa được hỗ trợ và không được áp dụng.",
        "collab.status.profileUnavailable" => {
            "Không tải được ảnh hồ sơ; đang hiển thị chữ viết tắt."
        }
        "collab.reject.staleBase" => "Tài liệu đã thay đổi trước. Đang đồng bộ rồi thử lại.",
        "collab.reject.readOnly" => "Bạn chỉ có quyền xem trong phiên này.",
        "collab.reject.unsupported" => "Chủ sở hữu không hỗ trợ chỉnh sửa đó.",
        "collab.reject.conflict" => "Chỉnh sửa đó xung đột với thay đổi mới hơn.",
        "collab.reject.resourceLimit" => "Chỉnh sửa đó vượt giới hạn của phiên.",
        "collab.reject.authentication" => "Quyền cộng tác của bạn không còn hợp lệ.",
        "collab.reject.unknown" => "Chủ sở hữu đã từ chối chỉnh sửa đó.",
        "collab.gate.pages" => "Thay đổi trang chưa được hỗ trợ khi cộng tác.",
        "collab.gate.pageBackground" => "Thay đổi nền trang chưa được hỗ trợ.",
        "collab.gate.variablesThemes" => "Biến và chủ đề chưa được hỗ trợ.",
        "collab.gate.components" => "Thay đổi sổ đăng ký thành phần chưa được hỗ trợ.",
        "collab.gate.uikit" => "Thay đổi UIKit chưa được hỗ trợ.",
        "collab.gate.externalAssets" => "Chưa thể nhập ảnh, SVG, HTML và tài nguyên bên ngoài.",
        "collab.gate.clipboardPaste" => "Dán nội dung tài liệu chưa được hỗ trợ.",
        "collab.gate.duplicate" => "Nhân đôi nút chưa được hỗ trợ.",
        "collab.gate.bulkWrite" => "Thay đổi hàng loạt bị tắt khi cộng tác.",
        "collab.gate.replaceDocument" => "Thay toàn bộ tài liệu bị tắt khi cộng tác.",
        "collab.gate.rootMetadata" => "Thay đổi siêu dữ liệu tài liệu chưa được hỗ trợ.",
        "collab.gate.typography" => "Thay đổi kiểu chữ chưa được hỗ trợ.",
        "collab.gate.effects" => "Hiệu ứng chưa được hỗ trợ.",
        "collab.gate.visibilityLocking" => "Thay đổi hiển thị và khóa chưa được hỗ trợ.",
        "collab.gate.nodeReplacement" => "Thay thế nút chưa được hỗ trợ.",
        "collab.gate.nodeProperty" => "Thuộc tính nút này chưa được hỗ trợ.",
        "collab.gate.nodeKind" => "Loại nút này chưa được hỗ trợ.",
        "collab.gate.sessionTransition" => "Chỉnh sửa tạm dừng trong khi chuẩn bị phiên.",
        "collab.gate.readOnly" => "Phiên cộng tác này chỉ đọc.",
        "collab.gate.pendingEdit" => "Hãy chờ chỉnh sửa đang chờ được xác nhận.",
        "collab.gate.aiMcp" => "Ghi tài liệu bằng AI và MCP bị tắt khi cộng tác.",
        "collab.gate.undoUnavailable" => {
            "Hoàn tác toàn cục bị tắt. Chỉ có thể hoàn tác thay đổi đã xác nhận của bạn."
        }
        "collab.gate.redoUnavailable" => "Làm lại chưa khả dụng khi cộng tác.",
        "collab.gate.ownerOnlySave" => "Chỉ chủ sở hữu mới có thể lưu tệp nguồn dùng chung.",
        "collab.gate.leaveSessionFirst" => "Rời phiên trước khi mở hoặc thay thế tài liệu khác.",
        "collab.a11y.participant" => "{{name}}, {{role}}",
        "collab.a11y.remoteCursor" => "Con trỏ của {{name}}",
        _ => return None,
    })
}
