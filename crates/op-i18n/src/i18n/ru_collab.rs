//! Строки совместной работы.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "collab.topbar.collaborate" => "Совместная работа",
        "collab.topbar.starting" => "Запуск совместной работы…",
        "collab.topbar.joining" => "Подключение…",
        "collab.topbar.authenticating" => "Аутентификация…",
        "collab.topbar.connected" => "Подключено",
        "collab.topbar.reconnecting" => "Повторное подключение…",
        "collab.topbar.readOnly" => "Только чтение",
        "collab.topbar.ended" => "Сеанс завершён",
        "collab.topbar.participants" => "Участников: {{count}}",
        "collab.topbar.unavailable" => "Совместная работа недоступна в этой сборке",
        "collab.action.start" => "Создать сеанс",
        "collab.action.join" => "Присоединиться",
        "collab.home.hint" => "Создайте новый сеанс или присоединитесь по приглашению.",
        "collab.create.choose" => "Выберите, как другие участники смогут подключаться.",
        "collab.action.leave" => "Покинуть сеанс",
        "collab.action.retry" => "Повторить",
        "collab.action.cancel" => "Отмена",
        "collab.action.connect" => "Подключиться",
        "collab.action.copyInviteCode" => "Скопировать код приглашения",
        "collab.action.copyInviteLink" => "Скопировать ссылку приглашения",
        "collab.action.findNearby" => "Найти поблизости",
        "collab.join.code" => "Код приглашения или IP-адрес",
        "collab.join.codePlaceholder" => "A2C4E6G8J0 или 192.168.1.8:43120",
        "collab.join.publicHint" => "Коды приглашений безопасно подключают через Интернет.",
        "collab.join.nearby" => "Сеансы поблизости",
        "collab.session.invite" => "Публичный код приглашения",
        "collab.session.preparingInvite" => "Подготовка безопасного приглашения…",
        "collab.session.publicReady" => "Публичный ретранслятор готов",
        "collab.session.region" => "Домашний регион ретранслятора",
        "collab.connection.lan" => "Локальная сеть",
        "collab.connection.relay" => "Публичный ретранслятор",
        "collab.region.china" => "Китай",
        "collab.region.global" => "Глобальный",
        "collab.error.inviteUnavailable" => "Приглашение недоступно или срок его действия истёк.",
        "collab.error.relayUnavailable" => "Публичный ретранслятор временно недоступен.",
        "collab.error.inviteInvalid" => {
            "Недействительный код приглашения. Проверьте его и попробуйте снова."
        }
        "collab.error.inviteExpired" => {
            "Срок действия кода приглашения истёк. Запросите новый у владельца."
        }
        "collab.error.relayNotConfigured" => {
            "На этом устройстве не настроен публичный ретранслятор."
        }
        "collab.error.regionUnavailable" => "Домашний регион ретранслятора приглашения недоступен.",
        "collab.error.secureKeyUnavailable" => "Защищённый ключ устройства недоступен. Проверьте доступ к связке ключей и повторите попытку.",
        "collab.error.rateLimited" => "Слишком много попыток подключения. Повторите позже.",
        "collab.action.discardPending" => "Отменить ожидающее изменение",
        "collab.action.saveAsFork" => "Сохранить как ответвление",
        "collab.action.approveEditor" => "Разрешить редактирование",
        "collab.action.approveViewer" => "Разрешить просмотр",
        "collab.action.rejectAdmission" => "Отклонить",
        "collab.admission.request" => "Аутентифицированный участник запрашивает доступ.",
        "collab.join.title" => "Присоединиться к совместному сеансу",
        "collab.join.discovering" => "Поиск сеансов в локальной сети…",
        "collab.join.noSessions" => "Локальные сеансы не найдены",
        "collab.join.address" => "IP-адрес и порт",
        "collab.join.addressPlaceholder" => "192.168.1.8:43120",
        "collab.join.authenticating" => "Проверка защищённого сеанса…",
        "collab.join.incompatible" => "Сеанс использует несовместимую версию",
        "collab.join.signInRequired" => "Войдите, чтобы начать сеанс или присоединиться",
        "collab.session.title" => "Совместная работа",
        "collab.session.name" => "Сеанс: {{name}}",
        "collab.session.shareAddress" => "Адрес для подключения",
        "collab.session.role.owner" => "Владелец",
        "collab.session.role.editor" => "Редактор",
        "collab.session.role.viewer" => "Наблюдатель",
        "collab.session.pending" => "Ожидание подтверждения изменения владельцем…",
        "collab.status.disconnectedReadOnly" => {
            "Соединение потеряно. Редактирование приостановлено до переподключения."
        }
        "collab.status.ticketExpired" => "Срок входа для совместной работы истёк. Войдите снова.",
        "collab.status.ownerLeft" => {
            "Владелец вышел, сеанс завершён. Можно сохранить отдельную копию."
        }
        "collab.status.epochChanged" => {
            "Владелец начал новый сеанс. Ожидающее изменение не отправлено."
        }
        "collab.status.undoConflict" => {
            "Изменение нельзя отменить: это поле позже изменил другой участник."
        }
        "collab.status.unsupportedEdit" => {
            "Это изменение пока не поддерживается и не было применено."
        }
        "collab.status.profileUnavailable" => "Фото профиля недоступно; показаны инициалы.",
        "collab.reject.staleBase" => {
            "Документ уже изменился. Выполняется синхронизация перед повтором."
        }
        "collab.reject.readOnly" => "В этом сеансе у вас доступ только для чтения.",
        "collab.reject.unsupported" => "Владелец не поддерживает это изменение.",
        "collab.reject.conflict" => "Изменение конфликтует с более новой версией.",
        "collab.reject.conflictDetail" => "Отменено: {{fields}} у «{{node}}».",
        "collab.action.reapply" => "Повторно применить моё изменение",
        "collab.reject.resourceLimit" => "Изменение превышает ограничения сеанса.",
        "collab.reject.authentication" => {
            "Разрешение на совместную работу больше не действительно."
        }
        "collab.reject.unknown" => "Владелец отклонил изменение.",
        "collab.gate.pages" => "Изменения страниц пока не поддерживаются.",
        "collab.gate.pageBackground" => "Фон страницы пока нельзя изменять совместно.",
        "collab.gate.variablesThemes" => "Переменные и темы пока не поддерживаются.",
        "collab.gate.components" => "Изменения реестра компонентов пока не поддерживаются.",
        "collab.gate.uikit" => "Изменения UIKit пока не поддерживаются.",
        "collab.gate.externalAssets" => {
            "Импорт изображений, SVG, HTML и внешних ресурсов пока недоступен."
        }
        "collab.gate.clipboardPaste" => "Вставка содержимого документа пока не поддерживается.",
        "collab.gate.duplicate" => "Дублирование узлов пока не поддерживается.",
        "collab.gate.bulkWrite" => "Массовые изменения отключены во время совместной работы.",
        "collab.gate.replaceDocument" => {
            "Замена всего документа отключена во время совместной работы."
        }
        "collab.gate.rootMetadata" => "Изменения метаданных документа пока не поддерживаются.",
        "collab.gate.typography" => "Изменения типографики пока не поддерживаются.",
        "collab.gate.effects" => "Эффекты пока не поддерживаются.",
        "collab.gate.visibilityLocking" => "Видимость и блокировка пока не поддерживаются.",
        "collab.gate.nodeReplacement" => "Замена узлов пока не поддерживается.",
        "collab.gate.nodeProperty" => "Это свойство узла пока не поддерживается.",
        "collab.gate.nodeKind" => "Этот тип узла пока не поддерживается.",
        "collab.gate.sessionTransition" => {
            "Редактирование приостановлено на время подготовки сеанса."
        }
        "collab.gate.readOnly" => "Этот совместный сеанс доступен только для чтения.",
        "collab.gate.pendingEdit" => "Дождитесь подтверждения ожидающего изменения.",
        "collab.gate.aiMcp" => "Запись документа через ИИ и MCP отключена.",
        "collab.gate.undoUnavailable" => {
            "Глобальная отмена отключена. Можно отменять только свои подтверждённые изменения."
        }
        "collab.gate.redoUnavailable" => "Повтор действий в совместной работе пока недоступен.",
        "collab.gate.ownerOnlySave" => "Общий исходный файл может сохранять только владелец.",
        "collab.gate.leaveSessionFirst" => {
            "Покиньте сеанс перед открытием или заменой другого документа."
        }
        "collab.status.localEditPreserved" => {
            "Удалённая версия применена. Используйте «Отменить», чтобы восстановить локальные изменения."
        }
        "collab.a11y.participant" => "{{name}}, {{role}}",
        "collab.a11y.remoteCursor" => "Курсор пользователя {{name}}",
        "icon.catalogLoading" => "Загрузка каталога значков…",
        "sceneTemplate.documentUnavailable" => "Не удалось загрузить документ этого шаблона. Попробуйте снова.",
        "sceneTemplate.deleteFailed" => "Не удалось удалить этот шаблон. Попробуйте снова.",
        "sceneTemplate.item.soundingNavyDeck.title" => "Карта промеров · стратегическая презентация",
        "sceneTemplate.item.soundingNavyDeck.summary" => "Тёплая белизна морской карты со стальным и чернильно-синим: каждая страница начинается с вывода и подкрепляется одним показателем с источником. Столбцы профиля глубин и оглавление-галс проходят через семь страниц — для консалтинговых материалов и стратегических разборов.",
        "sceneTemplate.item.tidemarkSlateDeck.title" => "Приливный сланец · презентация обзора данных",
        "sceneTemplate.item.tidemarkSlateDeck.summary" => "На сланцево-сером: обзорные плитки, линия тренда, детальная таблица, список рисков и дорожная карта по дорожкам. Семь страниц проходят полный обзор данных — для квартальных ретроспектив и встреч по метрикам.",
        "sceneTemplate.item.banxinRuleDeck.title" => "Наборная полоса и линейки · презентация китайской типографики",
        "sceneTemplate.item.banxinRuleDeck.summary" => "Построена на классической китайской полосе: наборное поле, линейки колонок, заметки на верхнем поле и колонцифра-рыбий хвост. Строки текста не длиннее тридцати иероглифов, с разворотом в две колонки и страницей цитаты — для лекций и семинаров.",
        "sceneTemplate.item.gridpaperGraphiteDeck.title" => "Графит на клетчатой бумаге · презентация защиты",
        "sceneTemplate.item.gridpaperGraphiteDeck.summary" => "Графитовый набор на клетчатой бумаге: проблема и пробел, метод, две страницы результатов, ограничения, вывод и список литературы. Восемь страниц в порядке защиты — для заявок и защиты диссертации.",
        "sceneTemplate.item.dossierLinenDeck.title" => "Льняное досье · презентация служебной записки",
        "sceneTemplate.item.dossierLinenDeck.summary" => "Досье на льняной бумаге: титульный лист, предыстория, текущие данные, анализ, сравнение вариантов и решение. Восемь страниц читаются как самостоятельная записка — для разбора решений.",
        "sceneTemplate.item.ledgerTickDeck.title" => "Разлиновка гроссбуха · презентация конкурентной матрицы",
        "sceneTemplate.item.ledgerTickDeck.summary" => "Критерии оценки, основная матрица, квантильные шкалы и сопоставление разрывов и сильных сторон на линовке гроссбуха. Семь страниц рассказывают сравнение конкурентов как сверяемый счёт — для выбора поставщика и анализа рынка.",
        "sceneTemplate.item.brandConceptSheet.title" => "Лист концепции бренда",
        "sceneTemplate.item.brandConceptSheet.summary" => "Горизонтальная доска для проверки основного локапа, логики построения, монохромных инверсий и читаемости в минимальном размере — для первой передачи концепции.",
        "sceneTemplate.item.logoQaBoard.title" => "QA-доска слияния логотипа",
        "sceneTemplate.item.logoQaBoard.summary" => "Четыре ячейки проверяют общую смысловую нагрузку, структурную зависимость, единство силуэта и то, выглядит ли вторичное прочтение найденным, а не прикреплённым.",
        "account.mcpToken" => "Токены MCP",
        "menu.saveAsTemplate" => "Сохранить как шаблон…",
        "menu.saveAsTemplate.saved" => "Сохранено как шаблон",
        "menu.saveAsTemplate.failed" => "Не удалось сохранить шаблон",
        "assetCenter.template.mine" => "Мои шаблоны",
        "assetCenter.template.builtIn" => "Встроенные шаблоны",
        "ai.designProgress.detail.failureReason" => "Причина: {{reason}}",
        "ai.designProgress.detail.noDiagnostic" => "Агент завершился с ошибкой, не вернув её описание.",
        "ai.designProgress.detail.noResult" => "Агент остановился до получения результата для этого раздела.",
        "ai.designProgress.detail.connectionClosed" => "Соединение с агентом закрылось до получения результата для этого раздела.",
        "ai.designProgress.detail.stoppedByUser" => "Остановлено пользователем до завершения этого раздела.",
        "builtin.modelsOnePerLine" => "ID моделей, по одному в строке",
        "builtin.typeModelManually" => "Не удалось получить список моделей. Введите ID вручную, по одному в строке.",
        _ => return None,
    })
}
