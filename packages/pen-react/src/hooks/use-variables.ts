import { useCallback, useMemo } from 'react';
import type { VariableDefinition } from '@zseven-w/pen-types';
import { useDesignEngine } from './use-design-engine.js';
import { useDocument } from './use-document.js';

interface VariablesState {
  variables: Record<string, VariableDefinition>;
  setVariable: (name: string, def: VariableDefinition) => void;
  removeVariable: (name: string) => void;
  renameVariable: (oldName: string, newName: string) => void;
  resolveVariable: (ref: string) => unknown;
}

/**
 * Returns variable definitions and mutation actions.
 * Re-renders when the document changes (variables are part of PenDocument).
 */
export function useVariables(): VariablesState {
  const engine = useDesignEngine();
  const doc = useDocument();
  const variables = useMemo(() => doc.variables ?? {}, [doc]);

  const setVariable = useCallback(
    (name: string, def: VariableDefinition) => engine.setVariable(name, def),
    [engine],
  );
  const removeVariable = useCallback((name: string) => engine.removeVariable(name), [engine]);
  const renameVariable = useCallback(
    (oldName: string, newName: string) => engine.renameVariable(oldName, newName),
    [engine],
  );
  const resolveVariable = useCallback((ref: string) => engine.resolveVariable(ref), [engine]);

  return { variables, setVariable, removeVariable, renameVariable, resolveVariable };
}
