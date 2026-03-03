import { openDocument, saveDocument, resolveDocPath } from '../document-manager'
import type { VariableDefinition } from '../../types/variables'

export interface GetVariablesParams {
  filePath: string
}

export interface SetVariablesParams {
  filePath: string
  variables: Record<string, VariableDefinition>
  replace?: boolean
}

export async function handleGetVariables(
  params: GetVariablesParams,
): Promise<{ variables: Record<string, VariableDefinition>; themes: Record<string, string[]> }> {
  const filePath = resolveDocPath(params.filePath)
  const doc = await openDocument(filePath)
  return {
    variables: doc.variables ?? {},
    themes: doc.themes ?? {},
  }
}

export async function handleSetVariables(
  params: SetVariablesParams,
): Promise<{ variables: Record<string, VariableDefinition> }> {
  const filePath = resolveDocPath(params.filePath)
  const doc = await openDocument(filePath)

  if (params.replace) {
    doc.variables = params.variables
  } else {
    doc.variables = { ...(doc.variables ?? {}), ...params.variables }
  }

  await saveDocument(filePath, doc)
  return { variables: doc.variables }
}
