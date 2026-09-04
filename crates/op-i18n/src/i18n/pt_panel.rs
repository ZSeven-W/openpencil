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
        "promptCenter.item.webKilnform.title" => "Kilnform · Site de infraestrutura de design",
        "promptCenter.item.webReefwright.title" => "Reefwright · Site de conhecimento de suporte com IA",
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
        "sceneTemplate.generate.placeholder" => "Descreva um tema e a IA gera a apresentação inteira",
        "sceneTemplate.generate.button" => "Gerar",
        "sceneTemplate.generate.hint" => "Um documento novo, criado a partir do seu tema como apresentação completa.",
        "sceneTemplate.generate.promptTemplate" => "Crie uma apresentação (PPT) sobre o seguinte tema: {{topic}}",
        "sceneTemplate.card.addToCanvas" => "Adicionar à tela",
        "sceneTemplate.card.generateFrom" => "Gerar com base neste",
        "sceneTemplate.generate.basis" => "Baseado em: ",
        "sceneTemplate.filter.all" => "Tudo",
        "sceneTemplate.scene.tutorial" => "Tutoriais",
        "sceneTemplate.scene.comparison" => "Comparação",
        "sceneTemplate.scene.carousel" => "Carrossel",
        "sceneTemplate.scene.slides" => "Slides",
        "sceneTemplate.scene.card" => "Cartões",
        "sceneTemplate.scene.web" => "Páginas web",
        "sceneTemplate.generate.webPromptTemplate" => "Projete uma página de destino web com várias seções sobre o seguinte tema: {{topic}}",
        "sceneTemplate.item.saasLandingOrange.title" => "Página SaaS · Laranja",
        "sceneTemplate.item.saasLandingOrange.summary" => "Uma página de marketing clara construída sobre painéis quase pretos e um único laranja: navegação, hero com captura do produto, três cartões de recursos, um passeio pelo fluxo, depoimentos e um rodapé de assinatura. Troque os textos e já é um site.",
        "sceneTemplate.item.productLandingLight.title" => "Página de produto · Clara",
        "sceneTemplate.item.productLandingLight.summary" => "Uma página de produto branco papel, com ar editorial: demonstração interativa no hero, colunas de recursos, painel de análise, comparação antes/depois e três planos de preço. Para sites SaaS e lançamentos.",
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
        "sceneTemplate.item.knowledgeCardVertical.title" => "Cartão de conhecimento · Retrato",
        "sceneTemplate.item.knowledgeCardVertical.summary" => "Um único cartão 3:4 com título, quatro pontos-chave e uma assinatura. Troque os textos e publique.",
        "sceneTemplate.item.knowledgeCardSquare.title" => "Cartão de conhecimento · Quadrado",
        "sceneTemplate.item.knowledgeCardSquare.summary" => "Um cartão 1:1 com a mesma composição, compacto para uma imagem de capa ou uma publicação social.",
        "sceneTemplate.item.pitchDeckDark.title" => "Pitch deck · Escuro",
        "sceneTemplate.item.pitchDeckDark.summary" => "Capa, problema, solução, números, roteiro e página de contato. Tipografia grande sobre fundo escuro, feito para captação e lançamentos.",
        "sceneTemplate.item.lectureDeckLight.title" => "Material de aula · Claro",
        "sceneTemplate.item.lectureDeckLight.summary" => "Capa do curso, objetivos, explicação do conceito, exercício resolvido, tabela comparativa e fechamento. Fundo branco papel, confortável durante toda a aula.",
        "sceneTemplate.item.minimalKeynote.title" => "Keynote minimalista",
        "sceneTemplate.item.minimalKeynote.summary" => "Espaço em branco, tipografia enorme e uma frase centralizada por página — nove páginas sem um único cartão e um índice só com fios e números. Para lançamentos e palestras.",
        "sceneTemplate.item.gradientTech.title" => "Tech gradiente",
        "sceneTemplate.item.gradientTech.summary" => "Fundo em gradiente escuro com cartões de vidro fosco: arquitetura, desempenho e mural de clientes. Para lançamentos de produto técnico.",
        "sceneTemplate.scene.infographic" => "Infográficos",
        "sceneTemplate.item.punchQuoteCard.title" => "Cartão de citação · Cartaz",
        "sceneTemplate.item.punchQuoteCard.summary" => "Um cartão 3:4 em fundo quase preto: duas linhas enormes sobre uma faixa amarela. Uma frase, só isso, para opiniões e citações.",
        "sceneTemplate.item.journalChecklistCard.title" => {
            "Cartão de tarefas · Base de conhecimento"
        }
        "sceneTemplate.item.journalChecklistCard.summary" => "Um cartão branco sobre fundo cinza-claro: cinco tarefas para marcar, uma etiqueta e uma citação. Para planos da semana.",
        "sceneTemplate.item.dataReportInfographic.title" => "Infográfico de resultados",
        "sceneTemplate.item.dataReportInfographic.summary" => "Uma imagem vertical para rolar: cabeçalho escuro, três números grandes, uma comparação em barras, uma divisão e três conclusões. Troque os números e publique.",
        "sceneTemplate.item.stepsFlowInfographic.title" => "Infográfico passo a passo",
        "sceneTemplate.item.stepsFlowInfographic.summary" => "Uma imagem vertical para rolar: cinco cartões numerados encadeados num fluxo, cada um com a duração, mais duas dicas. Para tutoriais e guias.",
        "sceneTemplate.item.eventPosterDeck.title" => "Deck de evento · Cartaz",
        "sceneTemplate.item.eventPosterDeck.summary" => "Capa, destaques, programação, como chegar, ingressos e encerramento. Fundo branco de galeria com blocos vermelhos e azuis, sem cantos arredondados e sem gradientes — para feiras, eventos e inaugurações.",
        "sceneTemplate.item.pitfallListInfographic.title" => "Infográfico de erros a evitar",
        "sceneTemplate.item.pitfallListInfographic.summary" => "Uma imagem vertical para rolar: seis erros ordenados por frequência, cada um com o que dá errado e o que fazer no lugar, mais uma checagem de quatro pontos antes de publicar. Só preto, branco e cinza.",
        "sceneTemplate.item.spineCultureCard.title" => {
            "Cartão de título vertical · Pigmento mineral"
        }
        "sceneTemplate.item.spineCultureCard.summary" => "Um cartão 3:4 sobre fundo de argila ocre: título chinês na vertical, reboco descascado e grãos de pigmento. Para cultura, textos longos e capas de autor.",
        "sceneTemplate.item.metricSingleCard.title" => "Cartão de valor único · Grelha Hanzi",
        "sceneTemplate.item.metricSingleCard.summary" => "Um cartão 1:1: um número enorme sobre branco puro, uma grelha suíça rigorosa e um único quadrado vermelho de sinal. Para conclusões e resultados.",
        "sceneTemplate.item.quoteFrameCard.title" => "Cartão de citação · Seda azul-verde",
        "sceneTemplate.item.quoteFrameCard.summary" => "Um cartão 4:5 sobre seda amarelecida: uma frase emoldurada e, ao pé, uma montanha de azurita e malaquita. Para excertos, entrevistas e citações.",
        "sceneTemplate.item.dailySignCard.title" => "Cartão diário · Janela de jardim",
        "sceneTemplate.item.dailySignCard.summary" => "Um cartão 3:4 sobre parede caiada com uma janela hexagonal: dentro, a data e uma linha. O vazio é o ornamento. Para posts diários e frases de marca.",
        "sceneTemplate.item.priceTierCard.title" => "Cartão de preços · Néon de arcada",
        "sceneTemplate.item.priceTierCard.summary" => "Um cartão 1:1 sobre noite azul-tinta: tabela de três níveis, contornos de tubos de néon e o seu halo. Para lojas, eventos e pacotes.",
        "sceneTemplate.item.noticeBoardCard.title" => "Cartão de aviso · Tipos de chumbo",
        "sceneTemplate.item.noticeBoardCard.summary" => "Um cartão 4:5 sobre papel de jornal: filetes de cabeçalho com registo desalinhado, cláusulas numeradas e um selo de série. Para avisos e regulamentos.",
        "sceneTemplate.item.milestoneTimelineInfographic.title" => "Infográfico de linha do tempo",
        "sceneTemplate.item.milestoneTimelineInfographic.summary" => "Uma imagem vertical para rolar: um eixo que percorre toda a altura, marcas de ano ao lado dos cartões de marcos e um fecho com o próximo passo. Para retrospectivas, história de marca e trajetórias de projeto.",
        "sceneTemplate.item.conceptContrastInfographic.title" => {
            "Infográfico de contraste de conceitos"
        }
        "sceneTemplate.item.conceptContrastInfographic.summary" => "Uma imagem vertical para rolar: primeiro a conclusão, depois um cartão de definição por conceito, um detalhamento em duas colunas por critério e, por fim, como escolher.",
        "sceneTemplate.item.rankingBoardInfographic.title" => "Infográfico de ranking Top N",
        "sceneTemplate.item.rankingBoardInfographic.summary" => "Uma imagem vertical para rolar: um quadro dourado sobre tinta — medalhas grandes para os três primeiros e contornadas do quarto ao oitavo, cada uma com quando usar e com que frequência.",
        "sceneTemplate.item.faqThreadInfographic.title" => "Infográfico de perguntas frequentes",
        "sceneTemplate.item.faqThreadInfographic.summary" => "Uma imagem vertical para rolar: seis pares de pergunta e resposta, P sólido e R contornado. Sem numeração nem ordem: cada par se sustenta sozinho.",
        "sceneTemplate.item.dataStoryInfographic.title" => "Infográfico de história de dados",
        "sceneTemplate.item.dataStoryInfographic.summary" => "Uma imagem vertical para rolar: quatro números encadeados numa linha causal, cada trecho como uma grade de dez blocos, e uma conclusão acionável no fim.",
        "sceneTemplate.item.challengeTrackerInfographic.title" => {
            "Infográfico de desafio de 30 dias"
        }
        "sceneTemplate.item.challengeTrackerInfographic.summary" => "Uma imagem vertical para rolar: uma grade de trinta quadros, seis por cinco, com marcos só nos dias 7, 15 e 30. Salve e risque um por dia.",
        "sceneTemplate.item.ecosystemMapInfographic.title" => "Infográfico de mapa de ecossistema",
        "sceneTemplate.item.ecosystemMapInfographic.summary" => "Uma imagem vertical para rolar: quatro posições de uma mesma cadeia em dois por dois, três atores em cada e as lacunas apontadas. Cartões brancos sobre ardósia.",
        "sceneTemplate.item.doDontComparison.title" => "Duas colunas: certo e errado",
        "sceneTemplate.item.doDontComparison.summary" => "Um cartão 3:4: duas maneiras de fazer a mesma coisa lado a lado, distinguidas por material e ícone em vez de vermelho contra verde — legível também para daltônicos.",
        "sceneTemplate.item.mythTruthComparison.title" => "Mitos e realidade",
        "sceneTemplate.item.mythTruthComparison.summary" => "Uma imagem alta: cinco pares «costuma-se dizer / na verdade», o mito estreito e claro à esquerda, a realidade larga e escura à direita.",
        "sceneTemplate.item.pricingTiersComparison.title" => "Comparação de planos",
        "sceneTemplate.item.pricingTiersComparison.summary" => "Um cartão 3:4: Gratuito, Pro e Equipa lado a lado, o preço como âncora, cada coluna contendo a anterior. Para páginas de preços.",
        "sceneTemplate.item.scenarioGuideComparison.title" => "Guia de escolha por situação",
        "sceneTemplate.item.scenarioGuideComparison.summary" => "Uma imagem alta: sem especificações, sete situações, cada uma com o seu veredicto. O leitor só procura a sua linha.",
        "sceneTemplate.item.specTableComparison.title" => "Tabela comparativa de especificações",
        "sceneTemplate.item.specTableComparison.summary" => "Uma imagem alta: dois candidatos numa tabela real, linha a linha, com a célula vencedora realçada em fundo escuro.",
        "sceneTemplate.item.threeWayComparison.title" => "Comparação de três opções",
        "sceneTemplate.item.threeWayComparison.summary" => "Uma imagem alta: três opções lado a lado com a recomendação ao centro; cada coluna abre com uma situação, não com um nome.",
        "sceneTemplate.item.timeShiftComparison.title" => "Há um ano e agora",
        "sceneTemplate.item.timeShiftComparison.summary" => "Um cartão 3:4: uma espinha central de rótulos, há um ano à esquerda e agora à direita, ambos os valores na mesma linha.",
        "sceneTemplate.item.tradeoffScaleComparison.title" => "Balança de prós e contras",
        "sceneTemplate.item.tradeoffScaleComparison.summary" => "Um cartão 1:1: uma trave e dois pratos — o que vale à esquerda, o que custa à direita, uma caixa vazia antes de cada linha.",
        "sceneTemplate.item.versionDiffComparison.title" => "Mudanças entre versões",
        "sceneTemplate.item.versionDiffComparison.summary" => "Um cartão 1:1: sem colunas — cada linha completa o seu próprio «antes → depois»; basta rolar.",
        "sceneTemplate.item.appOnboardingTriptych.title" => "Tríptico de onboarding de app",
        "sceneTemplate.item.appOnboardingTriptych.summary" => "Um cartão 3:4: três telemóveis lado a lado com espaços de imagem vazios. Coloque os seus três ecrãs, junte o texto e está pronto.",
        "sceneTemplate.item.diyBlueprintGuide.title" => "Guia DIY ilustrado",
        "sceneTemplate.item.diyBlueprintGuide.summary" => "Uma imagem alta em que a tabela de materiais ocupa tanto como os passos — o DIY falha na preparação, não nas mãos.",
        "sceneTemplate.item.photoCompositionTutorial.title" => {
            "Composição fotográfica com telemóvel"
        }
        "sceneTemplate.item.photoCompositionTutorial.summary" => "3:4, cinco quadros: cada um com um visor escuro e linhas-guia fluorescentes sobre o espaço da foto.",
        "sceneTemplate.item.recipeFourStep.title" => "Receita em quatro passos",
        "sceneTemplate.item.recipeFourStep.summary" => "Um cartão 4:5 em 2×2: os quatro passos num só cartão. Captura de ecrã e cozinhe — ao fogão ninguém quer virar páginas.",
        "sceneTemplate.item.skincareRoutineCards.title" => "Cartões de rotina de cuidados",
        "sceneTemplate.item.skincareRoutineCards.summary" => "4:5, seis quadros: cada passo fixa três números — quantidade, tempo de espera e se é de manhã ou à noite.",
        "sceneTemplate.item.softwareStepTutorial.title" => "Tutorial de software passo a passo",
        "sceneTemplate.item.softwareStepTutorial.summary" => "Um cartão 4:5, o único escuro da série: espaços de captura com instruções numeradas, para ferramentas e funcionalidades.",
        "sceneTemplate.item.storageMakeoverSteps.title" => "Passos de reorganização de arrumação",
        "sceneTemplate.item.storageMakeoverSteps.summary" => "3:4, seis quadros: além do gesto e da imagem, cada passo fixa um critério de conclusão e um orçamento de tempo.",
        "sceneTemplate.item.weeklyReportLesson.title" => "Lição de relatório semanal",
        "sceneTemplate.item.weeklyReportLesson.summary" => "Uma imagem alta: depois da estrutura em quatro partes entrega um esqueleto com espaços sublinhados para preencher.",
        "sceneTemplate.item.workoutBreakdownGuide.title" => "Guia de decomposição de exercícios",
        "sceneTemplate.item.workoutBreakdownGuide.summary" => "Uma imagem alta: cada movimento traz uma barra fixa de séries / repetições / descanso junto à imagem e às dicas.",
        "sceneTemplate.item.bookreviewSilkCarousel.title" => {
            "Carrossel de análise de livro / filme"
        }
        "sceneTemplate.item.bookreviewSilkCarousel.summary" => "3:4, cinco painéis: gancho, excerto anotado, três ideias, uma frase citável, fecho. Desmonta a obra em peças para levar, em vez de recontar o enredo.",
        "sceneTemplate.item.cityguideFilmCarousel.title" => "Carrossel de guia de cidade",
        "sceneTemplate.item.cityguideFilmCarousel.summary" => "3:4, sete painéis: lugares e percursos alternam — os lugares para quem sonha, o roteiro do dia e a tabela de comer e dormir para quem planeia.",
        "sceneTemplate.item.datareportGridCarousel.title" => "Carrossel de relatório de dados",
        "sceneTemplate.item.datareportGridCarousel.summary" => "3:4, seis painéis: cada painel de dados é seguido de um sem dados, para ninguém desistir no terceiro gráfico.",
        "sceneTemplate.item.opinionLongformCarousel.title" => "Carrossel de opinião longa",
        "sceneTemplate.item.opinionLongformCarousel.summary" => "3:4, seis painéis: uma matriz visual rígida do início ao fim, número e título sempre no mesmo lugar.",
        "sceneTemplate.item.qaChalkboardCarousel.title" => "Carrossel de perguntas e respostas",
        "sceneTemplate.item.qaChalkboardCarousel.summary" => "3:4, seis painéis: uma pergunta por painel, com um número-interrogação desenhado à mão no canto.",
        "sceneTemplate.item.storyNightCarousel.title" => "Carrossel de narrativa",
        "sceneTemplate.item.storyNightCarousel.summary" => "3:4, sete painéis: um balanço pessoal assente no tempo — a linha temporal do quinto painel é a parede-mestra.",
        "sceneTemplate.item.toolkitNotebookCarousel.title" => {
            "Carrossel de coletânea de ferramentas"
        }
        "sceneTemplate.item.toolkitNotebookCarousel.summary" => "3:4, seis painéis: seis ferramentas uma por painel, e o último lista-as com os números de página.",
        "sceneTemplate.item.tutorialJournalCarousel.title" => "Carrossel de tutorial",
        "sceneTemplate.item.tutorialJournalCarousel.summary" => {
            "3:4, seis painéis: um passo por painel, o dedo é a barra de progresso."
        }
        "sceneTemplate.item.yearreviewMineralCarousel.title" => "Carrossel de retrospetiva anual",
        "sceneTemplate.item.yearreviewMineralCarousel.summary" => {
            "3:4, oito painéis: painéis de números frios e painéis de reflexão quentes, alternados."
        }
        "fileMenu.newFromTemplate" => "Novo a partir de um modelo",
        "fileMenu.exportSlideshowHtml" => "Exportar apresentação HTML...",
        "fileMenu.exportPptx" => "Exportar para PowerPoint...",
        "dialog.slideshowHtmlTitle" => "Exportar apresentação",
        "dialog.slideshowHtmlSummary" => "{{count}} slides exportados para:",
        "dialog.slideshowHtmlEmpty" => "Esta apresentação não tem slides visíveis para exportar.",
        // HTML import diagnostics — one entry per `ImportWarning::code`.
        "htmlImport.warn.content.empty_input" => "O conteúdo HTML importável está indisponível.",
        "htmlImport.warn.content.empty_body" => {
            "O conteúdo importável no corpo do HTML está indisponível."
        }
        "htmlImport.warn.content.dom_depth_truncated" => {
            "O HTML aninhado além de {{max_depth}} níveis foi descartado."
        }
        "htmlImport.warn.content.node_limit_truncated" => {
            "Limite de nós atingido; o restante do conteúdo da página foi omitido."
        }
        "htmlImport.warn.content.node_limit_mapping" => {
            "Limite de nós atingido; parte da árvore HTML foi omitida."
        }
        "htmlImport.warn.content.node_limit_inline_row" => {
            "Limite de nós atingido; uma linha de formatação em linha foi omitida."
        }
        "htmlImport.warn.content.node_limit_pseudo" => {
            "Limite de nós atingido; os pseudoelementos gerados foram omitidos."
        }
        "htmlImport.warn.css.at_rule_depth_limit" => {
            "As regras CSS aninhadas além de {{max_depth}} regras @ foram ignoradas."
        }
        "htmlImport.warn.css.unterminated_rule" => "Uma regra CSS não terminada foi ignorada.",
        "htmlImport.warn.css.marker_rules_unsupported" => {
            "As regras CSS ::marker não foram importadas."
        }
        "htmlImport.warn.css.nesting_unsupported" => {
            "As regras de estilo CSS aninhadas foram ignoradas."
        }
        "htmlImport.warn.css.invalid_layer_name" => {
            "O nome de @layer inválido '{{name}}' foi ignorado."
        }
        "htmlImport.warn.css.unsupported_statement" => {
            "A instrução @{{name}} sem suporte foi ignorada."
        }
        "htmlImport.warn.css.media_without_viewport" => {
            "As regras @media sem uma área de visualização foram ignoradas."
        }
        "htmlImport.warn.css.invalid_layer_block_name" => {
            "O nome de bloco @layer inválido '{{name}}' foi ignorado."
        }
        "htmlImport.warn.css.unsupported_container_block" => "O bloco @container foi ignorado.",
        "htmlImport.warn.css.unsupported_block" => "O bloco @{{name}} sem suporte foi ignorado.",
        "htmlImport.warn.font.web_font_not_downloaded" => {
            "A fonte web @font-face '{{family}}' está indisponível."
        }
        "htmlImport.warn.layout.percentage_absolute_offset_inferred" => {
            "Os deslocamentos percentuais de um elemento posicionado de forma absoluta foram aproximados."
        }
        "htmlImport.warn.layout.percentage_relative_offset_inferred" => {
            "Os deslocamentos percentuais de position:relative foram aproximados."
        }
        "htmlImport.warn.layout.aspect_ratio_no_definite_axis" => {
            "O aspect-ratio CSS sem um eixo definido foi ignorado."
        }
        "htmlImport.warn.layout.aspect_ratio_indefinite_container" => {
            "O aspect-ratio CSS dentro de um bloco contêiner indefinido foi ignorado."
        }
        "htmlImport.warn.layout.position_sticky_ignored" => "O position:sticky CSS foi ignorado.",
        "htmlImport.warn.layout.grid_tracks_approximated" => {
            "As faixas de grid CSS sem suporte foram aproximadas."
        }
        "htmlImport.warn.layout.float_ignored" => "O float CSS foi ignorado.",
        "htmlImport.warn.layout.mix_blend_mode_no_node_equivalent" => {
            "O mix-blend-mode CSS no nível do nó foi aproximado."
        }
        "htmlImport.warn.layout.overflow_scroll_clipped" => {
            "O overflow: auto / scroll CSS foi aproximado."
        }
        "htmlImport.warn.layout.negative_margins_ignored" => {
            "As margens CSS negativas foram ignoradas."
        }
        "htmlImport.warn.layout.margins_on_visual_box_ignored" => {
            "As margens CSS em uma caixa visual foram ignoradas."
        }
        "htmlImport.warn.layout.inline_margin_wrapping_approximated" => "Um elemento inline com margens CSS foi convertido em caixa e pode não mais quebrar entre linhas.",
        "htmlImport.warn.layout.content_box_percentage_approximated" => {
            "O dimensionamento percentual content-box foi aproximado."
        }
        "htmlImport.warn.layout.grid_empty_cells_packed" => {
            "As células vazias do grid CSS deixadas por linhas iniciais explícitas foram aproximadas."
        }
        "htmlImport.warn.layout.grid_span_reflowed" => {
            "Um item de grid CSS cuja extensão não coube na linha inicial foi aproximado."
        }
        "htmlImport.warn.layout.grid_rows_node_limit" => {
            "Limite de nós atingido; os invólucros de linha do grid CSS foram omitidos."
        }
        "htmlImport.warn.layout.grid_track_widths_unresolved" => {
            "As larguras das faixas de grid CSS que usam auto-fit / auto-fill foram aproximadas."
        }
        "htmlImport.warn.layout.grid_template_areas_ignored" => {
            "O posicionamento por grid-template-areas CSS não foi importado."
        }
        "htmlImport.warn.layout.grid_row_placement_ignored" => {
            "O posicionamento por grid-row CSS não foi importado."
        }
        "htmlImport.warn.layout.grid_column_unsupported" => {
            "O grid-column CSS `{{value}}` foi aproximado."
        }
        "htmlImport.warn.layout.block_auto_margins_ignored" => {
            "As margens automáticas CSS no eixo de bloco não foram importadas."
        }
        "htmlImport.warn.layout.auto_margin_node_limit" => {
            "Limite de nós atingido; o alinhamento por margem automática CSS foi omitido."
        }
        "htmlImport.warn.layout.flow_offset_no_definite_size" => {
            "Um deslocamento CSS no fluxo em um elemento sem tamanho definido foi descartado."
        }
        "htmlImport.warn.layout.flow_offset_node_limit" => {
            "Limite de nós atingido; um deslocamento CSS no fluxo foi omitido."
        }
        "htmlImport.warn.layout.flow_offset_approximated" => {
            "Os deslocamentos CSS no fluxo (insets de position:relative, translação de transform) foram aproximados."
        }
        "htmlImport.warn.layout.flow_offset_no_wrapper" => {
            "Um deslocamento CSS no fluxo em uma caixa que não pode hospedar um invólucro de deslocamento foi descartado."
        }
        "htmlImport.warn.layout.flex_wrap_column_not_emulated" => {
            "O flex-wrap em um contêiner flex de coluna não foi importado."
        }
        "htmlImport.warn.layout.flex_wrap_reverse_plain" => {
            "O flex-wrap:wrap-reverse foi aproximado."
        }
        "htmlImport.warn.layout.flex_wrap_indefinite_width" => {
            "O flex-wrap em um contêiner sem largura definida foi ignorado."
        }
        "htmlImport.warn.layout.flex_align_content_ignored" => {
            "O align-content CSS em um contêiner flex com quebra não foi importado."
        }
        "htmlImport.warn.layout.flex_wrap_indeterminate_children" => {
            "O flex-wrap com tamanhos indeterminados dos filhos no eixo principal foi ignorado."
        }
        "htmlImport.warn.layout.flex_wrap_node_limit" => {
            "Limite de nós atingido; as linhas de flex-wrap foram omitidas."
        }
        "htmlImport.warn.transform.unsupported_syntax" => {
            "A sintaxe de transform CSS sem suporte foi ignorada."
        }
        "htmlImport.warn.transform.unsupported_function" => {
            "As funções de transform CSS sem suporte (3D, matrix3d) foram ignoradas."
        }
        "htmlImport.warn.transform.percentage_translation_dropped" => {
            "Uma translação percentual de transform CSS em um eixo indefinido foi descartada."
        }
        "htmlImport.warn.transform.non_finite_matrix" => {
            "Um transform CSS que produziu uma matriz não finita foi ignorado."
        }
        "htmlImport.warn.transform.skew_dropped" => "A inclinação de transform CSS foi descartada.",
        "htmlImport.warn.transform.degenerate_scale" => {
            "Um transform CSS com escala zero ou não finita foi aproximado."
        }
        "htmlImport.warn.transform.mirroring_absolute" => {
            "O espelhamento por transform CSS foi aproximado."
        }
        "htmlImport.warn.transform.origin_z_ignored" => {
            "O deslocamento Z de transform-origin CSS foi ignorado."
        }
        "htmlImport.warn.transform.scale_not_baked" => {
            "Uma escala de transform CSS que não pôde ser incorporada ao tamanho do nó foi descartada."
        }
        "htmlImport.warn.transform.scale_baked" => {
            "A escala de transform CSS incorporada ao tamanho do nó foi aproximada."
        }
        "htmlImport.warn.transform.scale_auto_size_ignored" => {
            "A escala de transform CSS em um elemento de tamanho automático foi ignorada."
        }
        "htmlImport.warn.visual.background_repeat_approximated" => {
            "O background-repeat CSS direcional ou espaçado foi aproximado."
        }
        "htmlImport.warn.visual.background_tile_size_ignored" => {
            "Um tamanho explícito de ladrilho de fundo CSS foi ignorado."
        }
        "htmlImport.warn.visual.background_size_auto_box" => {
            "O background-size CSS em um elemento de tamanho automático foi aproximado."
        }
        "htmlImport.warn.visual.background_size_needs_intrinsic_size" => {
            "O background-size CSS que precisa do tamanho intrínseco da imagem foi aproximado."
        }
        "htmlImport.warn.visual.background_position_unsupported" => {
            "Um background-position CSS sem suporte foi ignorado."
        }
        "htmlImport.warn.visual.background_image_url_empty" => {
            "Uma URL vazia de imagem de fundo CSS foi ignorada."
        }
        "htmlImport.warn.visual.conic_gradient_ignored" => {
            "Os gradientes cônicos CSS foram ignorados."
        }
        "htmlImport.warn.visual.background_image_layer_unsupported" => {
            "Uma camada de background-image CSS sem suporte foi ignorada."
        }
        "htmlImport.warn.visual.background_color_unresolved" => {
            "Uma cor de fundo CSS não resolvida foi ignorada."
        }
        "htmlImport.warn.visual.background_position_dropped" => {
            "O background-position CSS foi ignorado."
        }
        "htmlImport.warn.visual.border_colors_approximated" => {
            "As cores de borda CSS por lado foram aproximadas."
        }
        "htmlImport.warn.visual.border_styles_approximated" => {
            "Os estilos de borda CSS mistos por lado foram aproximados."
        }
        "htmlImport.warn.visual.border_style_complex" => {
            "Um estilo de borda CSS complexo foi aproximado."
        }
        "htmlImport.warn.visual.border_style_unsupported" => {
            "Um estilo de borda CSS sem suporte foi aproximado."
        }
        "htmlImport.warn.visual.border_radius_elliptical" => {
            "Os raios de borda CSS elípticos foram aproximados."
        }
        "htmlImport.warn.visual.border_radius_unsupported" => {
            "Um raio de borda CSS sem suporte foi ignorado."
        }
        "htmlImport.warn.visual.box_shadow_layer_unsupported" => {
            "Uma camada de box-shadow CSS sem suporte foi ignorada."
        }
        "htmlImport.warn.visual.gradient_interpolation_ignored" => {
            "O método de interpolação de cores do gradiente CSS foi ignorado."
        }
        "htmlImport.warn.visual.linear_gradient_direction_unsupported" => {
            "Uma direção de linear-gradient CSS sem suporte foi ignorada."
        }
        "htmlImport.warn.visual.gradient_color_hints_ignored" => {
            "As dicas de cor do gradiente CSS foram ignoradas."
        }
        "htmlImport.warn.visual.gradient_color_stop_unsupported" => {
            "Uma parada de cor de gradiente CSS sem suporte foi ignorada."
        }
        "htmlImport.warn.visual.gradient_too_few_stops" => {
            "Um gradiente CSS com menos de duas paradas utilizáveis foi ignorado."
        }
        "htmlImport.warn.visual.gradient_repeating_approximated" => {
            "Um gradiente CSS repetido foi aproximado."
        }
        "htmlImport.warn.visual.gradient_stops_clamped" => {
            "As paradas de gradiente CSS fora do intervalo foram aproximadas."
        }
        "htmlImport.warn.visual.blur_radius_unsupported" => {
            "Um raio de desfoque CSS sem suporte foi ignorado."
        }
        "htmlImport.warn.visual.filter_drop_shadow_unsupported" => {
            "Um drop-shadow() de filtro CSS sem suporte foi ignorado."
        }
        "htmlImport.warn.visual.filter_function_unsupported" => {
            "Uma função de filtro CSS sem suporte foi ignorada."
        }
        "htmlImport.warn.visual.backdrop_filter_unsupported" => {
            "Uma função backdrop-filter CSS sem suporte foi ignorada."
        }
        "htmlImport.warn.visual.background_blend_mode_unsupported" => {
            "Um background-blend-mode CSS sem suporte foi ignorado."
        }
        "htmlImport.warn.visual.mix_blend_mode_on_fills" => {
            "O mix-blend-mode CSS em preenchimentos individuais foi aproximado."
        }
        "htmlImport.warn.visual.mix_blend_mode_unsupported" => {
            "Um mix-blend-mode CSS sem suporte foi ignorado."
        }
        "htmlImport.warn.visual.property_not_representable" => {
            "A propriedade CSS {{property}} foi ignorada."
        }
        "htmlImport.warn.visual.gradient_background_size_ignored" => {
            "O background-size CSS em um gradiente foi ignorado."
        }
        "htmlImport.warn.visual.radial_gradient_position_unsupported" => {
            "Uma posição de radial-gradient CSS sem suporte foi ignorada."
        }
        "htmlImport.warn.visual.radial_gradient_elliptical" => {
            "Um radial-gradient CSS elíptico foi aproximado."
        }
        "htmlImport.warn.visual.radial_gradient_extent_approximated" => {
            "Uma palavra-chave de extensão de radial-gradient CSS foi aproximada."
        }
        "htmlImport.warn.visual.radial_gradient_size_unsupported" => {
            "Um tamanho de radial-gradient CSS sem suporte foi ignorado."
        }
        "htmlImport.warn.text.shadow_layer_unsupported" => {
            "Uma camada de text-shadow CSS sem suporte foi ignorada."
        }
        "htmlImport.warn.text.shadow_extra_layers_ignored" => {
            "As camadas de text-shadow CSS após a primeira foram ignoradas."
        }
        "htmlImport.warn.text.shadow_on_inline_ignored" => {
            "O text-shadow CSS em um elemento em linha foi ignorado."
        }
        "htmlImport.warn.list.style_image_ignored" => "O list-style-image CSS não foi importado.",
        "htmlImport.warn.list.marker_position_outside_approximated" => {
            "Um marcador suspenso `list-style-position: outside` foi aproximado."
        }
        "htmlImport.warn.list.style_type_unsupported" => {
            "O list-style-type CSS `{{value}}` sem suporte foi aproximado."
        }
        "htmlImport.warn.media.object_fit_scale_down" => {
            "O object-fit:scale-down CSS foi aproximado."
        }
        "htmlImport.warn.media.object_fit_none_ignored" => "O object-fit:none CSS foi ignorado.",
        "htmlImport.warn.media.object_position_ignored" => "O object-position CSS foi ignorado.",
        "htmlImport.warn.media.image_intrinsic_axis_unresolved" => {
            "A proporção intrínseca da imagem não pôde determinar o eixo ausente porque o tamanho definido é dinâmico ou o bloco que a contém não tem tamanho definido."
        }
        "htmlImport.warn.media.image_mix_blend_mode_unsupported" => {
            "Um mix-blend-mode CSS sem suporte em uma imagem foi ignorado."
        }
        "htmlImport.warn.media.inline_svg_placeholder" => {
            "Um elemento <svg> em linha foi importado como espaço reservado."
        }
        "htmlImport.warn.media.input_type_fallback" => {
            "Um tipo de <input> sem suporte foi aproximado."
        }
        "htmlImport.warn.media.element_placeholder" => {
            "O elemento <{{tag}}> foi importado como espaço reservado."
        }
        "htmlImport.warn.media.picture_undecodable_types" => {
            "Um <picture> apenas com tipos de origem não decodificáveis foi aproximado."
        }
        "htmlImport.warn.table.rowspan_ignored" => "O atributo HTML rowspan não foi importado.",
        "htmlImport.warn.table.row_groups_unflattened" => {
            "As larguras das colunas de uma tabela com grupos de linhas desachatados pelo CSS foram aproximadas."
        }
        "htmlImport.warn.table.indefinite_width_approximated" => {
            "As larguras das colunas de uma tabela CSS sem largura definida foram aproximadas."
        }
        "htmlImport.warn.resource.invalid_base_href" => {
            "O <base href> inválido {{href}} foi ignorado."
        }
        "htmlImport.warn.resource.base_href_outside_origin" => {
            "O <base href> {{href}} fora da origem do projeto foi ignorado."
        }
        "htmlImport.warn.resource.external_stylesheet_skipped" => {
            "A folha de estilos externa {{url}} está indisponível."
        }
        "htmlImport.warn.resource.image_outside_origin" => {
            "A imagem {{url}} fora da origem do projeto foi importada como espaço reservado."
        }
        "htmlImport.warn.resource.image_unavailable" => {
            "A imagem indisponível {{url}} foi importada como espaço reservado."
        }
        "htmlImport.warn.resource.css_import_invalid" => {
            "O @import CSS inválido {{prelude}} foi ignorado."
        }
        "htmlImport.warn.resource.css_import_unresolvable" => {
            "O @import CSS {{reference}} está indisponível."
        }
        "htmlImport.warn.resource.css_import_cycle" => {
            "O @import CSS cíclico {{url}} foi ignorado."
        }
        "htmlImport.warn.resource.css_import_depth_limit" => {
            "O @import CSS {{url}} além da profundidade {{max_depth}} foi ignorado."
        }
        "htmlImport.warn.resource.css_import_unavailable" => {
            "O @import CSS {{url}} está indisponível."
        }
        "htmlImport.warn.project.multiple_html_entries" => {
            "{{count}} entradas HTML foram encontradas; {{entry}} foi escolhida e as demais foram aproximadas."
        }
        "htmlImport.warn.snapshot.truncated" => "Parte da captura do navegador foi descartada.",
        "htmlImport.warn.snapshot.node_limit" => {
            "Limite de nós atingido; o restante do conteúdo da captura foi omitido."
        }
        "htmlImport.warn.snapshot.tainted_images" => {
            "{{count}} imagens contaminadas por CORS, mantidas como URLs remotas, estão indisponíveis."
        }
        "htmlImport.warn.snapshot.invalid_rect" => {
            "Um nó da captura com retângulo ausente ou inválido foi descartado."
        }
        "htmlImport.warn.snapshot.unknown_kind" => {
            "Um nó da captura de tipo desconhecido foi descartado."
        }
        "htmlImport.warn.snapshot.rejected" => {
            "A captura do navegador ({{reason}}) foi descartada."
        }
        "htmlImport.warn.snapshot.unsupported_transform" => {
            "Um transform de captura sem suporte foi ignorado."
        }
        "htmlImport.warn.css.media_empty_query" => "Uma consulta @media vazia foi ignorada.",
        "htmlImport.warn.css.media_unsupported_type" => {
            "O tipo de @media sem suporte '{{name}}' foi ignorado."
        }
        "htmlImport.warn.css.media_unsupported_condition" => {
            "A condição de @media sem suporte '{{input}}' foi ignorada."
        }
        "htmlImport.warn.css.media_invalid_orientation" => {
            "A orientação de @media inválida '{{value}}' foi ignorada."
        }
        "htmlImport.warn.css.media_unsupported_feature" => {
            "O recurso de @media sem suporte '{{name}}' foi ignorado."
        }
        "htmlImport.warn.css.media_unsupported_range" => {
            "O intervalo de @media sem suporte '({{input}})' foi ignorado."
        }
        "htmlImport.warn.css.media_invalid_range" => {
            "O intervalo de @media inválido '({{input}})' foi ignorado."
        }
        "htmlImport.warn.css.media_invalid_length" => {
            "O comprimento de @media inválido '{{value}}' foi ignorado."
        }
        "htmlImport.diagnostics.title" => "Importação de HTML concluída",
        "htmlImport.diagnostics.summary" => "Itens degradados: {{count}}",
        "htmlImport.diagnostics.dismiss" => "Dispensar",
        "htmlImport.diagnostics.expand" => "Mostrar detalhes",
        "htmlImport.diagnostics.collapse" => "Ocultar detalhes",
        "htmlImport.diagnostics.more" => "+{{count}} mais",
        "dialog.pptxTitle" => "Exportar para PowerPoint",
        "dialog.pptxSummary" => "{{count}} slides exportados para:",
        "dialog.pptxEmpty" => "Esta apresentação não tem slides visíveis para exportar.",
        "settings.agents.acpQuickAdd" => "Adição rápida",
        "settings.agents.acpPresetAdd" => "Adicionar",
        "settings.agents.acpNotInstalled" => "Não instalado",
        "assetCenter.title" => "Central de recursos",
        "assetCenter.tab.templates" => "Modelos",
        "assetCenter.tab.styles" => "Estilos",
        "assetCenter.style.empty" => "Nenhum estilo correspondente",
        "assetCenter.style.pinned" => "Fixado",
        "assetCenter.style.searchPlaceholder" => "Buscar estilos ou tags",
        "assetCenter.style.generateHint" => "Um novo documento criado a partir do seu tema, no estilo fixado.",
        "ai.pinnedStyle" => "Estilo: {{name}}",
        "assetCenter.style.import" => "Importar estilo",
        "assetCenter.style.mine" => "Meus estilos",
        "assetCenter.style.builtIn" => "Estilos integrados",
        "assetCenter.style.importTitle" => "Importar DESIGN.md",
        "assetCenter.style.importHint" => "Cole o DESIGN.md inteiro e confirme a importação.",
        "assetCenter.style.importSource" => "Você pode copiar um estilo de uma biblioteca DESIGN.md como styles.refero.design.",
        "assetCenter.style.importConfirm" => "Importar",
        "assetCenter.style.importCancel" => "Cancelar",
        "assetCenter.style.importPickFile" => "Escolher arquivo…",
        "assetCenter.style.importHintFile" => "Escolha um arquivo DESIGN.md ou cole o documento inteiro abaixo.",
        "assetCenter.style.importPlaceholder" => "Cole seu DESIGN.md aqui",
        "assetCenter.style.importEmpty" => "Esse arquivo está vazio ou é curto demais para ser um guia de estilo.",
        "assetCenter.style.importNotText" => "Esse arquivo não pode ser lido como texto Markdown.",
        "assetCenter.style.importTooLarge" => "Esse arquivo tem mais de 512 KB.",
        "slidesPanel.tabSlides" => "Slides",
        "slidesPanel.tabCards" => "Cartões",
        "slidesPanel.present" => "Apresentar",
        "slidesPanel.exportPdf" => "Exportar PDF",
        "slidesPanel.exportAllSlides" => "Exportar todos os slides",
        "slidesPanel.exportSelectedSlides" => "Exportar slides selecionados ({{count}})",
        "settings.tab.ai" => "IA",
        "settings.agents.heroTitle" => "Conecte seu provedor de IA",
        "settings.agents.heroSubtitle" => "O OpenPencil aciona seus agentes CLI locais e provedores de API — conecte um para começar a gerar designs.",
        "settings.agents.statusConnected" => "Conectado",
        "settings.agents.statusNotConnected" => "Não conectado",
        "settings.agents.statusChecking" => "Verificando…",
        "settings.mcp.heroTitle" => "Conecte o OpenPencil via MCP externamente",
        "settings.mcp.heroSubtitle" => "Aponte qualquer CLI ou editor compatível com MCP para este espaço de trabalho e conduza o canvas com as mesmas ferramentas do agente embutido.",
        "settings.mcp.terminalFootnote" => "* Ao iniciar, o MCP é configurado automaticamente para as ferramentas CLI selecionadas.",
        "settings.mcp.customConfigTitle" => "Configuração personalizada do servidor MCP",
        "settings.mcp.customConfigDesc" => "Cole isto em qualquer cliente que leia um bloco padrão de servidor MCP.",
        "settings.mcp.copyConfig" => "Copiar config do MCP",
        "settings.system.heroTitle" => "Preferências do sistema",
        "settings.system.heroSubtitle" => "Aparência, atualizações e comportamento do canvas nesta instalação.",
        "settings.system.appearance" => "Aparência",
        "settings.system.appearanceLight" => "Claro",
        "settings.system.appearanceDark" => "Escuro",
        "settings.system.pencilCursor" => "Cursor de lápis",
        "settings.images.heroTitle" => "Imagens para seus designs",
        "settings.images.heroSubtitle" => "Busque fotos no Openverse ou conecte um provedor para gerá-las sob demanda.",
        "settings.fonts.heroTitle" => "Fontes deste documento",
        "settings.fonts.heroSubtitle" => "Resolva as fontes que um documento pede e não existem nesta máquina, e gerencie as que você importou.",
        "settings.account.heroTitle" => "Sua conta",
        "settings.account.heroSubtitle" => "Entre para sincronizar seu espaço de trabalho e sua licença entre dispositivos.",
        "tooltip.topbar.file" => "Arquivo",
        "tooltip.topbar.import" => "Importar",
        "tooltip.topbar.language" => "Idioma",
        "tooltip.topbar.collaboration" => "Colaboração",
        "tooltip.topbar.preview" => "Visualização",
        "tooltip.topbar.exitPreview" => "Sair da visualização",
        "tooltip.topbar.account" => "Conta",
        "settings.agents.providerRollMore" => "e mais {{count}}",
        "ai.thinking.adaptive" => "Raciocínio: automático",
        "ai.thinking.disabled" => "Raciocínio: desativado",
        "ai.thinking.enabled" => "Raciocínio: ativado",
        "ai.designProgress.detail.repairsApplied" => "{{count}} reparo(s) automático(s) aplicado(s)",
        "ai.designProgress.detail.repairsMore" => "… e mais {{count}} (ver o registo)",
        "ai.styleCard.builtin" => "Estilo integrado",
        "ai.styleCard.imported" => "DESIGN.md importado",
        "ai.styleCard.documentDesignMd" => "design.md do documento",
        _ => return super::pt_collab::lookup(key),
    })
}
