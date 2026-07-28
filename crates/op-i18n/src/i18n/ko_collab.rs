//! 협업 UI 문자열.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "collab.topbar.collaborate" => "협업",
        "collab.topbar.starting" => "협업을 시작하는 중…",
        "collab.topbar.joining" => "참여하는 중…",
        "collab.topbar.authenticating" => "인증하는 중…",
        "collab.topbar.connected" => "연결됨",
        "collab.topbar.reconnecting" => "다시 연결하는 중…",
        "collab.topbar.readOnly" => "읽기 전용",
        "collab.topbar.ended" => "세션 종료됨",
        "collab.topbar.participants" => "참여자 {{count}}명",
        "collab.topbar.unavailable" => "이 빌드에서는 협업을 사용할 수 없습니다",
        "collab.action.start" => "세션 시작",
        "collab.action.join" => "세션 참여",
        "collab.action.leave" => "세션 나가기",
        "collab.action.retry" => "다시 시도",
        "collab.action.cancel" => "취소",
        "collab.action.connect" => "연결",
        "collab.action.discardPending" => "대기 중 편집 버리기",
        "collab.action.saveAsFork" => "분기본으로 저장",
        "collab.action.approveEditor" => "편집자로 승인",
        "collab.action.approveViewer" => "뷰어로 승인",
        "collab.action.rejectAdmission" => "거부",
        "collab.admission.request" => "인증된 참여자가 액세스를 요청하고 있습니다.",
        "collab.join.title" => "협업 세션 참여",
        "collab.join.discovering" => "로컬 네트워크에서 세션을 찾는 중…",
        "collab.join.noSessions" => "로컬 세션을 찾지 못했습니다",
        "collab.join.address" => "IP 주소 및 포트",
        "collab.join.addressPlaceholder" => "192.168.1.8:43120",
        "collab.join.authenticating" => "보안 세션을 확인하는 중…",
        "collab.join.incompatible" => "이 세션은 호환되지 않는 버전을 사용합니다",
        "collab.join.signInRequired" => "세션을 시작하거나 참여하려면 로그인하세요",
        "collab.session.title" => "협업",
        "collab.session.name" => "세션: {{name}}",
        "collab.session.shareAddress" => "공유 주소",
        "collab.session.role.owner" => "소유자",
        "collab.session.role.editor" => "편집자",
        "collab.session.role.viewer" => "뷰어",
        "collab.session.pending" => "소유자가 편집을 확인하기를 기다리는 중…",
        "collab.status.disconnectedReadOnly" => {
            "연결이 끊겼습니다. 다시 연결하는 동안 편집이 일시 중지됩니다."
        }
        "collab.status.ticketExpired" => "협업 로그인이 만료되었습니다. 다시 로그인하세요.",
        "collab.status.ownerLeft" => {
            "소유자가 나가 세션이 종료되었습니다. 별도 사본을 저장할 수 있습니다."
        }
        "collab.status.epochChanged" => {
            "소유자가 새 세션을 시작했습니다. 대기 중인 편집은 제출되지 않았습니다."
        }
        "collab.status.undoConflict" => "나중에 같은 필드가 편집되어 이 변경을 취소할 수 없습니다.",
        "collab.status.unsupportedEdit" => {
            "협업에서 아직 지원하지 않는 편집이므로 적용되지 않았습니다."
        }
        "collab.status.profileUnavailable" => "프로필 이미지를 불러올 수 없어 이니셜을 표시합니다.",
        "collab.reject.staleBase" => "문서가 먼저 변경되었습니다. 동기화 후 다시 시도합니다.",
        "collab.reject.readOnly" => "이 세션에서는 보기 권한만 있습니다.",
        "collab.reject.unsupported" => "소유자가 이 편집을 지원하지 않습니다.",
        "collab.reject.conflict" => "이 편집이 최신 변경과 충돌합니다.",
        "collab.reject.resourceLimit" => "이 편집이 세션 제한을 초과합니다.",
        "collab.reject.authentication" => "협업 인증이 더 이상 유효하지 않습니다.",
        "collab.reject.unknown" => "소유자가 이 편집을 거부했습니다.",
        "collab.gate.pages" => "페이지 변경은 협업에서 아직 지원되지 않습니다.",
        "collab.gate.pageBackground" => "페이지 배경 변경은 협업에서 아직 지원되지 않습니다.",
        "collab.gate.variablesThemes" => "변수와 테마는 협업에서 아직 지원되지 않습니다.",
        "collab.gate.components" => "컴포넌트 레지스트리 변경은 협업에서 아직 지원되지 않습니다.",
        "collab.gate.uikit" => "UIKit 변경은 협업에서 아직 지원되지 않습니다.",
        "collab.gate.externalAssets" => "이미지, SVG, HTML 및 외부 자산은 아직 가져올 수 없습니다.",
        "collab.gate.clipboardPaste" => "문서 내용 붙여넣기는 협업에서 아직 지원되지 않습니다.",
        "collab.gate.duplicate" => "노드 복제는 협업에서 아직 지원되지 않습니다.",
        "collab.gate.bulkWrite" => "협업 중에는 대량 문서 변경이 비활성화됩니다.",
        "collab.gate.replaceDocument" => "협업 중에는 전체 문서를 바꿀 수 없습니다.",
        "collab.gate.rootMetadata" => "문서 메타데이터 변경은 협업에서 아직 지원되지 않습니다.",
        "collab.gate.typography" => "타이포그래피 변경은 협업에서 아직 지원되지 않습니다.",
        "collab.gate.effects" => "효과는 협업에서 아직 지원되지 않습니다.",
        "collab.gate.visibilityLocking" => "표시 및 잠금 변경은 협업에서 아직 지원되지 않습니다.",
        "collab.gate.nodeReplacement" => "노드 교체는 협업에서 아직 지원되지 않습니다.",
        "collab.gate.nodeProperty" => "이 노드 속성은 협업에서 아직 지원되지 않습니다.",
        "collab.gate.nodeKind" => "이 노드 유형은 협업에서 아직 지원되지 않습니다.",
        "collab.gate.sessionTransition" => "협업 세션을 준비하는 동안 편집이 일시 중지됩니다.",
        "collab.gate.readOnly" => "이 협업 세션은 읽기 전용입니다.",
        "collab.gate.pendingEdit" => "대기 중인 편집이 확인될 때까지 다음 변경을 기다리세요.",
        "collab.gate.aiMcp" => "협업 중에는 AI 및 MCP 문서 쓰기가 비활성화됩니다.",
        "collab.gate.undoUnavailable" => {
            "협업에서는 전체 실행 취소가 비활성화됩니다. 확인된 본인 변경만 취소할 수 있습니다."
        }
        "collab.gate.redoUnavailable" => "협업에서는 다시 실행을 아직 사용할 수 없습니다.",
        "collab.gate.ownerOnlySave" => "공유 원본 파일은 소유자만 저장할 수 있습니다.",
        "collab.gate.leaveSessionFirst" => "다른 문서를 열거나 바꾸기 전에 협업 세션을 나가세요.",
        "collab.a11y.participant" => "{{name}}, {{role}}",
        "collab.a11y.remoteCursor" => "{{name}}의 커서",
        _ => return None,
    })
}
