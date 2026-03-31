import { styleGuideRegistry } from '@zseven-w/pen-ai-skills/_generated/style-guide-registry';
import { selectStyleGuide, STYLE_GUIDE_TAGS } from '@zseven-w/pen-ai-skills/style-guide';

export const STYLE_GUIDE_TOOL_DEFINITIONS = [
  {
    name: 'get_style_guide_tags',
    description: 'Returns all available style guide tags for filtering.',
    inputSchema: {
      type: 'object' as const,
      properties: {},
      required: [],
    },
  },
  {
    name: 'get_style_guide',
    description:
      'Returns a style guide for design inspiration based on tags or name. ' +
      'Provide 5-10 tags for best matching, or a specific name for direct lookup.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        tags: {
          type: 'array',
          items: { type: 'string' },
          description: 'Style guide tags for matching (5-10 recommended)',
        },
        name: {
          type: 'string',
          description: 'Direct lookup by style guide name',
        },
        platform: {
          type: 'string',
          enum: ['webapp', 'mobile', 'landing-page', 'slides'],
          description: 'Filter by target platform',
        },
      },
      required: [],
    },
  },
];

export const STYLE_GUIDE_TOOL_NAMES = new Set(['get_style_guide_tags', 'get_style_guide']);

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export async function handleStyleGuideToolCall(
  name: string,
  args: Record<string, unknown>,
): Promise<string> {
  const a = args as any; // eslint-disable-line @typescript-eslint/no-explicit-any
  switch (name) {
    case 'get_style_guide_tags':
      return JSON.stringify(
        {
          tags: [...STYLE_GUIDE_TAGS],
          count: STYLE_GUIDE_TAGS.length,
        },
        null,
        2,
      );
    case 'get_style_guide': {
      const guide = selectStyleGuide(styleGuideRegistry, {
        tags: a.tags,
        name: a.name,
        platform: a.platform,
      });
      if (!guide) {
        return JSON.stringify(
          { error: 'No matching style guide found. Try different tags or list available names.' },
          null,
          2,
        );
      }
      return JSON.stringify(
        {
          name: guide.name,
          tags: guide.tags,
          platform: guide.platform,
          content: guide.content,
        },
        null,
        2,
      );
    }
    default:
      return '';
  }
}
