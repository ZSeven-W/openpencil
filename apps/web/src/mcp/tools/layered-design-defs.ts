/**
 * Tool definitions for the layered design workflow:
 * design_skeleton → design_content → design_refine
 */
export const LAYERED_DESIGN_TOOLS = [
  {
    name: 'design_skeleton',
    description:
      'Create a layout skeleton with root frame + section frames. Part of the layered design workflow. ' +
      'Returns section IDs with per-section content guidelines and suggested roles. ' +
      'After creating skeleton, call design_content for each section, then design_refine.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: { type: 'string', description: 'Path to .op file, or omit for live canvas' },
        rootFrame: {
          type: 'object',
          description:
            'Root frame definition. Props: name, width (1200 desktop/375 mobile), height (0 for auto-expand, 812 for mobile), layout, gap, fill, padding.',
          properties: {
            name: { type: 'string' },
            width: { type: 'number' },
            height: { type: 'number' },
            layout: { type: 'string', enum: ['vertical', 'horizontal'] },
            gap: { type: 'number' },
            fill: { type: 'array', items: { type: 'object' } },
            padding: { type: 'object' },
          },
          required: ['width', 'height'],
        },
        sections: {
          type: 'array',
          description:
            'Section frame definitions. Each section becomes a direct child of root. ' +
            'Props: name (required), height, layout, padding, gap, fill, role, justifyContent, alignItems.',
          items: {
            type: 'object',
            properties: {
              name: { type: 'string' },
              height: { type: 'number' },
              layout: { type: 'string', enum: ['vertical', 'horizontal'] },
              padding: { type: 'object' },
              gap: { type: 'number' },
              fill: { type: 'array', items: { type: 'object' } },
              role: { type: 'string' },
              justifyContent: { type: 'string' },
              alignItems: { type: 'string' },
            },
            required: ['name'],
          },
        },
        styleGuide: {
          type: 'object',
          description:
            'Style guide for per-section content guidelines. Props: palette (Record<string,string>), fonts ({heading,body}), aesthetic (string).',
        },
        canvasWidth: { type: 'number', description: 'Canvas width (default: rootFrame.width)' },
        pageId: { type: 'string', description: 'Target page ID (defaults to first page)' },
      },
      required: ['rootFrame', 'sections'],
    },
  },
  {
    name: 'design_content',
    description:
      'Populate a section with content nodes. Part of the layered design workflow. ' +
      'Inserts children into an existing section frame created by design_skeleton. ' +
      'Runs per-section post-processing (role defaults, icon resolution, sanitization). ' +
      'Returns inserted count, warnings, and a depth-limited snapshot of the section.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: { type: 'string', description: 'Path to .op file, or omit for live canvas' },
        sectionId: {
          type: 'string',
          description: 'ID of the section frame (from design_skeleton result)',
        },
        children: {
          type: 'array',
          description:
            'Array of PenNode objects to insert as children of the section. ' +
            'Each node can have nested children. IDs are auto-generated if not provided. ' +
            'Node types: frame, text, path, rectangle, ellipse, image, group. ' +
            'Use roles for smart defaults (button, card, heading, etc.).',
          items: { type: 'object' },
        },
        postProcess: {
          type: 'boolean',
          description: 'Apply post-processing (default true). Set false to skip.',
        },
        canvasWidth: { type: 'number', description: 'Canvas width for layout (default 1200)' },
        pageId: { type: 'string', description: 'Target page ID (defaults to first page)' },
      },
      required: ['sectionId', 'children'],
    },
  },
  {
    name: 'design_refine',
    description:
      'Run full-tree validation and auto-fixes on a design. Part of the layered design workflow. ' +
      'Call after all sections are populated via design_content. ' +
      'Applies: role resolution, card row equalization, overflow fixes, text height estimation, ' +
      'icon resolution, layout sanitization, clipContent enforcement. ' +
      'Returns a list of fixes applied and a layout snapshot.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: { type: 'string', description: 'Path to .op file, or omit for live canvas' },
        rootId: {
          type: 'string',
          description: 'ID of the root frame (from design_skeleton result)',
        },
        canvasWidth: { type: 'number', description: 'Canvas width for layout (default 1200)' },
        pageId: { type: 'string', description: 'Target page ID (defaults to first page)' },
      },
      required: ['rootId'],
    },
  },
]
