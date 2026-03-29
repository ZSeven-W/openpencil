import type { PenDocument } from '@zseven-w/pen-types';
import type { VariableDefinition } from '@zseven-w/pen-types';
import { getDefaultTheme, resolveVariableRef, replaceVariableRefsInTree } from '@zseven-w/pen-core';

export interface VariableManagerOptions {
  getDocument: () => PenDocument;
  setDocument: (doc: PenDocument) => void;
}

/**
 * Manages design variables with theme support.
 * Extracted from apps/web/src/stores/document-store-variable-actions.ts.
 */
export class VariableManager {
  private getDocument: () => PenDocument;
  private setDocument: (doc: PenDocument) => void;

  constructor(options: VariableManagerOptions) {
    this.getDocument = options.getDocument;
    this.setDocument = options.setDocument;
  }

  setVariable(name: string, definition: VariableDefinition): void {
    const doc = this.getDocument();
    this.setDocument({
      ...doc,
      variables: { ...doc.variables, [name]: definition },
    });
  }

  removeVariable(name: string): void {
    const doc = this.getDocument();
    const vars = doc.variables;
    if (!vars || !(name in vars)) return;
    const { [name]: _removed, ...rest } = vars;
    const activeTheme = getDefaultTheme(doc.themes);

    if (doc.pages && doc.pages.length > 0) {
      const newPages = doc.pages.map((p) => ({
        ...p,
        children: replaceVariableRefsInTree(p.children, name, null, vars, activeTheme),
      }));
      this.setDocument({
        ...doc,
        variables: Object.keys(rest).length > 0 ? rest : undefined,
        pages: newPages,
      });
    } else {
      const newChildren = replaceVariableRefsInTree(doc.children, name, null, vars, activeTheme);
      this.setDocument({
        ...doc,
        variables: Object.keys(rest).length > 0 ? rest : undefined,
        children: newChildren,
      });
    }
  }

  renameVariable(oldName: string, newName: string): void {
    if (oldName === newName) return;
    const doc = this.getDocument();
    const vars = doc.variables;
    if (!vars || !(oldName in vars)) return;
    const def = vars[oldName];
    const { [oldName]: _removed, ...rest } = vars;
    const newVars = { ...rest, [newName]: def };
    const activeTheme = getDefaultTheme(doc.themes);

    if (doc.pages && doc.pages.length > 0) {
      const newPages = doc.pages.map((p) => ({
        ...p,
        children: replaceVariableRefsInTree(p.children, oldName, newName, vars, activeTheme),
      }));
      this.setDocument({ ...doc, variables: newVars, pages: newPages });
    } else {
      const newChildren = replaceVariableRefsInTree(
        doc.children,
        oldName,
        newName,
        vars,
        activeTheme,
      );
      this.setDocument({ ...doc, variables: newVars, children: newChildren });
    }
  }

  resolveVariable(ref: string): unknown {
    const doc = this.getDocument();
    const vars = doc.variables ?? {};
    const activeTheme = getDefaultTheme(doc.themes);
    // Strip leading $ if present
    const name = ref.startsWith('$') ? ref.slice(1) : ref;
    const def = vars[name];
    if (!def) return undefined;
    return resolveVariableRef(`$${name}`, vars, activeTheme);
  }

  setThemes(themes: Record<string, string[]>): void {
    const doc = this.getDocument();
    this.setDocument({ ...doc, themes });
  }
}
