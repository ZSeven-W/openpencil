---
name: examples
description: Reference examples for common UI components
phase: [generation]
trigger:
  keywords: [example, sample, show me, how to, 示例, 样例, 怎么]
priority: 50
budget: 1000
category: knowledge
---

EXAMPLES:

Button:
{ "id":"btn-1","type":"frame","role":"button","width":180,"cornerRadius":8,"fill":[{"type":"solid","color":"#3B82F6"}],"children":[{"id":"btn-icon","type":"path","name":"ArrowRightIcon","role":"icon","d":"M5 12h14m-7-7 7 7-7 7","width":20,"height":20,"stroke":{"thickness":2,"fill":[{"type":"solid","color":"#FFF"}]}},{"id":"btn-text","type":"text","role":"label","content":"Continue","fontSize":16,"fontWeight":600,"fill":[{"type":"solid","color":"#FFF"}]}] }

Card:
{ "id":"card-1","type":"frame","role":"card","width":320,"height":340,"fill":[{"type":"solid","color":"#FFF"}],"effects":[{"type":"shadow","offsetX":0,"offsetY":4,"blur":12,"spread":0,"color":"rgba(0,0,0,0.1)"}],"children":[{"id":"card-img","type":"image","width":"fill_container","height":180},{"id":"card-body","type":"frame","width":"fill_container","height":"fit_content","layout":"vertical","padding":20,"gap":8,"children":[{"id":"card-title","type":"text","role":"heading","content":"Title","fontSize":20,"fontWeight":700,"fill":[{"type":"solid","color":"#111827"}]},{"id":"card-desc","type":"text","role":"body-text","content":"Description","fontSize":14,"fill":[{"type":"solid","color":"#6B7280"}]}]}] }
