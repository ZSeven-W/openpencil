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
        "collab.action.start" => "Начать сеанс",
        "collab.action.join" => "Присоединиться",
        "collab.action.leave" => "Покинуть сеанс",
        "collab.action.retry" => "Повторить",
        "collab.action.cancel" => "Отмена",
        "collab.action.connect" => "Подключиться",
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
        "collab.a11y.participant" => "{{name}}, {{role}}",
        "collab.a11y.remoteCursor" => "Курсор пользователя {{name}}",
        _ => return None,
    })
}
