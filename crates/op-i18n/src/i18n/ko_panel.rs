//! Overflow-shard strings for this locale.
//!
//! The main table sits at the repo's 800-line file cap, so `ko_git`
//! falls through here for the `imagePanel.*` popover keys and the
//! `providerProbe.*` keys the Antigravity / Grok Build CLI probes emit.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "imagePanel.searchPlaceholder" => "이미지 검색…",
        "imagePanel.searching" => "검색 중…",
        "imagePanel.noResults" => "결과가 없습니다",
        "imagePanel.searchPrompt" => "이미지를 검색하세요",
        "imagePanel.sourceNotice" => {
            "{{source}} 제공 이미지. 자유 라이선스 — 사용 전에 라이선스를 확인하세요."
        }
        "imagePanel.genNotConfigured" => "이미지 생성이 설정되지 않았습니다",
        "imagePanel.openSettings" => "설정 열기",
        "imagePanel.promptPlaceholder" => "이미지를 설명하세요…",
        "providerProbe.connectedViaCli" => "{{name}} CLI를 통해 연결됨",
        "providerProbe.cliExitedWithError" => "{{name}} CLI가 오류와 함께 종료되었습니다",
        "providerProbe.cliNoVersionOutput" => "{{name}} CLI가 버전 정보를 출력하지 않았습니다",
        "providerProbe.modelQueryFailed" => "{{name}} 모델 조회에 실패했거나 시간이 초과되었습니다",
        "providerProbe.modelQueryFailedRunLogin" => {
            "{{name}} 모델 조회에 실패했습니다. {{command}}을(를) 한 번 실행해 인증하세요."
        }
        "providerProbe.modelQueryNeedsAuth" => {
            "{{name}} 모델 조회에는 인증이 필요합니다. {{command}}을(를) 한 번 실행해 로그인하세요."
        }
        "providerProbe.unrecognizedModelCatalog" => {
            "{{name}}이(가) 인식할 수 없는 모델 목록을 반환했습니다"
        }
        _ => return super::ko_collab::lookup(key),
    })
}
