/**
 * Text editing DOM overlay support.
 * Manages the <textarea> overlay for inline text editing.
 */
export interface TextEditOverlayOptions {
  onEditStart?: (nodeId: string) => void;
  onEditEnd?: (nodeId: string, content: string) => void;
}

// Full implementation will manage a <textarea> positioned over the canvas
// at the node's screen coordinates, handling commit on blur/Enter.
