// Shared JSON-Schema property fragments used by every element-tool
// definition in `element-tool-defs-{base,ext,ext-2}.ts`. Extracted
// here so each definition file can stay under the 800-line repo
// ceiling without duplicating the prop objects across shards.

export const schemaVersionProp = {
  type: 'string' as const,
  enum: ['1.0'],
  description:
    'Schema version this tool was authored against (v0-MUST §4.2). Clients MAY omit. Breaking schema changes ship as a new tool with _v1 suffix; old tools are kept one stage before being removed from ListTools.',
};

export const filePathProp = {
  type: 'string' as const,
  description: 'Path to .op file, or omit for live canvas',
};

export const parentIdProp = {
  type: 'string' as const,
  description: 'Target parent node id (must exist in the document). Omit for root-level insertion.',
};

export const pageIdProp = {
  type: 'string' as const,
  description: 'Target page ID (defaults to first page)',
};
