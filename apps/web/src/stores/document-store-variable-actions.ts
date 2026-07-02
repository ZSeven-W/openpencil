import type { PenDocument } from '@/types/pen';
import type { VariableDefinition } from '@/types/variables';

import { useHistoryStore } from '@/stores/history-store';
import { getDefaultTheme } from '@/variables/resolve-variables';
import { replaceVariableRefsInTree } from '@/variables/replace-refs';

interface VariableActions {
  setVariable: (name: string, definition: VariableDefinition) => void;
  removeVariable: (name: string) => void;
  renameVariable: (oldName: string, newName: string) => void;
  setThemes: (themes: Record<string, string[]>) => void;
}

type SetState = {
  (partial: Partial<{ document: PenDocument; isDirty: boolean }>): void;
  (
    fn: (state: { document: PenDocument }) => Partial<{ document: PenDocument; isDirty: boolean }>,
  ): void;
};

export function createVariableActions(
  set: SetState,
  get: () => { document: PenDocument },
): VariableActions {
  return {
    setVariable: (name, definition) => {
      useHistoryStore.getState().pushState(get().document);
      set((s) => ({
        document: {
          ...s.document,
          variables: { ...s.document.variables, [name]: definition },
        },
        isDirty: true,
      }));
    },

    removeVariable: (name) => {
      const state = get();
      const vars = state.document.variables;
      if (!vars || !(name in vars)) return;
      useHistoryStore.getState().pushState(state.document);
      const { [name]: _removed, ...rest } = vars;
      const activeTheme = getDefaultTheme(state.document.themes);
      // Replace variable refs across all pages
      const doc = state.document;
      if (doc.pages && doc.pages.length > 0) {
        const newPages = doc.pages.map((p) => ({
          ...p,
          children: replaceVariableRefsInTree(p.children, name, null, vars, activeTheme),
        }));
        set({
          document: {
            ...doc,
            variables: Object.keys(rest).length > 0 ? rest : undefined,
            pages: newPages,
          },
          isDirty: true,
        });
      } else {
        const newChildren = replaceVariableRefsInTree(doc.children, name, null, vars, activeTheme);
        set({
          document: {
            ...doc,
            variables: Object.keys(rest).length > 0 ? rest : undefined,
            children: newChildren,
          },
          isDirty: true,
        });
      }
    },

    renameVariable: (oldName, newName) => {
      if (oldName === newName) return;
      const state = get();
      const vars = state.document.variables;
      if (!vars || !(oldName in vars)) return;
      useHistoryStore.getState().pushState(state.document);
      const def = vars[oldName];
      const { [oldName]: _removed, ...rest } = vars;
      const newVars = { ...rest, [newName]: def };
      const activeTheme = getDefaultTheme(state.document.themes);
      // Rename variable refs across all pages
      const doc = state.document;
      if (doc.pages && doc.pages.length > 0) {
        const newPages = doc.pages.map((p) => ({
          ...p,
          children: replaceVariableRefsInTree(p.children, oldName, newName, vars, activeTheme),
        }));
        set({
          document: { ...doc, variables: newVars, pages: newPages },
          isDirty: true,
        });
      } else {
        const newChildren = replaceVariableRefsInTree(
          doc.children,
          oldName,
          newName,
          vars,
          activeTheme,
        );
        set({
          document: { ...doc, variables: newVars, children: newChildren },
          isDirty: true,
        });
      }
    },

    setThemes: (themes) => {
      useHistoryStore.getState().pushState(get().document);
      set((s) => ({
        document: { ...s.document, themes },
        isDirty: true,
      }));
    },
  };
}
