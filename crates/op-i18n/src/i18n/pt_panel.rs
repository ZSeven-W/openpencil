//! Overflow-shard strings for this locale.
//!
//! The main table sits at the repo's 800-line file cap, so `pt_git`
//! falls through here for the `imagePanel.*` popover keys and the
//! `providerProbe.*` keys the Antigravity / Grok Build CLI probes emit.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "imagePanel.searchPlaceholder" => "Pesquisar imagens…",
        "imagePanel.searching" => "Pesquisando…",
        "imagePanel.noResults" => "Nenhum resultado",
        "imagePanel.searchPrompt" => "Pesquise imagens",
        "imagePanel.sourceNotice" => {
            "Imagens de {{source}}. Licença livre — verifique a licença antes de usar."
        }
        "imagePanel.genNotConfigured" => "Geração de imagens não configurada",
        "imagePanel.openSettings" => "Abrir configurações",
        "imagePanel.promptPlaceholder" => "Descreva a imagem…",
        "providerProbe.connectedViaCli" => "Conectado via CLI do {{name}}",
        "providerProbe.cliExitedWithError" => "A CLI do {{name}} terminou com erro",
        "providerProbe.cliNoVersionOutput" => "A CLI do {{name}} não produziu informação de versão",
        "providerProbe.modelQueryFailed" => "A consulta de modelos do {{name}} falhou ou expirou",
        "providerProbe.modelQueryFailedRunLogin" => "A consulta de modelos do {{name}} falhou. Execute {{command}} uma vez para autenticar.",
        "providerProbe.modelQueryNeedsAuth" => "A consulta de modelos do {{name}} exige autenticação. Execute {{command}} uma vez para entrar.",
        "providerProbe.unrecognizedModelCatalog" => "{{name}} devolveu um catálogo de modelos não reconhecido",
        "promptCenter.title" => "Central de prompts",
        "promptCenter.searchPlaceholder" => "Pesquisar prompts…",
        "promptCenter.category.all" => "Tudo",
        "promptCenter.category.starter" => "Início rápido",
        "promptCenter.category.mobileApp" => "App móvel",
        "promptCenter.category.webPage" => "Página web",
        "promptCenter.category.dashboard" => "Painel",
        "promptCenter.category.component" => "Componente",
        "promptCenter.category.modify" => "Modificar",
        "promptCenter.category.custom" => "Meus prompts",
        "promptCenter.empty" => "Nenhum prompt correspondente",
        "promptCenter.saveCurrent" => "Salvar a entrada atual como prompt",
        "promptCenter.saveTitlePlaceholder" => "Título do prompt",
        "promptCenter.save" => "Salvar",
        "promptCenter.cancel" => "Cancelar",
        "promptCenter.delete" => "Excluir",
        "promptCenter.screens" => "{{count}} telas",
        "promptCenter.freeform" => "Forma livre",
        "promptCenter.item.wander.title" => "Wander · Roteiros de viagem",
        "promptCenter.item.forage.title" => "Forage · Receitas da estação",
        "promptCenter.item.still.title" => "Still · Meditação e sono",
        "promptCenter.item.hearth.title" => "Hearth · Casa inteligente",
        "promptCenter.item.meteo.title" => "Meteo · Clima imersivo",
        "promptCenter.item.marginalia.title" => "Marginalia · Leitura e anotações",
        "promptCenter.item.lingua.title" => "Lingua · Aprendizado de idiomas",
        "promptCenter.item.daybreak.title" => "Daybreak · Pedido de café",
        "promptCenter.item.verdant.title" => "Verdant · Cuidados com plantas",
        "promptCenter.item.companion.title" => "Companion · Vida com pets",
        "promptCenter.item.relic.title" => "Relic · Mercado selecionado de usados",
        "promptCenter.item.nocturne.title" => "Nocturne · Guia de observação das estrelas",
        "promptCenter.item.marquee.title" => "Marquee · Lista de filmes",
        "promptCenter.item.ritual.title" => "Ritual · Criação de hábitos",
        "promptCenter.item.ember.title" => "Ember · Diário de humor",
        "promptCenter.item.volt.title" => "Volt · Companheiro para veículo elétrico",
        "promptCenter.item.aloft.title" => "Aloft · Rastreamento de voos",
        "promptCenter.item.gallery.title" => "Gallery · Exposições e cultura",
        "promptCenter.item.nightcap.title" => "Nightcap · Coquetelaria em casa",
        "promptCenter.item.bloom.title" => "Bloom · Registro do crescimento familiar",
        "promptCenter.item.extremeWeather.title" => "Extremo · App de clima",
        "promptCenter.item.extremeNowPlaying.title" => "Extremo · Em reprodução",
        "promptCenter.item.extremeDailyApp.title" => "Extremo · Abrir todos os dias",
        "promptCenter.item.extremeCalendar.title" => "Extremo · Reinventar o calendário",
        "promptCenter.item.extremeCalm.title" => "Extremo · Uma tela de calma",
        "promptCenter.item.webOrbit.title" => "Orbit · Página do espaço de trabalho com IA",
        "promptCenter.item.webAtelier.title" => "Atelier · Comércio de móveis",
        "promptCenter.item.dashboardPulse.title" => "Pulse · Painel de análise de crescimento",
        "promptCenter.item.dashboardSentinel.title" => "Sentinel · Operações logísticas",
        "promptCenter.item.componentDataGrid.title" => "Gridworks · Tabela de dados empresarial",
        "promptCenter.item.componentFormLab.title" => {
            "Form Lab · Sistema de componentes de formulário"
        }
        "promptCenter.item.modifyPolishCurrent.title" => "Aprimorar a tela atual",
        "promptCenter.item.modifyCompleteStates.title" => "Completar estados dos componentes",
        "collab.ownerConfirm.title" => "Confirme a quem você vai se juntar",
        "collab.ownerConfirm.hint" => "Nada desta sessão foi carregado ainda.",
        "collab.ownerConfirm.account" => "Conta verificada",
        "collab.ownerConfirm.device" => "Dispositivo verificado",
        "collab.ownerConfirm.claimedName" => "Nome escolhido por esta conta (não verificado)",
        "collab.action.confirmOwner" => "Entrar nesta sessão",
        "collab.action.rejectOwner" => "Não entrar",
        "collab.error.ownerNotConfirmed" => "Você não confirmou o anfitrião, então nada foi carregado.",
        "sceneTemplate.title" => "Modelos de cenas",
        "sceneTemplate.searchPlaceholder" => "Pesquisar cenas ou modelos…",
        "sceneTemplate.empty" => "Nenhum modelo correspondente",
        "sceneTemplate.frames" => "Páginas: {{count}}",
        "sceneTemplate.filter.all" => "Tudo",
        "sceneTemplate.scene.tutorial" => "Tutorial",
        "sceneTemplate.scene.comparison" => "Comparativo",
        "sceneTemplate.scene.carousel" => "Cards de conhecimento",
        "sceneTemplate.scene.slides" => "PPT",
        "sceneTemplate.item.screenshotTutorial.title" => "Tutorial com capturas · 3 passos",
        "sceneTemplate.item.screenshotTutorial.summary" => {
            "Capa, três passos e uma chamada para ação no final. Substitua as capturas de tela e os textos para publicar."
        }
        "sceneTemplate.item.knowledgeCarousel.title" => "Carrossel de conhecimento e ideias",
        "sceneTemplate.item.knowledgeCarousel.summary" => {
            "Capa, três pontos e uma página de resumo, ideal para transformar uma ideia em cards deslizáveis."
        }
        "sceneTemplate.item.beforeAfter.title" => "Comparativo antes e depois",
        "sceneTemplate.item.beforeAfter.summary" => {
            "Comparação lado a lado do antes e depois, com notas das mudanças; ideal para retrospectivas e portfólios."
        }
        "sceneTemplate.item.slideDeck.title" => "Apresentação · 6 slides",
        "sceneTemplate.item.slideDeck.summary" => {
            "Capa, agenda, pontos-chave, dados, gráfico e encerramento, no formato 16:9. Substitua os textos e apresente."
        }
        "fileMenu.newFromTemplate" => "Novo a partir de um modelo",
        _ => return super::pt_collab::lookup(key),
    })
}
