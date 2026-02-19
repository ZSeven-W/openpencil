export interface ChatMessage {
  id: string
  role: 'user' | 'assistant'
  content: string
  timestamp: number
  isStreaming?: boolean
}

export interface AIDesignRequest {
  prompt: string
  context?: {
    selectedNodes?: string[]
    documentSummary?: string
    canvasSize?: { width: number; height: number }
  }
}

export interface AICodeRequest {
  prompt?: string
  format: 'react-tailwind' | 'html-css' | 'react-inline'
  nodeIds?: string[]
}

export interface AIStreamChunk {
  type: 'text' | 'thinking' | 'done' | 'error'
  content: string
}
