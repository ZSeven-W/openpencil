import { describe, it, expect, beforeEach } from 'vitest';
import { VariableManager } from '../core/variable-manager';
import type { PenDocument } from '@zseven-w/pen-types';
import type { VariableDefinition } from '@zseven-w/pen-types';
import { createEmptyDocument, migrateToPages, ensureDocumentNodeIds } from '@zseven-w/pen-core';

describe('VariableManager', () => {
  let vm: VariableManager;
  let doc: PenDocument;

  beforeEach(() => {
    doc = ensureDocumentNodeIds(migrateToPages(createEmptyDocument()));
    vm = new VariableManager({
      getDocument: () => doc,
      setDocument: (d) => {
        doc = d;
      },
    });
  });

  it('setVariable should add a variable to the document', () => {
    const def: VariableDefinition = { type: 'color', value: '#FF0000' };
    vm.setVariable('primary', def);
    expect(doc.variables?.['primary']).toEqual(def);
  });

  it('removeVariable should delete a variable', () => {
    vm.setVariable('primary', { type: 'color', value: '#FF0000' });
    vm.removeVariable('primary');
    expect(doc.variables?.['primary']).toBeUndefined();
  });

  it('renameVariable should change the key', () => {
    vm.setVariable('old-name', { type: 'color', value: '#00FF00' });
    vm.renameVariable('old-name', 'new-name');
    expect(doc.variables?.['old-name']).toBeUndefined();
    expect(doc.variables?.['new-name']).toBeDefined();
    expect(doc.variables!['new-name'].value).toBe('#00FF00');
  });

  it('renameVariable should be no-op for same name', () => {
    vm.setVariable('color', { type: 'color', value: '#FFF' });
    const before = { ...doc.variables };
    vm.renameVariable('color', 'color');
    expect(doc.variables).toEqual(before);
  });

  it('resolveVariable should resolve a $ref value', () => {
    vm.setVariable('accent', { type: 'color', value: '#3B82F6' });
    const resolved = vm.resolveVariable('$accent');
    expect(resolved).toBe('#3B82F6');
  });

  it('resolveVariable should return undefined for unknown ref', () => {
    const resolved = vm.resolveVariable('$nonexistent');
    expect(resolved).toBeUndefined();
  });

  it('setThemes should update document themes', () => {
    vm.setThemes({ 'Theme-1': ['Light', 'Dark'] });
    expect(doc.themes).toEqual({ 'Theme-1': ['Light', 'Dark'] });
  });
});
