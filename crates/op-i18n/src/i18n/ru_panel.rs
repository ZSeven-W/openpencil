//! Overflow-shard strings for this locale.
//!
//! The main table sits at the repo's 800-line file cap, so `ru_git`
//! falls through here for the `imagePanel.*` popover keys and the
//! `providerProbe.*` keys the Antigravity / Grok Build CLI probes emit.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "imagePanel.searchPlaceholder" => "Поиск изображений…",
        "imagePanel.searching" => "Поиск…",
        "imagePanel.noResults" => "Ничего не найдено",
        "imagePanel.searchPrompt" => "Найдите изображения",
        "imagePanel.sourceNotice" => "Изображения из {{source}}. Свободная лицензия — проверьте лицензию перед использованием.",
        "imagePanel.genNotConfigured" => "Генерация изображений не настроена",
        "imagePanel.openSettings" => "Открыть настройки",
        "imagePanel.promptPlaceholder" => "Опишите изображение…",
        "providerProbe.connectedViaCli" => "Подключено через CLI {{name}}",
        "providerProbe.cliExitedWithError" => "CLI {{name}} завершилась с ошибкой",
        "providerProbe.cliNoVersionOutput" => "CLI {{name}} не вернула сведения о версии",
        "providerProbe.modelQueryFailed" => "Запрос моделей {{name}} завершился ошибкой или тайм-аутом",
        "providerProbe.modelQueryFailedRunLogin" => "Запрос моделей {{name}} завершился ошибкой. Выполните {{command}} один раз для аутентификации.",
        "providerProbe.modelQueryNeedsAuth" => "Для запроса моделей {{name}} требуется аутентификация. Выполните {{command}} один раз, чтобы войти.",
        "providerProbe.unrecognizedModelCatalog" => "{{name}} вернул нераспознанный каталог моделей",
        _ => return super::ru_collab::lookup(key),
    })
}
