import type { SkillTrigger, SkillRegistryEntry } from './types'

export function matchTrigger(
  trigger: SkillTrigger,
  userMessage: string,
  flags: Record<string, boolean>
): boolean {
  if (trigger === null) return true

  if ('keywords' in trigger) {
    const msg = userMessage.toLowerCase()
    return trigger.keywords.some(kw => msg.includes(kw.toLowerCase()))
  }

  if ('flags' in trigger) {
    return trigger.flags.every(flag => flags[flag] === true)
  }

  return false
}

export function filterByIntent(
  skills: SkillRegistryEntry[],
  userMessage: string,
  flags: Record<string, boolean>
): SkillRegistryEntry[] {
  return skills
    .filter(skill => matchTrigger(skill.meta.trigger, userMessage, flags))
    .sort((a, b) => a.meta.priority - b.meta.priority)
}

export function injectDynamicContent(
  content: string,
  dynamicContent?: Record<string, string>
): string {
  if (!dynamicContent) return content
  return content.replace(/\{\{(\w+)\}\}/g, (_match, key) => {
    if (key in dynamicContent) return dynamicContent[key]
    console.warn(`[pen-ai-skills] Missing dynamic content for placeholder: {{${key}}}`)
    return ''
  })
}
