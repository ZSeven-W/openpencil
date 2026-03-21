import type { UIKit } from '@/types/uikit'
import { SHADCN_KIT_DOCUMENT } from './kits/shadcn-kit'
import { SHADCN_KIT_META } from './kits/shadcn-kit-meta'
import { extractComponentsFromDocument } from './kit-utils'

const shadcnKit: UIKit = {
  id: 'shadcn-ui',
  name: 'shadcn UI',
  description: 'shadcn/ui-styled components: buttons, inputs, cards, navigation, feedback, and layout primitives.',
  version: '1.0.0',
  builtIn: true,
  document: SHADCN_KIT_DOCUMENT,
  components: extractComponentsFromDocument(SHADCN_KIT_DOCUMENT, SHADCN_KIT_META),
}

export function getBuiltInKits(): UIKit[] {
  return [shadcnKit]
}
