export interface VariableDefinition {
  type: 'color' | 'number' | 'boolean' | 'string'
  value: VariableValue
}

export type VariableValue = string | number | boolean | ThemedValue[]

export interface ThemedValue {
  value: string | number | boolean
  theme?: Record<string, string>
}
