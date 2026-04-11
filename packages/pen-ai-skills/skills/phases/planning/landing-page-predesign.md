---
name: landing-page-predesign
description: Mandatory pre-design steps for landing pages — concept extraction, superfan simulation, transformation mapping
phase: [planning]
trigger:
  keywords: [landing, website, 官网, 营销, marketing, promotional, homepage]
priority: 8
budget: 1500
category: domain
---

LANDING PAGE PRE-DESIGN (perform these steps INTERNALLY, then include results as JSON fields in your output):

## Step 1: Concept Extraction

Identify the core concepts this page must communicate:

- Domain concepts: what space/category the product is in, what it does
- Qualitative concepts: what the experience should FEEL like

Mark each concept as primary or secondary.
Map each to a concrete design decision:

- Content: what does this concept say?
- Layout: how is it structured?
- Color: what palette supports it?
- Typography: what type treatment fits?

## Step 2: Superfan Simulation

Simulate a brief research interview with a product superfan. Extract 2-5 insights:

- What do they love about the product?
- What feels magical to them?
- What stories do they tell others about it?
- What visuals feel authentic to them?

Apply these insights to:

- Hero messaging (what headline resonates?)
- Content hierarchy (what comes first?)
- Section priorities (what matters most?)
- Visual direction (what feels right?)

## Step 3: Transformation Mapping

Define the emotional arc of the page:

- Before State: the pain, frustration, or limitation the visitor feels NOW
- After State: what life looks like AFTER using the product (emotionally, not just functionally)
- Bridge: how the product takes them from Before → After
- Feeling: ONE dominant emotion the page should evoke
  (confidence / liberation / belonging / power / calm / mastery)

Every section should subtly answer: "Here's where we're taking you."

## Output

Include the results as a "preDesignContext" field INSIDE your JSON plan object (alongside rootFrame, styleGuideName, subtasks):
"preDesignContext":{"primaryConcepts":["..."],"superfanInsights":["..."],"transformation":{"before":"...","after":"...","bridge":"...","feeling":"..."}}

Do NOT output these steps as prose. Do NOT explain your reasoning. Include them only as the JSON field above.

This context will guide the subtask decomposition and style guide selection.
