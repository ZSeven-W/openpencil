import assert from 'node:assert/strict'
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { test } from 'node:test'

import {
  DEFAULT_BUNDLE_PATH,
  DEFAULT_OUTPUT_PATH,
  buildDesignSkill,
  createDshDesignSkill,
} from '../scripts/build-design-skill.mjs'
import {
  OPENPENCIL_DESIGN_GUIDANCE_SECTION,
  OPENPENCIL_DESIGN_SKILL_CONTENT,
  OPENPENCIL_DESIGN_SKILL_NAME,
  registerOpenPencilDesignGuidance,
  registerOpenPencilDesignSkill,
} from '../lib/design-skill.js'

test('generated skill is a thin, direct adapter over the authoritative native draft context', async () => {
  const onDisk = await readFile(DEFAULT_OUTPUT_PATH, 'utf8')
  assert.equal(OPENPENCIL_DESIGN_SKILL_CONTENT, onDisk)
  assert.equal(OPENPENCIL_DESIGN_SKILL_NAME, 'openpencil-design')

  for (const section of [
    '## Strict Default Transaction',
    '## Begin Contract Is Authoritative',
    '## Publication Gate',
  ]) assert.match(onDisk, new RegExp(section.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')))

  assert.match(onDisk, /openpencil_pipeline_begin.*first `openpencil_pipeline_batch`.*second `openpencil_pipeline_batch`.*openpencil_pipeline_finish/is)
  assert.match(onDisk, /Call `openpencil_pipeline_begin\(\{brief\}\)` exactly once and immediately.*opens the private live canvas/is)
  assert.match(onDisk, /Omit `path` unless.*explicitly named.*plugin chooses a concrete collision-resistant `.op` filename.*preserve an explicitly named path exactly/is)
  assert.match(onDisk, /path.*rootNodeId.*platform.*canvas.*buildContract.*locked.*Never reinterpret.*switch paths.*rebuild/is)
  assert.match(onDisk, /Advance without narration.*do not put reasoning.*inspection.*another tool call between successful pipeline calls/is)
  assert.match(onDisk, /thrown error.*canContinue:false.*ends the transaction.*Do not retry.*inspect.*context-read.*abort.*replacement draft/is)
  assert.match(onDisk, /```js\nconst draftId = "<exact begin\.draftId>";\nconst script = String\.raw`\.\.\.`;\nconst r = await tools\.openpencil_pipeline_batch\(\{ draftId, script \}\);\nreturn r;\n```/is)
  assert.match(onDisk, /fresh scope.*batch 2.*do not recreate Page, App Content, Header, or Hero.*exact nodeId returned by batch 1/is)
  assert.match(onDisk, /I.*K.*return opaque node-id strings.*not nodes.*bindings only as.*parents.*never assign.*binding\.x\/y.*any member/is)
  assert.match(onDisk, /Finish each product card.*media.*omit.*name.*price.*before the next card/is)
  assert.match(onDisk, /bundled ecommerce-modern-light direction.*white base.*warm-tinted sections.*1120px centered content.*56px Hero display.*orange limited to CTA\/active\/price/is)
  assert.match(onDisk, /Visible copy follows the user's language.*Chinese request means Chinese copy/is)
  assert.match(onDisk, /literal hex colors in nodes.*no aliases.*Desktop commerce Header.*role navbar.*Nav role nav-links.*44px role nav-link.*frame containing text.*never a text node.*Header actions role toolbar.*44x44 role icon-button frames.*Button\/CTA.*role button.*at least 44px.*CTA is role button 160x48.*#C2410C\/#FFF.*exactly three equal-width product cards from one coherent collection.*gap 24.*no unused tail.*gray armchair.*Artemide Tolomeo lamp.*potted plant.*within four words/is)
  assert.match(onDisk, /Desktop Hero.*optional warm wrapper.*full-width Hero.*copy 512.*gap 64.*image 448.*horizontal padding 160.*headline\/subtitle.*width:"fill_container"/is)
  assert.match(onDisk, /Generic commerce uses the direct leaf `I\(hero,\{type:"image".*imageSearchQuery:"gray loveseat isolated photo"\}\)`.*never wrap it.*image:\{\.\.\.\}.*mix shapes.*reuse that query in a product card/is)
  assert.match(onDisk, /4–6 layer `layout:"none"` ellipse\/path composition only when the user explicitly requests illustration\/no photos/is)
  assert.match(onDisk, /plain stacked rounded rectangles.*lone small icon/is)
  assert.match(onDisk, /Each Hero\/Product\/Art\/Media frame.*one primary visual.*imageSearchQuery.*substantial composed shapes.*never both.*host enriches committed commerce images before each live preview.*canonical post-final fallback/is)
  assert.doesNotMatch(onDisk, /two or three distinct product images|copy <=60%|right visual 30–45%|2–4 layers/i)
  assert.match(onDisk, /Desktop commerce Header.*role navbar.*Nav role nav-links.*44px role nav-link.*frame containing text.*Header actions role toolbar.*44x44 role icon-button frames/is)
  assert.match(onDisk, /ordinary generation never calls context, inspect, render, read-image, or abort tools/is)
  assert.match(onDisk, /Every generated text node explicitly uses `fontFamily: "Inter, system-ui, sans-serif"`/i)
  assert.doesNotMatch(onDisk, /fontFamily:\s*["'](?:Inter|system-ui)["']/i)
  assert.match(onDisk, /stage:"needs_correction".*canContinue:true.*complete, non-empty `repairTargets`.*omitted === 0/is)
  assert.match(onDisk, /every target has `operation:"U"`.*exact non-empty `nodeId`.*non-empty `patch`/is)
  assert.match(onDisk, /exactly one additional bounded script using only `U\(target\.nodeId, target\.patch\)`.*finish exactly once more/is)
  assert.match(onDisk, /Never guess a patch or node id.*never retry.*inspect.*context-read.*abort.*rebuild/is)
  assert.match(onDisk, /successful finish already owns deterministic finalization.*exact PNG generation.*atomic `createIfAbsent` publication.*live-editor presentation/is)
  assert.match(onDisk, /Do not add a visual self-review loop/i)
  assert.doesNotMatch(onDisk, /get_design_agent_prompt|firstBurst|completionBurst|\bDSL\b|\boperations\b/i)
  assert.doesNotMatch(onDisk, /task list|task shell|eight-item|designAgentPrompt/i)

  const bytes = Buffer.byteLength(onDisk, 'utf8')
  const estimatedTokens = Math.ceil([...onDisk].length / 4)
  assert.deepEqual(onDisk.match(/^\d+\./gm), ['1.', '2.', '3.'])
  assert.ok(bytes <= 9_200, `thin adapter grew to ${bytes} bytes`)
  assert.ok(estimatedTokens <= 2_300, `thin adapter grew to about ${estimatedTokens} tokens`)
})

test('readmes document the bounded first preview and compact native text defaults', async () => {
  const [english, chinese] = await Promise.all([
    readFile(new URL('../README.md', import.meta.url), 'utf8'),
    readFile(new URL('../README.zh.md', import.meta.url), 'utf8'),
  ])

  assert.match(english, /ordinary one-line request.*openpencil_pipeline_begin.*bounded first direct-JS batch.*second and final direct-JS batch.*openpencil_pipeline_finish/is)
  assert.match(english, /Omit `path` unless.*explicitly named.*plugin creates.*collision-resistant `.op` filename.*avoiding template syntax.*begin\/abort loop/is)
  assert.match(english, /Platform and viewport.*latest direct user request.*model-expanded `brief` cannot silently turn.*mobile canvas/is)
  assert.match(english, /native finalization first.*enriches the canonical image slots.*post-final user preview/is)
  assert.match(english, /successful begin or batch call.*immediately without narration.*planning.*comparison.*inspection.*unrelated tool call/is)
  assert.match(english, /live sidebar.*both successful generation batches.*exact PNG preview cards/is)
  assert.match(english, /ecommerce-modern-light.*white base.*warm-tinted section rhythm.*orange actions.*1120px centered content.*56px Hero display.*visible copy.*language.*user.*request/is)
  assert.match(english, /desktop commerce Hero.*full-width horizontal frame.*padding:\[64,160\].*copy 512px.*gap 64px.*product visual 448px.*headline.*subtitle.*fill_container.*Never combine a fixed 1120px Hero width with that padding/is)
  assert.match(english, /generic commerce.*directly under Hero.*I\(hero,\{type:"image".*imageSearchQuery:"gray loveseat isolated photo"\}\).*imageSearchQuery.*direct node field.*image:\{\.\.\.\}.*neither wrapped nor mixed with decorative shapes.*reused by a product card/is)
  assert.match(english, /explicit `layout:"none"` stack.*4–6 positioned layers.*ellipse\/path.*allowed only when.*explicitly requests illustration or no photos.*plain stacked rounded rectangles.*empty right field.*overflow.*rolled back before batch two/is)
  assert.match(english, /Desktop commerce Hero geometry.*before batch two.*visual child.*layered shape.*fixed Hero inner width and height.*rolled back.*first batch/is)
  assert.match(english, /Generation receipts.*committed node mappings and previews.*native diagnostics.*deferred to finish.*exact repair transaction.*speculative mid-generation loop/is)
  assert.match(english, /desktop commerce category rail.*1120px.*product rail.*exactly three equal.*fill_container.*24px gaps.*no unused right tail/is)
  assert.match(english, /Outside commerce.*exactly one `type:"image"` leaf.*Commerce uses exactly three distinct product images/is)
  assert.match(english, /exact runtime-validated set.*gray armchair isolated photo.*artemide tolomeo lamp photo.*potted plant isolated photo.*last card.*potted plant.*rather than a vase/is)
  assert.match(english, /Every query stays at four English words or fewer.*names one concrete product.*never requests a lifestyle.*collection.*broad category/is)
  assert.match(english, /Every Hero\/Product\/Art\/Media frame has exactly one primary visual.*never an image plus an icon placeholder/is)
  assert.match(english, /each successful commerce batch.*host.*best-effort eight-second image-enrichment.*committed document.*before rendering.*live preview.*Hero.*batch one.*product rail.*batch two.*rather than all imagery.*only after finish/is)
  assert.match(english, /mobile category item.*56×56 tile frame.*desktop commerce category rail.*1120px.*exactly three equal.*fill_container.*24px gaps.*no unused right tail/is)
  assert.match(english, /header includes an explicit \*\*Close\*\* button.*dirty drafts.*save\/confirmation guard/is)
  assert.match(english, /I.*K.*opaque node-id strings.*not mutable nodes.*never assign `binding\.x`.*`binding\.y`.*any other member/is)
  assert.match(english, /Category rails.*distinct matching icon per label.*<label> icon tile.*never.*art.*media.*image.*native finalization.*stock-photo slot.*Complete each repeated product card.*media.*name.*price.*release gate collapses.*empty media shell.*partially executed script.*never publish.*blank product section/is)
  assert.match(english, /rejects illustration\/drawing\/engraving\/painting\/catalog metadata.*isolated-subject contract/is)
  assert.match(english, /Every generated text node.*fontFamily: "Inter, system-ui, sans-serif".*fontSize: 16.*lineHeight: 1\.5.*Desktop.*bundled Inter.*web host.*does not bundle Inter.*generic fallback.*Never use bare.*Inter.*CJK.*lineHeight < 1\.3.*raised to.*1\.5/is)
  assert.match(english, /stage: "needs_correction".*canContinue: true.*complete non-empty `repairTargets`.*omitted: 0.*operation: "U".*nodeId.*patch/is)
  assert.match(english, /exactly one `U\(\.\.\.\)`-only batch.*finish exactly once more without narration.*anything except `published: true` is terminal/is)
  assert.match(english, /thrown error.*canContinue: false.*other non-published finish result.*terminal.*do not retry.*inspect.*abort.*replacement draft/is)
  assert.match(english, /published: true.*exact final PNG and live editor.*end the run immediately.*openpencil_render.*read_image.*openpencil_pipeline_inspect/is)
  assert.match(english, /```js\nconst draftId = "<exact begin\.draftId>";\nconst script = String\.raw`\.\.\.`;\nconst r = await tools\.openpencil_pipeline_batch\(\{ draftId, script \}\);\nreturn r;\n```/is)

  assert.match(chinese, /普通一句话需求.*openpencil_pipeline_begin.*有界第一段直接 JS.*第二段也是最后一段直接 JS.*openpencil_pipeline_finish/is)
  assert.match(chinese, /用户明确指定文件名.*否则省略 `path`.*插件生成.*防冲突的 `.op` 文件名.*模板语法.*begin\/abort 重试/is)
  assert.match(chinese, /平台和 viewport.*最近一条真实用户请求.*模型扩写的 `brief`.*不能.*擅自改成移动端/is)
  assert.match(chinese, /先执行原生定稿.*富化定稿后真实存在的图片槽.*最终用户预览与发布/is)
  assert.match(chinese, /begin 或 batch 成功后.*不加叙述.*规划.*比较.*检查.*无关工具调用.*立即执行下一步/is)
  assert.match(chinese, /live canvas 侧边栏.*两次成功 batch.*精确 PNG 预览卡片/is)
  assert.match(chinese, /ecommerce-modern-light.*白色基底.*暖色区段节奏.*橙色操作色.*1120px.*56px Hero.*文案.*用户需求语言/is)
  assert.match(chinese, /桌面电商 Hero.*padding:\[64,160\].*全宽水平 frame.*512px 文案.*64px 间距.*448px 商品视觉.*禁止.*Hero 固定为 1120px.*padding.*标题和副标题.*fill_container/is)
  assert.match(chinese, /通用电商.*直接挂在 Hero 下.*I\(hero,\{type:"image".*imageSearchQuery:"gray loveseat isolated photo"\}\).*imageSearchQuery.*节点的直接字段.*image:\{\.\.\.\}.*wrapper.*装饰形状.*商品卡复用/is)
  assert.match(chinese, /只有用户明确要求插画或不使用照片.*layout:"none".*4–6 层.*ellipse\/path.*堆叠圆角矩形.*空白右栏.*溢出.*第二批前.*原子回滚/is)
  assert.match(chinese, /桌面电商 Hero 的几何结构.*第二批前.*视觉子节点.*叠加形状.*固定内宽高.*第一批原子回滚/is)
  assert.match(chinese, /Generation receipt.*节点映射与预览.*原生诊断.*finish.*精确修复事务.*猜测循环/is)
  assert.match(chinese, /桌面电商分类栏.*1120px.*商品栏.*三张等宽.*fill_container.*24px.*右侧.*空白/is)
  assert.match(chinese, /非电商设计默认只使用一个.*imageSearchQuery.*type:"image".*电商固定使用三张不同商品图/is)
  assert.match(chinese, /每个 Hero\/Product\/Art\/Media frame 只能有一个主视觉.*禁止 image 与占位 icon 并存/is)
  assert.match(chinese, /每次桌面电商 batch 提交成功.*未解析 query.*宿主.*最长 8 秒.*图片富化.*live preview.*第一批.*Hero 商品图.*第二批.*商品栏.*finish/is)
  assert.match(chinese, /分类项.*56×56 tile frame.*桌面电商分类栏.*1120px.*商品栏.*三张等宽.*fill_container.*24px.*空白/is)
  assert.match(chinese, /标题栏.*明确的“关闭”按钮.*dirty 草稿.*保存\/确认保护/is)
  assert.match(chinese, /I.*K.*不透明 node-id 字符串.*不是可修改的节点对象.*禁止赋值 `binding\.x`.*`binding\.y`.*任何成员/is)
  assert.match(chinese, /分类横栏.*每个 label.*不同且语义匹配的 icon.*<label> icon tile.*禁止使用.*art.*media.*image.*finalization.*图片槽.*每张商品卡.*media.*名称和价格.*发布门.*折叠空 media 壳.*半截执行的脚本.*不会再发布.*空白商品区/is)
  assert.match(chinese, /运行时 4\/4 实测通过.*gray armchair isolated photo.*artemide tolomeo lamp photo.*potted plant isolated photo.*第三张卡.*盆栽.*花瓶/is)
  assert.match(chinese, /每个 query.*四个英文词.*具体商品.*禁止.*lifestyle.*collection.*category/is)
  assert.match(chinese, /照片查询.*拒绝 illustration.*drawing.*engraving.*painting.*catalog.*isolated 查询.*独立主体证据/is)
  assert.match(chinese, /每个生成文本节点.*fontFamily: "Inter, system-ui, sans-serif".*桌面.*Inter.*Web 宿主.*通用字体回退/is)
  assert.match(chinese, /stage: "needs_correction".*canContinue: true.*完整非空 `repairTargets`.*omitted: 0.*operation: "U".*nodeId.*patch/is)
  assert.match(chinese, /唯一一段纯 `U\(\.\.\.\)` batch.*只再调用一次 finish.*任何不是 `published: true`.*终止/is)
  assert.match(chinese, /抛错.*canContinue: false.*其他未发布 finish 结果.*终止.*禁止重试.*inspect.*abort.*另起草稿/is)
  assert.match(chinese, /published: true.*精确最终 PNG 与 live editor.*立即向用户收尾.*openpencil_render.*read_image.*openpencil_pipeline_inspect/is)

  const combined = `${english}\n${chinese}`
  assert.doesNotMatch(combined, /clean finish.*retry|干净的 finish.*重试|at most three `type:"image"`|最多使用三个/is)
  assert.doesNotMatch(combined, /\bNOVA\b|\bDSL\b|repair uses operations|operations repair|operations 事务/i)
})

test('build is deterministic and fails closed when the upstream contract drifts', async () => {
  const bundle = JSON.parse(await readFile(DEFAULT_BUNDLE_PATH, 'utf8'))
  const upstream = bundle.files['skills/openpencil-design/SKILL.md']
  const first = createDshDesignSkill(upstream)
  const second = createDshDesignSkill(upstream)
  assert.equal(first, second)
  assert.equal(first, await readFile(DEFAULT_OUTPUT_PATH, 'utf8'))

  const root = await mkdtemp(join(tmpdir(), 'dsh-openpencil-design-skill-'))
  try {
    const rebuiltPath = join(root, 'rebuilt', 'SKILL.md')
    const rebuilt = await buildDesignSkill({ outputPath: rebuiltPath })
    assert.equal(rebuilt.outputPath, rebuiltPath)
    assert.equal(await readFile(rebuiltPath, 'utf8'), first)

    const brokenPath = join(root, 'skill-bundle.json')
    const broken = structuredClone(bundle)
    broken.files['skills/openpencil-design/SKILL.md'] = upstream.replace('## Layout Rules', '## Layout Contract')
    await writeFile(brokenPath, JSON.stringify(broken), 'utf8')
    await assert.rejects(
      buildDesignSkill({ bundlePath: brokenPath, outputPath: join(root, 'SKILL.md') }),
      /Layout Rules count is 0|missing sentinel.*layout containers/i,
    )
  } finally {
    await rm(root, { recursive: true, force: true })
  }
})

function serviceHarness() {
  const injections = []
  const registeredSkills = []
  const promptSections = []
  let skillRemovalCount = 0
  let promptRemovalCount = 0
  let fiberDisposeCount = 0
  const ctx = {
    inject(names, install) {
      injections.push([...names])
      const effectDisposers = []
      const child = {
        ...(names.includes('skills') ? {
          skills: {
            register(skill) {
              registeredSkills.push(skill)
              return () => { skillRemovalCount += 1 }
            },
          },
        } : {}),
        ...(names.includes('systemPrompt') ? {
          systemPrompt: {
            section(section) {
              promptSections.push(section)
              return () => { promptRemovalCount += 1 }
            },
          },
        } : {}),
        effect(mount) {
          const dispose = mount()
          if (typeof dispose === 'function') effectDisposers.push(dispose)
          return dispose
        },
      }
      install(child)
      return {
        dispose() {
          fiberDisposeCount += 1
          for (const dispose of effectDisposers.reverse()) dispose()
        },
      }
    },
  }
  return {
    ctx,
    injections,
    registeredSkills,
    promptSections,
    counts: () => ({ skillRemovalCount, promptRemovalCount, fiberDisposeCount }),
  }
}

test('optional skill and system-prompt services register and tear down with their fibers', () => {
  const harness = serviceHarness()
  const disposeSkill = registerOpenPencilDesignSkill(harness.ctx)
  const disposeGuidance = registerOpenPencilDesignGuidance(harness.ctx)

  assert.deepEqual(harness.injections, [['skills'], ['systemPrompt']])
  assert.equal(harness.registeredSkills.length, 1)
  assert.deepEqual(
    {
      name: harness.registeredSkills[0].name,
      source: harness.registeredSkills[0].source,
      content: harness.registeredSkills[0].content,
    },
    {
      name: 'openpencil-design',
      source: 'bundled',
      content: OPENPENCIL_DESIGN_SKILL_CONTENT,
    },
  )
  assert.match(harness.registeredSkills[0].description, /Optional troubleshooting\/reference.*Ordinary creation.*pipeline_begin.*must not load/is)
  assert.match(harness.registeredSkills[0].whenToUse, /Only when.*explicitly asks.*openpencil-design.*troubleshooting.*never.*ordinary design request/is)

  assert.equal(harness.promptSections.length, 1)
  assert.equal(harness.promptSections[0], OPENPENCIL_DESIGN_GUIDANCE_SECTION)
  assert.equal(harness.promptSections[0].name, 'openpencil:design-skill-guidance')
  assert.equal(harness.promptSections[0].order, 130)
  const guidance = harness.promptSections[0].text()
  assert.match(guidance, /one strict transaction.*openpencil_pipeline_begin.*batch 1.*batch 2.*openpencil_pipeline_finish/is)
  assert.match(guidance, /Call begin exactly once and immediately.*never load `openpencil-design` first/is)
  assert.match(guidance, /Omit begin\.path unless.*explicitly named.*plugin chooses.*concrete collision-resistant `.op` filename.*Preserve an explicitly named path exactly/is)
  assert.match(guidance, /Pass the user's direct request as begin\.brief.*without adding a platform.*viewport.*device.*did not name/is)
  assert.match(guidance, /path.*root.*platform.*canvas.*never reinterpret.*switch.*rebuild/is)
  assert.match(guidance, /without narration.*planning.*comparison.*inspection.*unrelated tool call/is)
  assert.match(guidance, /thrown error.*canContinue:false.*terminal.*do not retry.*context-read.*abort.*replacement draft/is)
  assert.match(guidance, /run_code use exactly:\nconst draftId = "<exact begin\.draftId>";\nconst script = String\.raw`\.\.\.`;\nconst r = await tools\.openpencil_pipeline_batch\(\{ draftId, script \}\);\nreturn r;/is)
  assert.match(guidance, /Quote the exact begin\.draftId into the standalone `draftId` string first/is)
  assert.match(guidance, /fixed call object contains only `draftId` and `script`.*return only `r`/is)
  assert.match(guidance, /category rail.*distinct matching icon tile.*数码\/electronics.*smartphone.*camera.*食品\/food.*utensils.*sandwich.*croissant.*never use `lamp` or `coffee`/is)
  assert.match(guidance, /Default to one image outside commerce.*ecommerce-modern-light.*clean white base.*warm-tinted section rhythm.*1120px centered content.*56px Hero display.*horizontal two-column split row.*optional warm section wrapper.*Hero is full-width.*padding:\[64,160\].*copy width 512.*gap 64.*image width 448.*headline and subtitle.*width:"fill_container"/is)
  assert.match(guidance, /Visible copy follows the user request language consistently.*Chinese request means Chinese copy/is)
  assert.match(guidance, /literal hex colors.*not palette alias variables.*Header role navbar.*height 64.*padding:\[0,160\].*Nav role nav-links.*44px role nav-link frames.*text child.*never put nav-link on text.*Header actions role toolbar.*44x44 role icon-button frames.*CTA role button.*160x48.*#C2410C\/#FFFFFF.*label inside the CTA binding.*Generic commerce.*directly under Hero.*I\(hero,\{type:"image".*imageSearchQuery:"gray loveseat isolated photo"\}\).*`imageSearchQuery` is a direct field.*never `image:\{\.\.\.\}`.*never a wrapper.*never mixed with shapes.*never reused by a product card/is)
  assert.match(guidance, /4-6 layer layout:none ellipse\/path composition only when the user explicitly requests illustration\/no photos/is)
  assert.match(guidance, /Desktop commerce uses exactly three equal-width fill_container product cards from one coherent collection.*24px gaps.*no unused right tail.*mobile product rail.*at most 2 fill_container cards or equal numeric-width cards.*clipped scroller.*gray armchair isolated photo.*artemide tolomeo lamp photo.*potted plant isolated photo.*third card is a potted plant.*imageSearchQuery.*no more than four English words.*exactly one product.*never uses a broad lifestyle, collection, or category query.*lone small icon.*one primary visual.*never an image plus an icon placeholder/is)
  assert.match(guidance, /host enriches committed commerce images before each live preview.*finish retains one canonical post-final fallback/is)
  assert.match(guidance, /published:true.*terminal success/is)
  assert.match(guidance, /canContinue:true.*continues by doing exactly what its `next` says.*needs_preview.*needs_refinalization.*call finish exactly once more.*stage:"needs_correction".*complete non-empty `repairTargets`.*omitted.*`0`/is)
  assert.match(guidance, /every item contains `operation:"U"`.*nodeId.*patch.*exactly one U-only batch.*finish exactly once more/is)
  assert.match(guidance, /host bounds repair rounds at two.*another complete repairTargets array.*stop on anything except `published:true`.*never retry.*inspect.*read an image\/context.*abort.*rebuild/is)
  assert.doesNotMatch(guidance, /firstBurst|completionBurst|\bDSL\b/i)

  disposeGuidance()
  disposeSkill()
  assert.deepEqual(harness.counts(), {
    skillRemovalCount: 1,
    promptRemovalCount: 1,
    fiberDisposeCount: 2,
  })
})
