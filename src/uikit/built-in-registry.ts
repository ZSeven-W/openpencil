import type { UIKit } from '@/types/uikit'
import { DEFAULT_KIT_DOCUMENT } from './kits/default-kit'
import { DEFAULT_KIT_META } from './kits/default-kit-meta'
import { extractComponentsFromDocument } from './kit-utils'

const defaultKit: UIKit = {
  id: 'default-uikit',
  name: 'Default UIKit',
  description: 'Built-in UI components: buttons, inputs, cards, navigation, feedback, and layout primitives.',
  version: '1.0.0',
  builtIn: true,
  document: DEFAULT_KIT_DOCUMENT,
  components: extractComponentsFromDocument(DEFAULT_KIT_DOCUMENT, DEFAULT_KIT_META),
}

export function getBuiltInKits(): UIKit[] {
  return [defaultKit]
}
