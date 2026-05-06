export type AIProviderType = 'anthropic' | 'openai' | 'opencode' | 'copilot' | 'gemini';

export interface AIProviderConfig {
  type: AIProviderType;
  displayName: string;
  isConnected: boolean;
  connectionMethod: 'claude-code' | 'codex-cli' | 'opencode' | 'copilot' | 'gemini-cli' | null;
  /** 当用户连接此提供商时获取 Models */
  models: GroupedModel[];
  /** Human-可读的连接状态，例如“Connected 通过 API 键” */
  connectionInfo?: string;
  /** Config 提示的文件路径（客户端呈现本地化文本） */
  hintPath?: string;
}

export type MCPCliTool =
  | 'claude-code'
  | 'codex-cli'
  | 'gemini-cli'
  | 'opencode-cli'
  | 'kiro-cli'
  | 'copilot-cli';

export type MCPTransportMode = 'stdio' | 'http' | 'both';

export interface MCPCliIntegration {
  tool: MCPCliTool;
  displayName: string;
  enabled: boolean;
  installed: boolean;
}

export interface GroupedModel {
  value: string;
  displayName: string;
  description: string;
  provider: AIProviderType | string;
  /** When 设置，此模型来自内置提供程序（API 密钥）而不是 CLI 工具 */
  builtinProviderId?: string;
}

export interface ModelGroup {
  provider: AIProviderType | string;
  providerName: string;
  models: GroupedModel[];
}

export interface AcpAgentConfig {
  id: string;
  displayName: string;
  connectionType: 'local' | 'remote';
  command?: string;
  args?: string[];
  env?: Record<string, string>;
  url?: string;
  enabled: boolean;
}
