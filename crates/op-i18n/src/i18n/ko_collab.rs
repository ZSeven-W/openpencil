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
        "collab.action.start" => "세션 만들기",
        "collab.action.join" => "세션 참여",
        "collab.home.hint" => "새 세션을 만들거나 초대로 참여하세요.",
        "collab.create.choose" => "다른 사용자의 연결 방식을 선택하세요.",
        "collab.action.leave" => "세션 나가기",
        "collab.action.retry" => "다시 시도",
        "collab.action.cancel" => "취소",
        "collab.action.connect" => "연결",
        "collab.action.copyInviteCode" => "초대 코드 복사",
        "collab.action.copyInviteLink" => "초대 링크 복사",
        "collab.action.findNearby" => "주변 찾기",
        "collab.join.code" => "초대 코드 또는 IP 주소",
        "collab.join.codePlaceholder" => "A2C4E6G8J0 또는 192.168.1.8:43120",
        "collab.join.publicHint" => "초대 코드로 인터넷을 통해 안전하게 연결할 수 있습니다.",
        "collab.join.nearby" => "주변 세션",
        "collab.session.invite" => "공개 초대 코드",
        "collab.session.preparingInvite" => "안전한 초대 준비 중…",
        "collab.session.publicReady" => "공개 릴레이 준비 완료",
        "collab.session.region" => "홈 릴레이 지역",
        "collab.connection.lan" => "로컬 네트워크",
        "collab.connection.relay" => "공개 릴레이",
        "collab.region.china" => "중국",
        "collab.region.global" => "글로벌",
        "collab.error.inviteUnavailable" => "이 초대를 사용할 수 없거나 만료되었습니다.",
        "collab.error.relayUnavailable" => "공개 릴레이를 일시적으로 사용할 수 없습니다.",
        "collab.error.inviteInvalid" => "초대 코드가 유효하지 않습니다. 확인 후 다시 시도하세요.",
        "collab.error.inviteExpired" => {
            "초대 코드가 만료되었습니다. 소유자에게 새 코드를 요청하세요."
        }
        "collab.error.relayNotConfigured" => "이 기기에는 공용 릴레이가 구성되어 있지 않습니다.",
        "collab.error.regionUnavailable" => "초대의 홈 릴레이 지역을 사용할 수 없습니다.",
        "collab.error.secureKeyUnavailable" => {
            "기기 보안 키에 액세스할 수 없습니다. 키체인 접근 권한을 확인한 후 다시 시도하세요."
        }
        "collab.error.rateLimited" => "연결 시도가 너무 많습니다. 잠시 후 다시 시도하세요.",
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
        "collab.reject.conflictDetail" => "삭제됨: {{node}}의 {{fields}}.",
        "collab.action.reapply" => "내 편집 다시 적용",
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
        "collab.status.localEditPreserved" => {
            "원격 버전을 적용했습니다. “실행 취소”로 로컬 변경 사항을 복원할 수 있습니다."
        }
        "collab.a11y.participant" => "{{name}}, {{role}}",
        "collab.a11y.remoteCursor" => "{{name}}의 커서",
        "icon.catalogLoading" => "아이콘 카탈로그를 불러오는 중…",
        "sceneTemplate.documentUnavailable" => {
            "이 템플릿의 문서를 불러오지 못했습니다. 다시 시도하세요."
        }
        "sceneTemplate.deleteFailed" => "해당 템플릿을 삭제하지 못했습니다. 다시 시도해 주세요.",
        "sceneTemplate.item.soundingNavyDeck.title" => "수심도 · 전략 제안 덱",
        "sceneTemplate.item.soundingNavyDeck.summary" => "해도의 따뜻한 백색에 강청색과 먹빛 남색. 각 장은 결론으로 시작하고 출처가 붙은 수치 하나로 뒷받침한다. 수심 단면 막대와 항적 목차가 일곱 장을 관통하는, 컨설팅 산출물과 전략 보고용.",
        "sceneTemplate.item.tidemarkSlateDeck.title" => "조수 자국 슬레이트 · 데이터 리뷰 덱",
        "sceneTemplate.item.tidemarkSlateDeck.summary" => "슬레이트 회색 바탕의 전경 타일, 추세선, 명세표, 위험 목록, 스윔레인 로드맵. 일곱 장으로 데이터 리뷰를 한 바퀴 도는, 분기 회고와 지표 정례 회의용.",
        "sceneTemplate.item.banxinRuleDeck.title" => "판심과 계선 · 중국어 조판 덱",
        "sceneTemplate.item.banxinRuleDeck.summary" => "판심, 계선, 두주 주석, 어미 쪽번호로 세운 중국어 본문 조판. 본문은 한 줄 서른 자를 넘기지 않고 두 단 대조와 인용 쪽이 읽기 좋은, 강의와 세미나용.",
        "sceneTemplate.item.gridpaperGraphiteDeck.title" => "모눈 그래파이트 · 학술 발표 덱",
        "sceneTemplate.item.gridpaperGraphiteDeck.summary" => "모눈종이 바탕에 흑연빛 글자. 문제와 공백, 방법, 그림과 표로 나눈 결과 두 장, 한계, 결론, 참고문헌까지 여덟 장을 심사 순서대로 배치한, 착수 발표와 논문 심사용.",
        "sceneTemplate.item.dossierLinenDeck.title" => "리넨 도시에 · 문서 메모 덱",
        "sceneTemplate.item.dossierLinenDeck.summary" => "리넨 종이 서류철의 질감으로 표지, 배경, 현황 데이터, 분석, 안 비교에서 결의까지. 그 자체로 읽히는 메모로 쓴 여덟 장, 의사결정 검토용.",
        "sceneTemplate.item.ledgerTickDeck.title" => "장부 눈금 · 경쟁 매트릭스 덱",
        "sceneTemplate.item.ledgerTickDeck.summary" => "장부 괘선 위의 평가 기준, 주 매트릭스, 분위 눈금, 격차와 강점 대조. 일곱 장으로 경쟁 비교를 대조 가능한 장부처럼 풀어내는, 제품 선정과 시장 분석용.",
        "sceneTemplate.item.brandConceptSheet.title" => "브랜드 콘셉트 시트",
        "sceneTemplate.item.brandConceptSheet.summary" => "주 로크업, 구성 논리, 흑백 반전, 최소 크기 생존 여부를 한 번에 검토하는 가로형 보드. 첫 콘셉트 전달용.",
        "sceneTemplate.item.logoQaBoard.title" => "로고 융합 QA 보드",
        "sceneTemplate.item.logoQaBoard.summary" => "네 칸에서 의미의 공동 부담, 구조적 의존, 실루엣의 통일, 보조 해석이 덧붙인 것이 아니라 발견한 것처럼 보이는지를 확인합니다.",
        "account.mcpToken" => "MCP 토큰",
        "menu.saveAsTemplate" => "템플릿으로 저장…",
        "menu.saveAsTemplate.saved" => "템플릿으로 저장됨",
        "menu.saveAsTemplate.failed" => "템플릿을 저장할 수 없음",
        "assetCenter.template.mine" => "내 템플릿",
        "assetCenter.template.builtIn" => "기본 템플릿",
        "ai.designProgress.detail.failureReason" => "실패 원인: {{reason}}",
        "ai.designProgress.detail.noDiagnostic" => "에이전트가 오류 설명을 반환하지 않고 실패했습니다.",
        "ai.designProgress.detail.noResult" => "이 섹션의 결과를 반환하기 전에 에이전트가 중지되었습니다.",
        "ai.designProgress.detail.connectionClosed" => "이 섹션의 결과가 반환되기 전에 에이전트 연결이 끊겼습니다.",
        "ai.designProgress.detail.stoppedByUser" => "사용자가 중지하여 이 섹션이 완료되지 않았습니다.",
        "builtin.modelsOnePerLine" => "모델 ID, 한 줄에 하나씩",
        "builtin.typeModelManually" => "모델 목록을 가져오지 못했습니다. 모델 ID를 한 줄에 하나씩 입력하세요",
        "settings.provider.deepSeekHarness" => "DeepSeek Harness 모델",
        "settings.agents.deleteProvider" => "프로바이더 삭제",
        _ => return None,
    })
}
