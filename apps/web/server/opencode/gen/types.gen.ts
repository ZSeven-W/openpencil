// This 文件由 @hey-api/openapi-ts 自动生成

export type ClientOptions = {
  baseUrl: `${string}://${string}` | (string & {});
};

export type EventInstallationUpdated = {
  type: 'installation.updated';
  properties: {
    version: string;
  };
};

export type EventInstallationUpdateAvailable = {
  type: 'installation.update-available';
  properties: {
    version: string;
  };
};

export type Project = {
  id: string;
  worktree: string;
  vcs?: 'git';
  name?: string;
  icon?: {
    url?: string;
    override?: string;
    color?: string;
  };
  commands?: {
    /**
     * 创建新工作区（工作树）时运行的 Startup 脚本
     */
    start?: string;
  };
  time: {
    created: number;
    updated: number;
    initialized?: number;
  };
  sandboxes: Array<string>;
};

export type EventProjectUpdated = {
  type: 'project.updated';
  properties: Project;
};

export type EventFileEdited = {
  type: 'file.edited';
  properties: {
    file: string;
  };
};

export type EventServerInstanceDisposed = {
  type: 'server.instance.disposed';
  properties: {
    directory: string;
  };
};

export type EventFileWatcherUpdated = {
  type: 'file.watcher.updated';
  properties: {
    file: string;
    event: 'add' | 'change' | 'unlink';
  };
};

export type PermissionRequest = {
  id: string;
  sessionID: string;
  permission: string;
  patterns: Array<string>;
  metadata: {
    [key: string]: unknown;
  };
  always: Array<string>;
  tool?: {
    messageID: string;
    callID: string;
  };
};

export type EventPermissionAsked = {
  type: 'permission.asked';
  properties: PermissionRequest;
};

export type EventPermissionReplied = {
  type: 'permission.replied';
  properties: {
    sessionID: string;
    requestID: string;
    reply: 'once' | 'always' | 'reject';
  };
};

export type EventVcsBranchUpdated = {
  type: 'vcs.branch.updated';
  properties: {
    branch?: string;
  };
};

export type QuestionOption = {
  /**
   * Display 文本（1-5 个字，简洁）
   */
  label: string;
  /**
   * 选择的 Explanation
   */
  description: string;
};

export type QuestionInfo = {
  /**
   * Complete 问题
   */
  question: string;
  /**
   * Very 短标签（最多 30 个字符）
   */
  header: string;
  /**
   * Available 选择
   */
  options: Array<QuestionOption>;
  /**
   * Allow 选择多项
   */
  multiple?: boolean;
  /**
   * Allow 输入自定义答案（默认值：true）
   */
  custom?: boolean;
};

export type QuestionRequest = {
  id: string;
  sessionID: string;
  /**
   * Questions 询问
   */
  questions: Array<QuestionInfo>;
  tool?: {
    messageID: string;
    callID: string;
  };
};

export type EventQuestionAsked = {
  type: 'question.asked';
  properties: QuestionRequest;
};

export type QuestionAnswer = Array<string>;

export type EventQuestionReplied = {
  type: 'question.replied';
  properties: {
    sessionID: string;
    requestID: string;
    answers: Array<QuestionAnswer>;
  };
};

export type EventQuestionRejected = {
  type: 'question.rejected';
  properties: {
    sessionID: string;
    requestID: string;
  };
};

export type EventServerConnected = {
  type: 'server.connected';
  properties: {
    [key: string]: unknown;
  };
};

export type EventGlobalDisposed = {
  type: 'global.disposed';
  properties: {
    [key: string]: unknown;
  };
};

export type EventLspClientDiagnostics = {
  type: 'lsp.client.diagnostics';
  properties: {
    serverID: string;
    path: string;
  };
};

export type EventLspUpdated = {
  type: 'lsp.updated';
  properties: {
    [key: string]: unknown;
  };
};

export type OutputFormatText = {
  type: 'text';
};

export type JsonSchema = {
  [key: string]: unknown;
};

export type OutputFormatJsonSchema = {
  type: 'json_schema';
  schema: JsonSchema;
  retryCount?: number;
};

export type OutputFormat = OutputFormatText | OutputFormatJsonSchema;

export type FileDiff = {
  file: string;
  before: string;
  after: string;
  additions: number;
  deletions: number;
  status?: 'added' | 'deleted' | 'modified';
};

export type UserMessage = {
  id: string;
  sessionID: string;
  role: 'user';
  time: {
    created: number;
  };
  format?: OutputFormat;
  summary?: {
    title?: string;
    body?: string;
    diffs: Array<FileDiff>;
  };
  agent: string;
  model: {
    providerID: string;
    modelID: string;
  };
  system?: string;
  tools?: {
    [key: string]: boolean;
  };
  variant?: string;
};

export type ProviderAuthError = {
  name: 'ProviderAuthError';
  data: {
    providerID: string;
    message: string;
  };
};

export type UnknownError = {
  name: 'UnknownError';
  data: {
    message: string;
  };
};

export type MessageOutputLengthError = {
  name: 'MessageOutputLengthError';
  data: {
    [key: string]: unknown;
  };
};

export type MessageAbortedError = {
  name: 'MessageAbortedError';
  data: {
    message: string;
  };
};

export type StructuredOutputError = {
  name: 'StructuredOutputError';
  data: {
    message: string;
    retries: number;
  };
};

export type ContextOverflowError = {
  name: 'ContextOverflowError';
  data: {
    message: string;
    responseBody?: string;
  };
};

export type ApiError = {
  name: 'APIError';
  data: {
    message: string;
    statusCode?: number;
    isRetryable: boolean;
    responseHeaders?: {
      [key: string]: string;
    };
    responseBody?: string;
    metadata?: {
      [key: string]: string;
    };
  };
};

export type AssistantMessage = {
  id: string;
  sessionID: string;
  role: 'assistant';
  time: {
    created: number;
    completed?: number;
  };
  error?:
    | ProviderAuthError
    | UnknownError
    | MessageOutputLengthError
    | MessageAbortedError
    | StructuredOutputError
    | ContextOverflowError
    | ApiError;
  parentID: string;
  modelID: string;
  providerID: string;
  mode: string;
  agent: string;
  path: {
    cwd: string;
    root: string;
  };
  summary?: boolean;
  cost: number;
  tokens: {
    total?: number;
    input: number;
    output: number;
    reasoning: number;
    cache: {
      read: number;
      write: number;
    };
  };
  structured?: unknown;
  variant?: string;
  finish?: string;
};

export type Message = UserMessage | AssistantMessage;

export type EventMessageUpdated = {
  type: 'message.updated';
  properties: {
    info: Message;
  };
};

export type EventMessageRemoved = {
  type: 'message.removed';
  properties: {
    sessionID: string;
    messageID: string;
  };
};

export type TextPart = {
  id: string;
  sessionID: string;
  messageID: string;
  type: 'text';
  text: string;
  synthetic?: boolean;
  ignored?: boolean;
  time?: {
    start: number;
    end?: number;
  };
  metadata?: {
    [key: string]: unknown;
  };
};

export type SubtaskPart = {
  id: string;
  sessionID: string;
  messageID: string;
  type: 'subtask';
  prompt: string;
  description: string;
  agent: string;
  model?: {
    providerID: string;
    modelID: string;
  };
  command?: string;
};

export type ReasoningPart = {
  id: string;
  sessionID: string;
  messageID: string;
  type: 'reasoning';
  text: string;
  metadata?: {
    [key: string]: unknown;
  };
  time: {
    start: number;
    end?: number;
  };
};

export type FilePartSourceText = {
  value: string;
  start: number;
  end: number;
};

export type FileSource = {
  text: FilePartSourceText;
  type: 'file';
  path: string;
};

export type Range = {
  start: {
    line: number;
    character: number;
  };
  end: {
    line: number;
    character: number;
  };
};

export type SymbolSource = {
  text: FilePartSourceText;
  type: 'symbol';
  path: string;
  range: Range;
  name: string;
  kind: number;
};

export type ResourceSource = {
  text: FilePartSourceText;
  type: 'resource';
  clientName: string;
  uri: string;
};

export type FilePartSource = FileSource | SymbolSource | ResourceSource;

export type FilePart = {
  id: string;
  sessionID: string;
  messageID: string;
  type: 'file';
  mime: string;
  filename?: string;
  url: string;
  source?: FilePartSource;
};

export type ToolStatePending = {
  status: 'pending';
  input: {
    [key: string]: unknown;
  };
  raw: string;
};

export type ToolStateRunning = {
  status: 'running';
  input: {
    [key: string]: unknown;
  };
  title?: string;
  metadata?: {
    [key: string]: unknown;
  };
  time: {
    start: number;
  };
};

export type ToolStateCompleted = {
  status: 'completed';
  input: {
    [key: string]: unknown;
  };
  output: string;
  title: string;
  metadata: {
    [key: string]: unknown;
  };
  time: {
    start: number;
    end: number;
    compacted?: number;
  };
  attachments?: Array<FilePart>;
};

export type ToolStateError = {
  status: 'error';
  input: {
    [key: string]: unknown;
  };
  error: string;
  metadata?: {
    [key: string]: unknown;
  };
  time: {
    start: number;
    end: number;
  };
};

export type ToolState = ToolStatePending | ToolStateRunning | ToolStateCompleted | ToolStateError;

export type ToolPart = {
  id: string;
  sessionID: string;
  messageID: string;
  type: 'tool';
  callID: string;
  tool: string;
  state: ToolState;
  metadata?: {
    [key: string]: unknown;
  };
};

export type StepStartPart = {
  id: string;
  sessionID: string;
  messageID: string;
  type: 'step-start';
  snapshot?: string;
};

export type StepFinishPart = {
  id: string;
  sessionID: string;
  messageID: string;
  type: 'step-finish';
  reason: string;
  snapshot?: string;
  cost: number;
  tokens: {
    total?: number;
    input: number;
    output: number;
    reasoning: number;
    cache: {
      read: number;
      write: number;
    };
  };
};

export type SnapshotPart = {
  id: string;
  sessionID: string;
  messageID: string;
  type: 'snapshot';
  snapshot: string;
};

export type PatchPart = {
  id: string;
  sessionID: string;
  messageID: string;
  type: 'patch';
  hash: string;
  files: Array<string>;
};

export type AgentPart = {
  id: string;
  sessionID: string;
  messageID: string;
  type: 'agent';
  name: string;
  source?: {
    value: string;
    start: number;
    end: number;
  };
};

export type RetryPart = {
  id: string;
  sessionID: string;
  messageID: string;
  type: 'retry';
  attempt: number;
  error: ApiError;
  time: {
    created: number;
  };
};

export type CompactionPart = {
  id: string;
  sessionID: string;
  messageID: string;
  type: 'compaction';
  auto: boolean;
  overflow?: boolean;
};

export type Part =
  | TextPart
  | SubtaskPart
  | ReasoningPart
  | FilePart
  | ToolPart
  | StepStartPart
  | StepFinishPart
  | SnapshotPart
  | PatchPart
  | AgentPart
  | RetryPart
  | CompactionPart;

export type EventMessagePartUpdated = {
  type: 'message.part.updated';
  properties: {
    part: Part;
  };
};

export type EventMessagePartDelta = {
  type: 'message.part.delta';
  properties: {
    sessionID: string;
    messageID: string;
    partID: string;
    field: string;
    delta: string;
  };
};

export type EventMessagePartRemoved = {
  type: 'message.part.removed';
  properties: {
    sessionID: string;
    messageID: string;
    partID: string;
  };
};

export type SessionStatus =
  | {
      type: 'idle';
    }
  | {
      type: 'retry';
      attempt: number;
      message: string;
      next: number;
    }
  | {
      type: 'busy';
    };

export type EventSessionStatus = {
  type: 'session.status';
  properties: {
    sessionID: string;
    status: SessionStatus;
  };
};

export type EventSessionIdle = {
  type: 'session.idle';
  properties: {
    sessionID: string;
  };
};

export type EventSessionCompacted = {
  type: 'session.compacted';
  properties: {
    sessionID: string;
  };
};

export type Todo = {
  /**
   * Brief 任务描述
   */
  content: string;
  /**
   * Current 任务状态：待处理、in_progress、已完成、已取消
   */
  status: string;
  /**
   * Priority 任务级别：高、中、低
   */
  priority: string;
};

export type EventTodoUpdated = {
  type: 'todo.updated';
  properties: {
    sessionID: string;
    todos: Array<Todo>;
  };
};

export type EventTuiPromptAppend = {
  type: 'tui.prompt.append';
  properties: {
    text: string;
  };
};

export type EventTuiCommandExecute = {
  type: 'tui.command.execute';
  properties: {
    command:
      | 'session.list'
      | 'session.new'
      | 'session.share'
      | 'session.interrupt'
      | 'session.compact'
      | 'session.page.up'
      | 'session.page.down'
      | 'session.line.up'
      | 'session.line.down'
      | 'session.half.page.up'
      | 'session.half.page.down'
      | 'session.first'
      | 'session.last'
      | 'prompt.clear'
      | 'prompt.submit'
      | 'agent.cycle'
      | string;
  };
};

export type EventTuiToastShow = {
  type: 'tui.toast.show';
  properties: {
    title?: string;
    message: string;
    variant: 'info' | 'success' | 'warning' | 'error';
    /**
     * Duration 以毫秒为单位
     */
    duration?: number;
  };
};

export type EventTuiSessionSelect = {
  type: 'tui.session.select';
  properties: {
    /**
     * Session ID 导航至
     */
    sessionID: string;
  };
};

export type EventMcpToolsChanged = {
  type: 'mcp.tools.changed';
  properties: {
    server: string;
  };
};

export type EventMcpBrowserOpenFailed = {
  type: 'mcp.browser.open.failed';
  properties: {
    mcpName: string;
    url: string;
  };
};

export type EventCommandExecuted = {
  type: 'command.executed';
  properties: {
    name: string;
    sessionID: string;
    arguments: string;
    messageID: string;
  };
};

export type PermissionAction = 'allow' | 'deny' | 'ask';

export type PermissionRule = {
  permission: string;
  pattern: string;
  action: PermissionAction;
};

export type PermissionRuleset = Array<PermissionRule>;

export type Session = {
  id: string;
  slug: string;
  projectID: string;
  workspaceID?: string;
  directory: string;
  parentID?: string;
  summary?: {
    additions: number;
    deletions: number;
    files: number;
    diffs?: Array<FileDiff>;
  };
  share?: {
    url: string;
  };
  title: string;
  version: string;
  time: {
    created: number;
    updated: number;
    compacting?: number;
    archived?: number;
  };
  permission?: PermissionRuleset;
  revert?: {
    messageID: string;
    partID?: string;
    snapshot?: string;
    diff?: string;
  };
};

export type EventSessionCreated = {
  type: 'session.created';
  properties: {
    info: Session;
  };
};

export type EventSessionUpdated = {
  type: 'session.updated';
  properties: {
    info: Session;
  };
};

export type EventSessionDeleted = {
  type: 'session.deleted';
  properties: {
    info: Session;
  };
};

export type EventSessionDiff = {
  type: 'session.diff';
  properties: {
    sessionID: string;
    diff: Array<FileDiff>;
  };
};

export type EventSessionError = {
  type: 'session.error';
  properties: {
    sessionID?: string;
    error?:
      | ProviderAuthError
      | UnknownError
      | MessageOutputLengthError
      | MessageAbortedError
      | StructuredOutputError
      | ContextOverflowError
      | ApiError;
  };
};

export type EventWorkspaceReady = {
  type: 'workspace.ready';
  properties: {
    name: string;
  };
};

export type EventWorkspaceFailed = {
  type: 'workspace.failed';
  properties: {
    message: string;
  };
};

export type Pty = {
  id: string;
  title: string;
  command: string;
  args: Array<string>;
  cwd: string;
  status: 'running' | 'exited';
  pid: number;
};

export type EventPtyCreated = {
  type: 'pty.created';
  properties: {
    info: Pty;
  };
};

export type EventPtyUpdated = {
  type: 'pty.updated';
  properties: {
    info: Pty;
  };
};

export type EventPtyExited = {
  type: 'pty.exited';
  properties: {
    id: string;
    exitCode: number;
  };
};

export type EventPtyDeleted = {
  type: 'pty.deleted';
  properties: {
    id: string;
  };
};

export type EventWorktreeReady = {
  type: 'worktree.ready';
  properties: {
    name: string;
    branch: string;
  };
};

export type EventWorktreeFailed = {
  type: 'worktree.failed';
  properties: {
    message: string;
  };
};

export type Event =
  | EventInstallationUpdated
  | EventInstallationUpdateAvailable
  | EventProjectUpdated
  | EventFileEdited
  | EventServerInstanceDisposed
  | EventFileWatcherUpdated
  | EventPermissionAsked
  | EventPermissionReplied
  | EventVcsBranchUpdated
  | EventQuestionAsked
  | EventQuestionReplied
  | EventQuestionRejected
  | EventServerConnected
  | EventGlobalDisposed
  | EventLspClientDiagnostics
  | EventLspUpdated
  | EventMessageUpdated
  | EventMessageRemoved
  | EventMessagePartUpdated
  | EventMessagePartDelta
  | EventMessagePartRemoved
  | EventSessionStatus
  | EventSessionIdle
  | EventSessionCompacted
  | EventTodoUpdated
  | EventTuiPromptAppend
  | EventTuiCommandExecute
  | EventTuiToastShow
  | EventTuiSessionSelect
  | EventMcpToolsChanged
  | EventMcpBrowserOpenFailed
  | EventCommandExecuted
  | EventSessionCreated
  | EventSessionUpdated
  | EventSessionDeleted
  | EventSessionDiff
  | EventSessionError
  | EventWorkspaceReady
  | EventWorkspaceFailed
  | EventPtyCreated
  | EventPtyUpdated
  | EventPtyExited
  | EventPtyDeleted
  | EventWorktreeReady
  | EventWorktreeFailed;

export type GlobalEvent = {
  directory: string;
  payload: Event;
};

/**
 * Log 等级
 */
export type LogLevel = 'DEBUG' | 'INFO' | 'WARN' | 'ERROR';

/**
 * Server opencode 服务和 Web 命令的配置
 */
export type ServerConfig = {
  /**
   * Port 收听
   */
  port?: number;
  /**
   * Hostname 收听
   */
  hostname?: string;
  /**
   * Enable mDNS 服务发现
   */
  mdns?: boolean;
  /**
   * Custom 服务的域名（默认：opencode.local）
   */
  mdnsDomain?: string;
  /**
   * 允许 CORS 的 Additional 域
   */
  cors?: Array<string>;
};

export type PermissionActionConfig = 'ask' | 'allow' | 'deny';

export type PermissionObjectConfig = {
  [key: string]: PermissionActionConfig;
};

export type PermissionRuleConfig = PermissionActionConfig | PermissionObjectConfig;

export type PermissionConfig =
  | {
      __originalKeys?: Array<string>;
      read?: PermissionRuleConfig;
      edit?: PermissionRuleConfig;
      glob?: PermissionRuleConfig;
      grep?: PermissionRuleConfig;
      list?: PermissionRuleConfig;
      bash?: PermissionRuleConfig;
      task?: PermissionRuleConfig;
      external_directory?: PermissionRuleConfig;
      todowrite?: PermissionActionConfig;
      todoread?: PermissionActionConfig;
      question?: PermissionActionConfig;
      webfetch?: PermissionActionConfig;
      websearch?: PermissionActionConfig;
      codesearch?: PermissionActionConfig;
      lsp?: PermissionRuleConfig;
      doom_loop?: PermissionActionConfig;
      skill?: PermissionRuleConfig;
      [key: string]: PermissionRuleConfig | Array<string> | PermissionActionConfig | undefined;
    }
  | PermissionActionConfig;

export type AgentConfig = {
  model?: string;
  /**
   * Default 此代理的模型变体（仅在使用代理的配置模型时适用）。
   */
  variant?: string;
  temperature?: number;
  top_p?: number;
  prompt?: string;
  /**
   * @deprecated Use 改为“权限”字段
   */
  tools?: {
    [key: string]: boolean;
  };
  disable?: boolean;
  /**
   * Description 何时使用代理
   */
  description?: string;
  mode?: 'subagent' | 'primary' | 'all';
  /**
   * Hide 来自@自动完成菜单的子代理（默认值：false，仅适用于模式：子代理）
   */
  hidden?: boolean;
  options?: {
    [key: string]: unknown;
  };
  /**
   * Hex 颜色代码（例如，#FF5733）或主题颜色（例如，原色）
   */
  color?: string | 'primary' | 'secondary' | 'accent' | 'success' | 'warning' | 'error' | 'info';
  /**
   * Maximum 强制仅文本响应之前的代理迭代次数
   */
  steps?: number;
  /**
   * @deprecated Use '步骤' 字段代替。
   */
  maxSteps?: number;
  permission?: PermissionConfig;
  [key: string]:
    | unknown
    | string
    | number
    | {
        [key: string]: boolean;
      }
    | boolean
    | 'subagent'
    | 'primary'
    | 'all'
    | {
        [key: string]: unknown;
      }
    | string
    | 'primary'
    | 'secondary'
    | 'accent'
    | 'success'
    | 'warning'
    | 'error'
    | 'info'
    | number
    | PermissionConfig
    | undefined;
};

export type ProviderConfig = {
  api?: string;
  name?: string;
  env?: Array<string>;
  id?: string;
  npm?: string;
  models?: {
    [key: string]: {
      id?: string;
      name?: string;
      family?: string;
      release_date?: string;
      attachment?: boolean;
      reasoning?: boolean;
      temperature?: boolean;
      tool_call?: boolean;
      interleaved?:
        | true
        | {
            field: 'reasoning_content' | 'reasoning_details';
          };
      cost?: {
        input: number;
        output: number;
        cache_read?: number;
        cache_write?: number;
        context_over_200k?: {
          input: number;
          output: number;
          cache_read?: number;
          cache_write?: number;
        };
      };
      limit?: {
        context: number;
        input?: number;
        output: number;
      };
      modalities?: {
        input: Array<'text' | 'audio' | 'image' | 'video' | 'pdf'>;
        output: Array<'text' | 'audio' | 'image' | 'video' | 'pdf'>;
      };
      experimental?: boolean;
      status?: 'alpha' | 'beta' | 'deprecated';
      options?: {
        [key: string]: unknown;
      };
      headers?: {
        [key: string]: string;
      };
      provider?: {
        npm?: string;
        api?: string;
      };
      /**
       * Variant 特定配置
       */
      variants?: {
        [key: string]: {
          /**
           * Disable 该模型的变体
           */
          disabled?: boolean;
          [key: string]: unknown | boolean | undefined;
        };
      };
    };
  };
  whitelist?: Array<string>;
  blacklist?: Array<string>;
  options?: {
    apiKey?: string;
    baseURL?: string;
    /**
     * GitHub Enterprise URL 用于副驾驶身份验证
     */
    enterpriseUrl?: string;
    /**
     * 此提供商的 Enable promptCacheKey （默认 false）
     */
    setCacheKey?: boolean;
    /**
     * Timeout 向此提供程序发出的请求以毫秒为单位。 Default 是 300000（5 分钟）。 Set 设置为 false 以禁用超时。
     */
    timeout?: number | false;
    /**
     * 此提供程序的流式传输 SSE 块之间的 Timeout（以毫秒为单位）。 If 在此窗口内没有块到达，请求被中止。
     */
    chunkTimeout?: number;
    [key: string]: unknown | string | boolean | number | false | number | undefined;
  };
};

export type McpLocalConfig = {
  /**
   * MCP 服务器连接的 Type
   */
  type: 'local';
  /**
   * Command 和运行 MCP 服务器的参数
   */
  command: Array<string>;
  /**
   * 运行 MCP 服务器时要设置的 Environment 变量
   */
  environment?: {
    [key: string]: string;
  };
  /**
   * Enable 或在启动时禁用 MCP 服务器
   */
  enabled?: boolean;
  /**
   * MCP 服务器请求的 Timeout（以毫秒为单位）。如果未指定，则 Defaults 为 5000（5 秒）。
   */
  timeout?: number;
};

export type McpOAuthConfig = {
  /**
   * OAuth 客户端 ID。未提供 If，将尝试动态客户端注册 (RFC 7591)。
   */
  clientId?: string;
  /**
   * OAuth 客户端密钥（如果授权服务器需要）
   */
  clientSecret?: string;
  /**
   * OAuth 授权期间请求的范围
   */
  scope?: string;
};

export type McpRemoteConfig = {
  /**
   * MCP 服务器连接的 Type
   */
  type: 'remote';
  /**
   * 远程 MCP 服务器的 URL
   */
  url: string;
  /**
   * Enable 或在启动时禁用 MCP 服务器
   */
  enabled?: boolean;
  /**
   * Headers 与请求一起发送
   */
  headers?: {
    [key: string]: string;
  };
  /**
   * OAuth 服务器的 OAuth 身份验证配置。 Set 设置为 false 以禁用 OAuth 自动检测。
   */
  oauth?: McpOAuthConfig | false;
  /**
   * MCP 服务器请求的 Timeout（以毫秒为单位）。如果未指定，则 Defaults 为 5000（5 秒）。
   */
  timeout?: number;
};

/**
 * @deprecated Always 使用拉伸布局。
 */
export type LayoutConfig = 'auto' | 'stretch';

export type Config = {
  /**
   * 用于配置验证的 JSON 架构参考
   */
  $schema?: string;
  logLevel?: LogLevel;
  server?: ServerConfig;
  /**
   * Command 配置，参见 https://opencode.ai/docs/commands
   */
  command?: {
    [key: string]: {
      template: string;
      description?: string;
      agent?: string;
      model?: string;
      subtask?: boolean;
    };
  };
  /**
   * Additional 技能文件夹路径
   */
  skills?: {
    /**
     * Additional 技能文件夹的路径
     */
    paths?: Array<string>;
    /**
     * URLs 从中获取技能（例如，https://example.com/.well-known/skills/)
     */
    urls?: Array<string>;
  };
  watcher?: {
    ignore?: Array<string>;
  };
  plugin?: Array<string>;
  /**
   * Enable 或禁用快照跟踪。 When false，不记录文件系统快照，撤消或恢复不会对 undo/redo 文件更改。 Defaults 为真。
   */
  snapshot?: boolean;
  /**
   * Control 共享行为：'manual' 允许通过命令手动共享，'auto' 启用自动共享，'disabled' 禁用所有共享
   */
  share?: 'manual' | 'auto' | 'disabled';
  /**
   * @deprecated Use 改为“共享”字段。 Share 自动新建会话
   */
  autoshare?: boolean;
  /**
   * Automatically 更新到最新版本。 Set 设置为 true 表示自动更新，设置为 false 表示禁用，或“notify”表示显示更新通知
   */
  autoupdate?: boolean | 'notify';
  /**
   * 自动加载的 Disable 提供程序
   */
  disabled_providers?: Array<string>;
  /**
   * When 设置，ONLY 这些提供程序将被启用。 All 其他提供商将被忽略
   */
  enabled_providers?: Array<string>;
  /**
   * Model 以 provider/model 的格式使用，例如 anthropic/claude-2
   */
  model?: string;
  /**
   * Small 模型，用于以 provider/model 格式生成标题等任务
   */
  small_model?: string;
  /**
   * 没有指定时使用的 Default 代理。 Must 是主要代理。如果未设置或指定的代理无效，Falls 返回“构建”。
   */
  default_agent?: string;
  /**
   * Custom 在对话中显示的用户名而不是系统用户名
   */
  username?: string;
  /**
   * @deprecated Use 改为 `agent` 字段。
   */
  mode?: {
    build?: AgentConfig;
    plan?: AgentConfig;
    [key: string]: AgentConfig | undefined;
  };
  /**
   * Agent 配置，参见 https://opencode.ai/docs/agents
   */
  agent?: {
    plan?: AgentConfig;
    build?: AgentConfig;
    general?: AgentConfig;
    explore?: AgentConfig;
    title?: AgentConfig;
    summary?: AgentConfig;
    compaction?: AgentConfig;
    [key: string]: AgentConfig | undefined;
  };
  /**
   * Custom 提供程序配置和模型覆盖
   */
  provider?: {
    [key: string]: ProviderConfig;
  };
  /**
   * MCP (Model Context Protocol) 服务器配置
   */
  mcp?: {
    [key: string]:
      | McpLocalConfig
      | McpRemoteConfig
      | {
          enabled: boolean;
        };
  };
  formatter?:
    | false
    | {
        [key: string]: {
          disabled?: boolean;
          command?: Array<string>;
          environment?: {
            [key: string]: string;
          };
          extensions?: Array<string>;
        };
      };
  lsp?:
    | false
    | {
        [key: string]:
          | {
              disabled: true;
            }
          | {
              command: Array<string>;
              extensions?: Array<string>;
              disabled?: boolean;
              env?: {
                [key: string]: string;
              };
              initialization?: {
                [key: string]: unknown;
              };
            };
      };
  /**
   * Additional 要包含的指令文件或模式
   */
  instructions?: Array<string>;
  layout?: LayoutConfig;
  permission?: PermissionConfig;
  tools?: {
    [key: string]: boolean;
  };
  enterprise?: {
    /**
     * Enterprise URL
     */
    url?: string;
  };
  compaction?: {
    /**
     * Enable 当上下文已满时自动压缩（默认值：true）
     */
    auto?: boolean;
    /**
     * Enable 修剪旧工具输出（默认值：true）
     */
    prune?: boolean;
    /**
     * Token 用于压缩的缓冲区。 Leaves 足够的窗口以避免压缩期间溢出。
     */
    reserved?: number;
  };
  experimental?: {
    disable_paste_summary?: boolean;
    /**
     * Enable 批处理工具
     */
    batch_tool?: boolean;
    /**
     * Enable OpenTelemetry 跨越 AI SDK 调用（使用“experimental_telemetry”标志）
     */
    openTelemetry?: boolean;
    /**
     * Tools 应该仅对主要代理可用。
     */
    primary_tools?: Array<string>;
    /**
     * Continue 工具调用被拒绝时代理循环
     */
    continue_loop_on_deny?: boolean;
    /**
     * 模型上下文协议 (MCP) 请求的 Timeout（以毫秒为单位）
     */
    mcp_timeout?: number;
  };
};

export type BadRequestError = {
  data: unknown;
  errors: Array<{
    [key: string]: unknown;
  }>;
  success: false;
};

export type OAuth = {
  type: 'oauth';
  refresh: string;
  access: string;
  expires: number;
  accountId?: string;
  enterpriseUrl?: string;
};

export type ApiAuth = {
  type: 'api';
  key: string;
};

export type WellKnownAuth = {
  type: 'wellknown';
  key: string;
  token: string;
};

export type Auth = OAuth | ApiAuth | WellKnownAuth;

export type NotFoundError = {
  name: 'NotFoundError';
  data: {
    message: string;
  };
};

export type Model = {
  id: string;
  providerID: string;
  api: {
    id: string;
    url: string;
    npm: string;
  };
  name: string;
  family?: string;
  capabilities: {
    temperature: boolean;
    reasoning: boolean;
    attachment: boolean;
    toolcall: boolean;
    input: {
      text: boolean;
      audio: boolean;
      image: boolean;
      video: boolean;
      pdf: boolean;
    };
    output: {
      text: boolean;
      audio: boolean;
      image: boolean;
      video: boolean;
      pdf: boolean;
    };
    interleaved:
      | boolean
      | {
          field: 'reasoning_content' | 'reasoning_details';
        };
  };
  cost: {
    input: number;
    output: number;
    cache: {
      read: number;
      write: number;
    };
    experimentalOver200K?: {
      input: number;
      output: number;
      cache: {
        read: number;
        write: number;
      };
    };
  };
  limit: {
    context: number;
    input?: number;
    output: number;
  };
  status: 'alpha' | 'beta' | 'deprecated' | 'active';
  options: {
    [key: string]: unknown;
  };
  headers: {
    [key: string]: string;
  };
  release_date: string;
  variants?: {
    [key: string]: {
      [key: string]: unknown;
    };
  };
};

export type Provider = {
  id: string;
  name: string;
  source: 'env' | 'config' | 'custom' | 'api';
  env: Array<string>;
  key?: string;
  options: {
    [key: string]: unknown;
  };
  models: {
    [key: string]: Model;
  };
};

export type ToolIds = Array<string>;

export type ToolListItem = {
  id: string;
  description: string;
  parameters: unknown;
};

export type ToolList = Array<ToolListItem>;

export type Workspace = {
  id: string;
  type: string;
  branch: string | null;
  name: string | null;
  directory: string | null;
  extra: unknown | null;
  projectID: string;
};

export type Worktree = {
  name: string;
  branch: string;
  directory: string;
};

export type WorktreeCreateInput = {
  name?: string;
  /**
   * Additional 启动脚本在项目启动命令后运行
   */
  startCommand?: string;
};

export type WorktreeRemoveInput = {
  directory: string;
};

export type WorktreeResetInput = {
  directory: string;
};

export type ProjectSummary = {
  id: string;
  name?: string;
  worktree: string;
};

export type GlobalSession = {
  id: string;
  slug: string;
  projectID: string;
  workspaceID?: string;
  directory: string;
  parentID?: string;
  summary?: {
    additions: number;
    deletions: number;
    files: number;
    diffs?: Array<FileDiff>;
  };
  share?: {
    url: string;
  };
  title: string;
  version: string;
  time: {
    created: number;
    updated: number;
    compacting?: number;
    archived?: number;
  };
  permission?: PermissionRuleset;
  revert?: {
    messageID: string;
    partID?: string;
    snapshot?: string;
    diff?: string;
  };
  project: ProjectSummary | null;
};

export type McpResource = {
  name: string;
  uri: string;
  description?: string;
  mimeType?: string;
  client: string;
};

export type TextPartInput = {
  id?: string;
  type: 'text';
  text: string;
  synthetic?: boolean;
  ignored?: boolean;
  time?: {
    start: number;
    end?: number;
  };
  metadata?: {
    [key: string]: unknown;
  };
};

export type FilePartInput = {
  id?: string;
  type: 'file';
  mime: string;
  filename?: string;
  url: string;
  source?: FilePartSource;
};

export type AgentPartInput = {
  id?: string;
  type: 'agent';
  name: string;
  source?: {
    value: string;
    start: number;
    end: number;
  };
};

export type SubtaskPartInput = {
  id?: string;
  type: 'subtask';
  prompt: string;
  description: string;
  agent: string;
  model?: {
    providerID: string;
    modelID: string;
  };
  command?: string;
};

export type ProviderAuthMethod = {
  type: 'oauth' | 'api';
  label: string;
  prompts?: Array<
    | {
        type: 'text';
        key: string;
        message: string;
        placeholder?: string;
        when?: {
          key: string;
          op: 'eq' | 'neq';
          value: string;
        };
      }
    | {
        type: 'select';
        key: string;
        message: string;
        options: Array<{
          label: string;
          value: string;
          hint?: string;
        }>;
        when?: {
          key: string;
          op: 'eq' | 'neq';
          value: string;
        };
      }
  >;
};

export type ProviderAuthAuthorization = {
  url: string;
  method: 'auto' | 'code';
  instructions: string;
};

export type Symbol = {
  name: string;
  kind: number;
  location: {
    uri: string;
    range: Range;
  };
};

export type FileNode = {
  name: string;
  path: string;
  absolute: string;
  type: 'file' | 'directory';
  ignored: boolean;
};

export type FileContent = {
  type: 'text' | 'binary';
  content: string;
  diff?: string;
  patch?: {
    oldFileName: string;
    newFileName: string;
    oldHeader?: string;
    newHeader?: string;
    hunks: Array<{
      oldStart: number;
      oldLines: number;
      newStart: number;
      newLines: number;
      lines: Array<string>;
    }>;
    index?: string;
  };
  encoding?: 'base64';
  mimeType?: string;
};

export type File = {
  path: string;
  added: number;
  removed: number;
  status: 'added' | 'deleted' | 'modified';
};

export type McpStatusConnected = {
  status: 'connected';
};

export type McpStatusDisabled = {
  status: 'disabled';
};

export type McpStatusFailed = {
  status: 'failed';
  error: string;
};

export type McpStatusNeedsAuth = {
  status: 'needs_auth';
};

export type McpStatusNeedsClientRegistration = {
  status: 'needs_client_registration';
  error: string;
};

export type McpStatus =
  | McpStatusConnected
  | McpStatusDisabled
  | McpStatusFailed
  | McpStatusNeedsAuth
  | McpStatusNeedsClientRegistration;

export type Path = {
  home: string;
  state: string;
  config: string;
  worktree: string;
  directory: string;
};

export type VcsInfo = {
  branch: string;
};

export type Command = {
  name: string;
  description?: string;
  agent?: string;
  model?: string;
  source?: 'command' | 'mcp' | 'skill';
  template: string;
  subtask?: boolean;
  hints: Array<string>;
};

export type Agent = {
  name: string;
  description?: string;
  mode: 'subagent' | 'primary' | 'all';
  native?: boolean;
  hidden?: boolean;
  topP?: number;
  temperature?: number;
  color?: string;
  permission: PermissionRuleset;
  model?: {
    modelID: string;
    providerID: string;
  };
  variant?: string;
  prompt?: string;
  options: {
    [key: string]: unknown;
  };
  steps?: number;
};

export type LspStatus = {
  id: string;
  name: string;
  root: string;
  status: 'connected' | 'error';
};

export type FormatterStatus = {
  name: string;
  extensions: Array<string>;
  enabled: boolean;
};

export type GlobalHealthData = {
  body?: never;
  path?: never;
  query?: never;
  url: '/global/health';
};

export type GlobalHealthResponses = {
  /**
   * Health 信息
   */
  200: {
    healthy: true;
    version: string;
  };
};

export type GlobalHealthResponse = GlobalHealthResponses[keyof GlobalHealthResponses];

export type GlobalEventData = {
  body?: never;
  path?: never;
  query?: never;
  url: '/global/event';
};

export type GlobalEventResponses = {
  /**
   * Event 流
   */
  200: GlobalEvent;
};

export type GlobalEventResponse = GlobalEventResponses[keyof GlobalEventResponses];

export type GlobalConfigGetData = {
  body?: never;
  path?: never;
  query?: never;
  url: '/global/config';
};

export type GlobalConfigGetResponses = {
  /**
   * Get 全局配置信息
   */
  200: Config;
};

export type GlobalConfigGetResponse = GlobalConfigGetResponses[keyof GlobalConfigGetResponses];

export type GlobalConfigUpdateData = {
  body?: Config;
  path?: never;
  query?: never;
  url: '/global/config';
};

export type GlobalConfigUpdateErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
};

export type GlobalConfigUpdateError = GlobalConfigUpdateErrors[keyof GlobalConfigUpdateErrors];

export type GlobalConfigUpdateResponses = {
  /**
   * Successfully 更新了全局配置
   */
  200: Config;
};

export type GlobalConfigUpdateResponse =
  GlobalConfigUpdateResponses[keyof GlobalConfigUpdateResponses];

export type GlobalDisposeData = {
  body?: never;
  path?: never;
  query?: never;
  url: '/global/dispose';
};

export type GlobalDisposeResponses = {
  /**
   * Global 已处置
   */
  200: boolean;
};

export type GlobalDisposeResponse = GlobalDisposeResponses[keyof GlobalDisposeResponses];

export type AuthRemoveData = {
  body?: never;
  path: {
    providerID: string;
  };
  query?: never;
  url: '/auth/{providerID}';
};

export type AuthRemoveErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
};

export type AuthRemoveError = AuthRemoveErrors[keyof AuthRemoveErrors];

export type AuthRemoveResponses = {
  /**
   * Successfully 删除了身份验证凭据
   */
  200: boolean;
};

export type AuthRemoveResponse = AuthRemoveResponses[keyof AuthRemoveResponses];

export type AuthSetData = {
  body?: Auth;
  path: {
    providerID: string;
  };
  query?: never;
  url: '/auth/{providerID}';
};

export type AuthSetErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
};

export type AuthSetError = AuthSetErrors[keyof AuthSetErrors];

export type AuthSetResponses = {
  /**
   * Successfully 设置身份验证凭据
   */
  200: boolean;
};

export type AuthSetResponse = AuthSetResponses[keyof AuthSetResponses];

export type ProjectListData = {
  body?: never;
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/project';
};

export type ProjectListResponses = {
  /**
   * List 项目
   */
  200: Array<Project>;
};

export type ProjectListResponse = ProjectListResponses[keyof ProjectListResponses];

export type ProjectCurrentData = {
  body?: never;
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/project/current';
};

export type ProjectCurrentResponses = {
  /**
   * Current 项目信息
   */
  200: Project;
};

export type ProjectCurrentResponse = ProjectCurrentResponses[keyof ProjectCurrentResponses];

export type ProjectInitGitData = {
  body?: never;
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/project/git/init';
};

export type ProjectInitGitResponses = {
  /**
   * git 初始化后 Project 信息
   */
  200: Project;
};

export type ProjectInitGitResponse = ProjectInitGitResponses[keyof ProjectInitGitResponses];

export type ProjectUpdateData = {
  body?: {
    name?: string;
    icon?: {
      url?: string;
      override?: string;
      color?: string;
    };
    commands?: {
      /**
       * 创建新工作区（工作树）时运行的 Startup 脚本
       */
      start?: string;
    };
  };
  path: {
    projectID: string;
  };
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/project/{projectID}';
};

export type ProjectUpdateErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
  /**
   * 找到了 Not
   */
  404: NotFoundError;
};

export type ProjectUpdateError = ProjectUpdateErrors[keyof ProjectUpdateErrors];

export type ProjectUpdateResponses = {
  /**
   * Updated 项目信息
   */
  200: Project;
};

export type ProjectUpdateResponse = ProjectUpdateResponses[keyof ProjectUpdateResponses];

export type PtyListData = {
  body?: never;
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/pty';
};

export type PtyListResponses = {
  /**
   * List 会话数
   */
  200: Array<Pty>;
};

export type PtyListResponse = PtyListResponses[keyof PtyListResponses];

export type PtyCreateData = {
  body?: {
    command?: string;
    args?: Array<string>;
    cwd?: string;
    title?: string;
    env?: {
      [key: string]: string;
    };
  };
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/pty';
};

export type PtyCreateErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
};

export type PtyCreateError = PtyCreateErrors[keyof PtyCreateErrors];

export type PtyCreateResponses = {
  /**
   * Created 会话
   */
  200: Pty;
};

export type PtyCreateResponse = PtyCreateResponses[keyof PtyCreateResponses];

export type PtyRemoveData = {
  body?: never;
  path: {
    ptyID: string;
  };
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/pty/{ptyID}';
};

export type PtyRemoveErrors = {
  /**
   * 找到了 Not
   */
  404: NotFoundError;
};

export type PtyRemoveError = PtyRemoveErrors[keyof PtyRemoveErrors];

export type PtyRemoveResponses = {
  /**
   * Session 已删除
   */
  200: boolean;
};

export type PtyRemoveResponse = PtyRemoveResponses[keyof PtyRemoveResponses];

export type PtyGetData = {
  body?: never;
  path: {
    ptyID: string;
  };
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/pty/{ptyID}';
};

export type PtyGetErrors = {
  /**
   * 找到了 Not
   */
  404: NotFoundError;
};

export type PtyGetError = PtyGetErrors[keyof PtyGetErrors];

export type PtyGetResponses = {
  /**
   * Session 信息
   */
  200: Pty;
};

export type PtyGetResponse = PtyGetResponses[keyof PtyGetResponses];

export type PtyUpdateData = {
  body?: {
    title?: string;
    size?: {
      rows: number;
      cols: number;
    };
  };
  path: {
    ptyID: string;
  };
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/pty/{ptyID}';
};

export type PtyUpdateErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
};

export type PtyUpdateError = PtyUpdateErrors[keyof PtyUpdateErrors];

export type PtyUpdateResponses = {
  /**
   * Updated 会话
   */
  200: Pty;
};

export type PtyUpdateResponse = PtyUpdateResponses[keyof PtyUpdateResponses];

export type PtyConnectData = {
  body?: never;
  path: {
    ptyID: string;
  };
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/pty/{ptyID}/connect';
};

export type PtyConnectErrors = {
  /**
   * 找到了 Not
   */
  404: NotFoundError;
};

export type PtyConnectError = PtyConnectErrors[keyof PtyConnectErrors];

export type PtyConnectResponses = {
  /**
   * Connected 会话
   */
  200: boolean;
};

export type PtyConnectResponse = PtyConnectResponses[keyof PtyConnectResponses];

export type ConfigGetData = {
  body?: never;
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/config';
};

export type ConfigGetResponses = {
  /**
   * Get 配置信息
   */
  200: Config;
};

export type ConfigGetResponse = ConfigGetResponses[keyof ConfigGetResponses];

export type ConfigUpdateData = {
  body?: Config;
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/config';
};

export type ConfigUpdateErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
};

export type ConfigUpdateError = ConfigUpdateErrors[keyof ConfigUpdateErrors];

export type ConfigUpdateResponses = {
  /**
   * Successfully 更新配置
   */
  200: Config;
};

export type ConfigUpdateResponse = ConfigUpdateResponses[keyof ConfigUpdateResponses];

export type ConfigProvidersData = {
  body?: never;
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/config/providers';
};

export type ConfigProvidersResponses = {
  /**
   * List 提供商
   */
  200: {
    providers: Array<Provider>;
    default: {
      [key: string]: string;
    };
  };
};

export type ConfigProvidersResponse = ConfigProvidersResponses[keyof ConfigProvidersResponses];

export type ToolIdsData = {
  body?: never;
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/experimental/tool/ids';
};

export type ToolIdsErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
};

export type ToolIdsError = ToolIdsErrors[keyof ToolIdsErrors];

export type ToolIdsResponses = {
  /**
   * Tool IDs
   */
  200: ToolIds;
};

export type ToolIdsResponse = ToolIdsResponses[keyof ToolIdsResponses];

export type ToolListData = {
  body?: never;
  path?: never;
  query: {
    directory?: string;
    workspace?: string;
    provider: string;
    model: string;
  };
  url: '/experimental/tool';
};

export type ToolListErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
};

export type ToolListError = ToolListErrors[keyof ToolListErrors];

export type ToolListResponses = {
  /**
   * Tools
   */
  200: ToolList;
};

export type ToolListResponse = ToolListResponses[keyof ToolListResponses];

export type ExperimentalWorkspaceListData = {
  body?: never;
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/experimental/workspace';
};

export type ExperimentalWorkspaceListResponses = {
  /**
   * Workspaces
   */
  200: Array<Workspace>;
};

export type ExperimentalWorkspaceListResponse =
  ExperimentalWorkspaceListResponses[keyof ExperimentalWorkspaceListResponses];

export type ExperimentalWorkspaceCreateData = {
  body?: {
    id?: string;
    type: string;
    branch: string | null;
    extra: unknown | null;
  };
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/experimental/workspace';
};

export type ExperimentalWorkspaceCreateErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
};

export type ExperimentalWorkspaceCreateError =
  ExperimentalWorkspaceCreateErrors[keyof ExperimentalWorkspaceCreateErrors];

export type ExperimentalWorkspaceCreateResponses = {
  /**
   * Workspace 创建
   */
  200: Workspace;
};

export type ExperimentalWorkspaceCreateResponse =
  ExperimentalWorkspaceCreateResponses[keyof ExperimentalWorkspaceCreateResponses];

export type ExperimentalWorkspaceRemoveData = {
  body?: never;
  path: {
    id: string;
  };
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/experimental/workspace/{id}';
};

export type ExperimentalWorkspaceRemoveErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
};

export type ExperimentalWorkspaceRemoveError =
  ExperimentalWorkspaceRemoveErrors[keyof ExperimentalWorkspaceRemoveErrors];

export type ExperimentalWorkspaceRemoveResponses = {
  /**
   * Workspace 已删除
   */
  200: Workspace;
};

export type ExperimentalWorkspaceRemoveResponse =
  ExperimentalWorkspaceRemoveResponses[keyof ExperimentalWorkspaceRemoveResponses];

export type WorktreeRemoveData = {
  body?: WorktreeRemoveInput;
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/experimental/worktree';
};

export type WorktreeRemoveErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
};

export type WorktreeRemoveError = WorktreeRemoveErrors[keyof WorktreeRemoveErrors];

export type WorktreeRemoveResponses = {
  /**
   * Worktree 已删除
   */
  200: boolean;
};

export type WorktreeRemoveResponse = WorktreeRemoveResponses[keyof WorktreeRemoveResponses];

export type WorktreeListData = {
  body?: never;
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/experimental/worktree';
};

export type WorktreeListResponses = {
  /**
   * 工作树目录的 List
   */
  200: Array<string>;
};

export type WorktreeListResponse = WorktreeListResponses[keyof WorktreeListResponses];

export type WorktreeCreateData = {
  body?: WorktreeCreateInput;
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/experimental/worktree';
};

export type WorktreeCreateErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
};

export type WorktreeCreateError = WorktreeCreateErrors[keyof WorktreeCreateErrors];

export type WorktreeCreateResponses = {
  /**
   * Worktree 创建
   */
  200: Worktree;
};

export type WorktreeCreateResponse = WorktreeCreateResponses[keyof WorktreeCreateResponses];

export type WorktreeResetData = {
  body?: WorktreeResetInput;
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/experimental/worktree/reset';
};

export type WorktreeResetErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
};

export type WorktreeResetError = WorktreeResetErrors[keyof WorktreeResetErrors];

export type WorktreeResetResponses = {
  /**
   * Worktree 重置
   */
  200: boolean;
};

export type WorktreeResetResponse = WorktreeResetResponses[keyof WorktreeResetResponses];

export type ExperimentalSessionListData = {
  body?: never;
  path?: never;
  query?: {
    /**
     * Filter 会话（按项目目录）
     */
    directory?: string;
    workspace?: string;
    /**
     * Only 返回根会话（无 parentID）
     */
    roots?: boolean;
    /**
     * Filter 会话在此时间戳记或之后更新（自纪元以来的毫秒数）
     */
    start?: number;
    /**
     * Return 会话在此时间戳之前更新（自纪元以来的毫秒数）
     */
    cursor?: number;
    /**
     * Filter 会话按标题（不区分大小写）
     */
    search?: string;
    /**
     * Maximum 返回的会话数
     */
    limit?: number;
    /**
     * Include 存档会话（默认 false）
     */
    archived?: boolean;
  };
  url: '/experimental/session';
};

export type ExperimentalSessionListResponses = {
  /**
   * List 会话数
   */
  200: Array<GlobalSession>;
};

export type ExperimentalSessionListResponse =
  ExperimentalSessionListResponses[keyof ExperimentalSessionListResponses];

export type ExperimentalResourceListData = {
  body?: never;
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/experimental/resource';
};

export type ExperimentalResourceListResponses = {
  /**
   * MCP 资源
   */
  200: {
    [key: string]: McpResource;
  };
};

export type ExperimentalResourceListResponse =
  ExperimentalResourceListResponses[keyof ExperimentalResourceListResponses];

export type SessionListData = {
  body?: never;
  path?: never;
  query?: {
    /**
     * Filter 会话（按项目目录）
     */
    directory?: string;
    workspace?: string;
    /**
     * Only 返回根会话（无 parentID）
     */
    roots?: boolean;
    /**
     * Filter 会话在此时间戳记或之后更新（自纪元以来的毫秒数）
     */
    start?: number;
    /**
     * Filter 会话按标题（不区分大小写）
     */
    search?: string;
    /**
     * Maximum 返回的会话数
     */
    limit?: number;
  };
  url: '/session';
};

export type SessionListResponses = {
  /**
   * List 会话数
   */
  200: Array<Session>;
};

export type SessionListResponse = SessionListResponses[keyof SessionListResponses];

export type SessionCreateData = {
  body?: {
    parentID?: string;
    title?: string;
    permission?: PermissionRuleset;
    workspaceID?: string;
  };
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/session';
};

export type SessionCreateErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
};

export type SessionCreateError = SessionCreateErrors[keyof SessionCreateErrors];

export type SessionCreateResponses = {
  /**
   * Successfully 创建会话
   */
  200: Session;
};

export type SessionCreateResponse = SessionCreateResponses[keyof SessionCreateResponses];

export type SessionStatusData = {
  body?: never;
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/session/status';
};

export type SessionStatusErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
};

export type SessionStatusError = SessionStatusErrors[keyof SessionStatusErrors];

export type SessionStatusResponses = {
  /**
   * Get 会话状态
   */
  200: {
    [key: string]: SessionStatus;
  };
};

export type SessionStatusResponse = SessionStatusResponses[keyof SessionStatusResponses];

export type SessionDeleteData = {
  body?: never;
  path: {
    sessionID: string;
  };
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/session/{sessionID}';
};

export type SessionDeleteErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
  /**
   * 找到了 Not
   */
  404: NotFoundError;
};

export type SessionDeleteError = SessionDeleteErrors[keyof SessionDeleteErrors];

export type SessionDeleteResponses = {
  /**
   * Successfully 已删除会话
   */
  200: boolean;
};

export type SessionDeleteResponse = SessionDeleteResponses[keyof SessionDeleteResponses];

export type SessionGetData = {
  body?: never;
  path: {
    sessionID: string;
  };
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/session/{sessionID}';
};

export type SessionGetErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
  /**
   * 找到了 Not
   */
  404: NotFoundError;
};

export type SessionGetError = SessionGetErrors[keyof SessionGetErrors];

export type SessionGetResponses = {
  /**
   * Get 会话
   */
  200: Session;
};

export type SessionGetResponse = SessionGetResponses[keyof SessionGetResponses];

export type SessionUpdateData = {
  body?: {
    title?: string;
    time?: {
      archived?: number;
    };
  };
  path: {
    sessionID: string;
  };
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/session/{sessionID}';
};

export type SessionUpdateErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
  /**
   * 找到了 Not
   */
  404: NotFoundError;
};

export type SessionUpdateError = SessionUpdateErrors[keyof SessionUpdateErrors];

export type SessionUpdateResponses = {
  /**
   * Successfully 更新了会话
   */
  200: Session;
};

export type SessionUpdateResponse = SessionUpdateResponses[keyof SessionUpdateResponses];

export type SessionChildrenData = {
  body?: never;
  path: {
    sessionID: string;
  };
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/session/{sessionID}/children';
};

export type SessionChildrenErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
  /**
   * 找到了 Not
   */
  404: NotFoundError;
};

export type SessionChildrenError = SessionChildrenErrors[keyof SessionChildrenErrors];

export type SessionChildrenResponses = {
  /**
   * 孩子们的 List
   */
  200: Array<Session>;
};

export type SessionChildrenResponse = SessionChildrenResponses[keyof SessionChildrenResponses];

export type SessionTodoData = {
  body?: never;
  path: {
    sessionID: string;
  };
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/session/{sessionID}/todo';
};

export type SessionTodoErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
  /**
   * 找到了 Not
   */
  404: NotFoundError;
};

export type SessionTodoError = SessionTodoErrors[keyof SessionTodoErrors];

export type SessionTodoResponses = {
  /**
   * Todo 清单
   */
  200: Array<Todo>;
};

export type SessionTodoResponse = SessionTodoResponses[keyof SessionTodoResponses];

export type SessionInitData = {
  body?: {
    modelID: string;
    providerID: string;
    messageID: string;
  };
  path: {
    sessionID: string;
  };
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/session/{sessionID}/init';
};

export type SessionInitErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
  /**
   * 找到了 Not
   */
  404: NotFoundError;
};

export type SessionInitError = SessionInitErrors[keyof SessionInitErrors];

export type SessionInitResponses = {
  /**
   * 200
   */
  200: boolean;
};

export type SessionInitResponse = SessionInitResponses[keyof SessionInitResponses];

export type SessionForkData = {
  body?: {
    messageID?: string;
  };
  path: {
    sessionID: string;
  };
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/session/{sessionID}/fork';
};

export type SessionForkResponses = {
  /**
   * 200
   */
  200: Session;
};

export type SessionForkResponse = SessionForkResponses[keyof SessionForkResponses];

export type SessionAbortData = {
  body?: never;
  path: {
    sessionID: string;
  };
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/session/{sessionID}/abort';
};

export type SessionAbortErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
  /**
   * 找到了 Not
   */
  404: NotFoundError;
};

export type SessionAbortError = SessionAbortErrors[keyof SessionAbortErrors];

export type SessionAbortResponses = {
  /**
   * Aborted 会话
   */
  200: boolean;
};

export type SessionAbortResponse = SessionAbortResponses[keyof SessionAbortResponses];

export type SessionUnshareData = {
  body?: never;
  path: {
    sessionID: string;
  };
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/session/{sessionID}/share';
};

export type SessionUnshareErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
  /**
   * 找到了 Not
   */
  404: NotFoundError;
};

export type SessionUnshareError = SessionUnshareErrors[keyof SessionUnshareErrors];

export type SessionUnshareResponses = {
  /**
   * Successfully 非共享会话
   */
  200: Session;
};

export type SessionUnshareResponse = SessionUnshareResponses[keyof SessionUnshareResponses];

export type SessionShareData = {
  body?: never;
  path: {
    sessionID: string;
  };
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/session/{sessionID}/share';
};

export type SessionShareErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
  /**
   * 找到了 Not
   */
  404: NotFoundError;
};

export type SessionShareError = SessionShareErrors[keyof SessionShareErrors];

export type SessionShareResponses = {
  /**
   * Successfully 共享会话
   */
  200: Session;
};

export type SessionShareResponse = SessionShareResponses[keyof SessionShareResponses];

export type SessionDiffData = {
  body?: never;
  path: {
    sessionID: string;
  };
  query?: {
    directory?: string;
    workspace?: string;
    messageID?: string;
  };
  url: '/session/{sessionID}/diff';
};

export type SessionDiffResponses = {
  /**
   * Successfully 检索到差异
   */
  200: Array<FileDiff>;
};

export type SessionDiffResponse = SessionDiffResponses[keyof SessionDiffResponses];

export type SessionSummarizeData = {
  body?: {
    providerID: string;
    modelID: string;
    auto?: boolean;
  };
  path: {
    sessionID: string;
  };
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/session/{sessionID}/summarize';
};

export type SessionSummarizeErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
  /**
   * 找到了 Not
   */
  404: NotFoundError;
};

export type SessionSummarizeError = SessionSummarizeErrors[keyof SessionSummarizeErrors];

export type SessionSummarizeResponses = {
  /**
   * Summarized 会话
   */
  200: boolean;
};

export type SessionSummarizeResponse = SessionSummarizeResponses[keyof SessionSummarizeResponses];

export type SessionMessagesData = {
  body?: never;
  path: {
    sessionID: string;
  };
  query?: {
    directory?: string;
    workspace?: string;
    /**
     * Maximum 返回的消息数
     */
    limit?: number;
    before?: string;
  };
  url: '/session/{sessionID}/message';
};

export type SessionMessagesErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
  /**
   * 找到了 Not
   */
  404: NotFoundError;
};

export type SessionMessagesError = SessionMessagesErrors[keyof SessionMessagesErrors];

export type SessionMessagesResponses = {
  /**
   * List 条消息
   */
  200: Array<{
    info: Message;
    parts: Array<Part>;
  }>;
};

export type SessionMessagesResponse = SessionMessagesResponses[keyof SessionMessagesResponses];

export type SessionPromptData = {
  body?: {
    messageID?: string;
    model?: {
      providerID: string;
      modelID: string;
    };
    agent?: string;
    noReply?: boolean;
    /**
     * @deprecated 工具和权限已合并，您现在可以在会话本身上设置权限
     */
    tools?: {
      [key: string]: boolean;
    };
    format?: OutputFormat;
    system?: string;
    variant?: string;
    parts: Array<TextPartInput | FilePartInput | AgentPartInput | SubtaskPartInput>;
  };
  path: {
    sessionID: string;
  };
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/session/{sessionID}/message';
};

export type SessionPromptErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
  /**
   * 找到了 Not
   */
  404: NotFoundError;
};

export type SessionPromptError = SessionPromptErrors[keyof SessionPromptErrors];

export type SessionPromptResponses = {
  /**
   * Created 消息
   */
  200: {
    info: AssistantMessage;
    parts: Array<Part>;
  };
};

export type SessionPromptResponse = SessionPromptResponses[keyof SessionPromptResponses];

export type SessionDeleteMessageData = {
  body?: never;
  path: {
    sessionID: string;
    messageID: string;
  };
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/session/{sessionID}/message/{messageID}';
};

export type SessionDeleteMessageErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
  /**
   * 找到了 Not
   */
  404: NotFoundError;
};

export type SessionDeleteMessageError =
  SessionDeleteMessageErrors[keyof SessionDeleteMessageErrors];

export type SessionDeleteMessageResponses = {
  /**
   * Successfully 已删除消息
   */
  200: boolean;
};

export type SessionDeleteMessageResponse =
  SessionDeleteMessageResponses[keyof SessionDeleteMessageResponses];

export type SessionMessageData = {
  body?: never;
  path: {
    sessionID: string;
    messageID: string;
  };
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/session/{sessionID}/message/{messageID}';
};

export type SessionMessageErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
  /**
   * 找到了 Not
   */
  404: NotFoundError;
};

export type SessionMessageError = SessionMessageErrors[keyof SessionMessageErrors];

export type SessionMessageResponses = {
  /**
   * Message
   */
  200: {
    info: Message;
    parts: Array<Part>;
  };
};

export type SessionMessageResponse = SessionMessageResponses[keyof SessionMessageResponses];

export type PartDeleteData = {
  body?: never;
  path: {
    sessionID: string;
    messageID: string;
    partID: string;
  };
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/session/{sessionID}/message/{messageID}/part/{partID}';
};

export type PartDeleteErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
  /**
   * 找到了 Not
   */
  404: NotFoundError;
};

export type PartDeleteError = PartDeleteErrors[keyof PartDeleteErrors];

export type PartDeleteResponses = {
  /**
   * Successfully 删除部分
   */
  200: boolean;
};

export type PartDeleteResponse = PartDeleteResponses[keyof PartDeleteResponses];

export type PartUpdateData = {
  body?: Part;
  path: {
    sessionID: string;
    messageID: string;
    partID: string;
  };
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/session/{sessionID}/message/{messageID}/part/{partID}';
};

export type PartUpdateErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
  /**
   * 找到了 Not
   */
  404: NotFoundError;
};

export type PartUpdateError = PartUpdateErrors[keyof PartUpdateErrors];

export type PartUpdateResponses = {
  /**
   * Successfully 更新部分
   */
  200: Part;
};

export type PartUpdateResponse = PartUpdateResponses[keyof PartUpdateResponses];

export type SessionPromptAsyncData = {
  body?: {
    messageID?: string;
    model?: {
      providerID: string;
      modelID: string;
    };
    agent?: string;
    noReply?: boolean;
    /**
     * @deprecated 工具和权限已合并，您现在可以在会话本身上设置权限
     */
    tools?: {
      [key: string]: boolean;
    };
    format?: OutputFormat;
    system?: string;
    variant?: string;
    parts: Array<TextPartInput | FilePartInput | AgentPartInput | SubtaskPartInput>;
  };
  path: {
    sessionID: string;
  };
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/session/{sessionID}/prompt_async';
};

export type SessionPromptAsyncErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
  /**
   * 找到了 Not
   */
  404: NotFoundError;
};

export type SessionPromptAsyncError = SessionPromptAsyncErrors[keyof SessionPromptAsyncErrors];

export type SessionPromptAsyncResponses = {
  /**
   * Prompt 已接受
   */
  204: void;
};

export type SessionPromptAsyncResponse =
  SessionPromptAsyncResponses[keyof SessionPromptAsyncResponses];

export type SessionCommandData = {
  body?: {
    messageID?: string;
    agent?: string;
    model?: string;
    arguments: string;
    command: string;
    variant?: string;
    parts?: Array<{
      id?: string;
      type: 'file';
      mime: string;
      filename?: string;
      url: string;
      source?: FilePartSource;
    }>;
  };
  path: {
    sessionID: string;
  };
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/session/{sessionID}/command';
};

export type SessionCommandErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
  /**
   * 找到了 Not
   */
  404: NotFoundError;
};

export type SessionCommandError = SessionCommandErrors[keyof SessionCommandErrors];

export type SessionCommandResponses = {
  /**
   * Created 消息
   */
  200: {
    info: AssistantMessage;
    parts: Array<Part>;
  };
};

export type SessionCommandResponse = SessionCommandResponses[keyof SessionCommandResponses];

export type SessionShellData = {
  body?: {
    agent: string;
    model?: {
      providerID: string;
      modelID: string;
    };
    command: string;
  };
  path: {
    sessionID: string;
  };
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/session/{sessionID}/shell';
};

export type SessionShellErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
  /**
   * 找到了 Not
   */
  404: NotFoundError;
};

export type SessionShellError = SessionShellErrors[keyof SessionShellErrors];

export type SessionShellResponses = {
  /**
   * Created 消息
   */
  200: AssistantMessage;
};

export type SessionShellResponse = SessionShellResponses[keyof SessionShellResponses];

export type SessionRevertData = {
  body?: {
    messageID: string;
    partID?: string;
  };
  path: {
    sessionID: string;
  };
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/session/{sessionID}/revert';
};

export type SessionRevertErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
  /**
   * 找到了 Not
   */
  404: NotFoundError;
};

export type SessionRevertError = SessionRevertErrors[keyof SessionRevertErrors];

export type SessionRevertResponses = {
  /**
   * Updated 会话
   */
  200: Session;
};

export type SessionRevertResponse = SessionRevertResponses[keyof SessionRevertResponses];

export type SessionUnrevertData = {
  body?: never;
  path: {
    sessionID: string;
  };
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/session/{sessionID}/unrevert';
};

export type SessionUnrevertErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
  /**
   * 找到了 Not
   */
  404: NotFoundError;
};

export type SessionUnrevertError = SessionUnrevertErrors[keyof SessionUnrevertErrors];

export type SessionUnrevertResponses = {
  /**
   * Updated 会话
   */
  200: Session;
};

export type SessionUnrevertResponse = SessionUnrevertResponses[keyof SessionUnrevertResponses];

export type PermissionRespondData = {
  body?: {
    response: 'once' | 'always' | 'reject';
  };
  path: {
    sessionID: string;
    permissionID: string;
  };
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/session/{sessionID}/permissions/{permissionID}';
};

export type PermissionRespondErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
  /**
   * 找到了 Not
   */
  404: NotFoundError;
};

export type PermissionRespondError = PermissionRespondErrors[keyof PermissionRespondErrors];

export type PermissionRespondResponses = {
  /**
   * Permission 处理成功
   */
  200: boolean;
};

export type PermissionRespondResponse =
  PermissionRespondResponses[keyof PermissionRespondResponses];

export type PermissionReplyData = {
  body?: {
    reply: 'once' | 'always' | 'reject';
    message?: string;
  };
  path: {
    requestID: string;
  };
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/permission/{requestID}/reply';
};

export type PermissionReplyErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
  /**
   * 找到了 Not
   */
  404: NotFoundError;
};

export type PermissionReplyError = PermissionReplyErrors[keyof PermissionReplyErrors];

export type PermissionReplyResponses = {
  /**
   * Permission 处理成功
   */
  200: boolean;
};

export type PermissionReplyResponse = PermissionReplyResponses[keyof PermissionReplyResponses];

export type PermissionListData = {
  body?: never;
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/permission';
};

export type PermissionListResponses = {
  /**
   * List 待处理权限
   */
  200: Array<PermissionRequest>;
};

export type PermissionListResponse = PermissionListResponses[keyof PermissionListResponses];

export type QuestionListData = {
  body?: never;
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/question';
};

export type QuestionListResponses = {
  /**
   * List 待决问题
   */
  200: Array<QuestionRequest>;
};

export type QuestionListResponse = QuestionListResponses[keyof QuestionListResponses];

export type QuestionReplyData = {
  body?: {
    /**
     * User 按问题顺序回答（每个答案是选定标签的数组）
     */
    answers: Array<QuestionAnswer>;
  };
  path: {
    requestID: string;
  };
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/question/{requestID}/reply';
};

export type QuestionReplyErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
  /**
   * 找到了 Not
   */
  404: NotFoundError;
};

export type QuestionReplyError = QuestionReplyErrors[keyof QuestionReplyErrors];

export type QuestionReplyResponses = {
  /**
   * Question 接听成功
   */
  200: boolean;
};

export type QuestionReplyResponse = QuestionReplyResponses[keyof QuestionReplyResponses];

export type QuestionRejectData = {
  body?: never;
  path: {
    requestID: string;
  };
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/question/{requestID}/reject';
};

export type QuestionRejectErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
  /**
   * 找到了 Not
   */
  404: NotFoundError;
};

export type QuestionRejectError = QuestionRejectErrors[keyof QuestionRejectErrors];

export type QuestionRejectResponses = {
  /**
   * Question 拒绝成功
   */
  200: boolean;
};

export type QuestionRejectResponse = QuestionRejectResponses[keyof QuestionRejectResponses];

export type ProviderListData = {
  body?: never;
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/provider';
};

export type ProviderListResponses = {
  /**
   * List 提供商
   */
  200: {
    all: Array<{
      api?: string;
      name: string;
      env: Array<string>;
      id: string;
      npm?: string;
      models: {
        [key: string]: {
          id: string;
          name: string;
          family?: string;
          release_date: string;
          attachment: boolean;
          reasoning: boolean;
          temperature: boolean;
          tool_call: boolean;
          interleaved?:
            | true
            | {
                field: 'reasoning_content' | 'reasoning_details';
              };
          cost?: {
            input: number;
            output: number;
            cache_read?: number;
            cache_write?: number;
            context_over_200k?: {
              input: number;
              output: number;
              cache_read?: number;
              cache_write?: number;
            };
          };
          limit: {
            context: number;
            input?: number;
            output: number;
          };
          modalities?: {
            input: Array<'text' | 'audio' | 'image' | 'video' | 'pdf'>;
            output: Array<'text' | 'audio' | 'image' | 'video' | 'pdf'>;
          };
          experimental?: boolean;
          status?: 'alpha' | 'beta' | 'deprecated';
          options: {
            [key: string]: unknown;
          };
          headers?: {
            [key: string]: string;
          };
          provider?: {
            npm?: string;
            api?: string;
          };
          variants?: {
            [key: string]: {
              [key: string]: unknown;
            };
          };
        };
      };
    }>;
    default: {
      [key: string]: string;
    };
    connected: Array<string>;
  };
};

export type ProviderListResponse = ProviderListResponses[keyof ProviderListResponses];

export type ProviderAuthData = {
  body?: never;
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/provider/auth';
};

export type ProviderAuthResponses = {
  /**
   * Provider 验证方法
   */
  200: {
    [key: string]: Array<ProviderAuthMethod>;
  };
};

export type ProviderAuthResponse = ProviderAuthResponses[keyof ProviderAuthResponses];

export type ProviderOauthAuthorizeData = {
  body?: {
    /**
     * Auth 方法索引
     */
    method: number;
    /**
     * Prompt 输入
     */
    inputs?: {
      [key: string]: string;
    };
  };
  path: {
    /**
     * Provider ID
     */
    providerID: string;
  };
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/provider/{providerID}/oauth/authorize';
};

export type ProviderOauthAuthorizeErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
};

export type ProviderOauthAuthorizeError =
  ProviderOauthAuthorizeErrors[keyof ProviderOauthAuthorizeErrors];

export type ProviderOauthAuthorizeResponses = {
  /**
   * Authorization URL 和方法
   */
  200: ProviderAuthAuthorization;
};

export type ProviderOauthAuthorizeResponse =
  ProviderOauthAuthorizeResponses[keyof ProviderOauthAuthorizeResponses];

export type ProviderOauthCallbackData = {
  body?: {
    /**
     * Auth 方法索引
     */
    method: number;
    /**
     * OAuth 授权码
     */
    code?: string;
  };
  path: {
    /**
     * Provider ID
     */
    providerID: string;
  };
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/provider/{providerID}/oauth/callback';
};

export type ProviderOauthCallbackErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
};

export type ProviderOauthCallbackError =
  ProviderOauthCallbackErrors[keyof ProviderOauthCallbackErrors];

export type ProviderOauthCallbackResponses = {
  /**
   * OAuth 回调处理成功
   */
  200: boolean;
};

export type ProviderOauthCallbackResponse =
  ProviderOauthCallbackResponses[keyof ProviderOauthCallbackResponses];

export type FindTextData = {
  body?: never;
  path?: never;
  query: {
    directory?: string;
    workspace?: string;
    pattern: string;
  };
  url: '/find';
};

export type FindTextResponses = {
  /**
   * Matches
   */
  200: Array<{
    path: {
      text: string;
    };
    lines: {
      text: string;
    };
    line_number: number;
    absolute_offset: number;
    submatches: Array<{
      match: {
        text: string;
      };
      start: number;
      end: number;
    }>;
  }>;
};

export type FindTextResponse = FindTextResponses[keyof FindTextResponses];

export type FindFilesData = {
  body?: never;
  path?: never;
  query: {
    directory?: string;
    workspace?: string;
    query: string;
    dirs?: 'true' | 'false';
    type?: 'file' | 'directory';
    limit?: number;
  };
  url: '/find/file';
};

export type FindFilesResponses = {
  /**
   * File 路径
   */
  200: Array<string>;
};

export type FindFilesResponse = FindFilesResponses[keyof FindFilesResponses];

export type FindSymbolsData = {
  body?: never;
  path?: never;
  query: {
    directory?: string;
    workspace?: string;
    query: string;
  };
  url: '/find/symbol';
};

export type FindSymbolsResponses = {
  /**
   * Symbols
   */
  200: Array<Symbol>;
};

export type FindSymbolsResponse = FindSymbolsResponses[keyof FindSymbolsResponses];

export type FileListData = {
  body?: never;
  path?: never;
  query: {
    directory?: string;
    workspace?: string;
    path: string;
  };
  url: '/file';
};

export type FileListResponses = {
  /**
   * Files 和目录
   */
  200: Array<FileNode>;
};

export type FileListResponse = FileListResponses[keyof FileListResponses];

export type FileReadData = {
  body?: never;
  path?: never;
  query: {
    directory?: string;
    workspace?: string;
    path: string;
  };
  url: '/file/content';
};

export type FileReadResponses = {
  /**
   * File 内容
   */
  200: FileContent;
};

export type FileReadResponse = FileReadResponses[keyof FileReadResponses];

export type FileStatusData = {
  body?: never;
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/file/status';
};

export type FileStatusResponses = {
  /**
   * File 状态
   */
  200: Array<File>;
};

export type FileStatusResponse = FileStatusResponses[keyof FileStatusResponses];

export type EventSubscribeData = {
  body?: never;
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/event';
};

export type EventSubscribeResponses = {
  /**
   * Event 流
   */
  200: Event;
};

export type EventSubscribeResponse = EventSubscribeResponses[keyof EventSubscribeResponses];

export type McpStatusData = {
  body?: never;
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/mcp';
};

export type McpStatusResponses = {
  /**
   * MCP 服务器状态
   */
  200: {
    [key: string]: McpStatus;
  };
};

export type McpStatusResponse = McpStatusResponses[keyof McpStatusResponses];

export type McpAddData = {
  body?: {
    name: string;
    config: McpLocalConfig | McpRemoteConfig;
  };
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/mcp';
};

export type McpAddErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
};

export type McpAddError = McpAddErrors[keyof McpAddErrors];

export type McpAddResponses = {
  /**
   * MCP 服务器添加成功
   */
  200: {
    [key: string]: McpStatus;
  };
};

export type McpAddResponse = McpAddResponses[keyof McpAddResponses];

export type McpAuthRemoveData = {
  body?: never;
  path: {
    name: string;
  };
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/mcp/{name}/auth';
};

export type McpAuthRemoveErrors = {
  /**
   * 找到了 Not
   */
  404: NotFoundError;
};

export type McpAuthRemoveError = McpAuthRemoveErrors[keyof McpAuthRemoveErrors];

export type McpAuthRemoveResponses = {
  /**
   * OAuth 凭据已删除
   */
  200: {
    success: true;
  };
};

export type McpAuthRemoveResponse = McpAuthRemoveResponses[keyof McpAuthRemoveResponses];

export type McpAuthStartData = {
  body?: never;
  path: {
    name: string;
  };
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/mcp/{name}/auth';
};

export type McpAuthStartErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
  /**
   * 找到了 Not
   */
  404: NotFoundError;
};

export type McpAuthStartError = McpAuthStartErrors[keyof McpAuthStartErrors];

export type McpAuthStartResponses = {
  /**
   * OAuth 流程已开始
   */
  200: {
    /**
     * URL 在浏览器中打开进行授权
     */
    authorizationUrl: string;
  };
};

export type McpAuthStartResponse = McpAuthStartResponses[keyof McpAuthStartResponses];

export type McpAuthCallbackData = {
  body?: {
    /**
     * OAuth 回调中的 Authorization 代码
     */
    code: string;
  };
  path: {
    name: string;
  };
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/mcp/{name}/auth/callback';
};

export type McpAuthCallbackErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
  /**
   * 找到了 Not
   */
  404: NotFoundError;
};

export type McpAuthCallbackError = McpAuthCallbackErrors[keyof McpAuthCallbackErrors];

export type McpAuthCallbackResponses = {
  /**
   * OAuth 认证完成
   */
  200: McpStatus;
};

export type McpAuthCallbackResponse = McpAuthCallbackResponses[keyof McpAuthCallbackResponses];

export type McpAuthAuthenticateData = {
  body?: never;
  path: {
    name: string;
  };
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/mcp/{name}/auth/authenticate';
};

export type McpAuthAuthenticateErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
  /**
   * 找到了 Not
   */
  404: NotFoundError;
};

export type McpAuthAuthenticateError = McpAuthAuthenticateErrors[keyof McpAuthAuthenticateErrors];

export type McpAuthAuthenticateResponses = {
  /**
   * OAuth 认证完成
   */
  200: McpStatus;
};

export type McpAuthAuthenticateResponse =
  McpAuthAuthenticateResponses[keyof McpAuthAuthenticateResponses];

export type McpConnectData = {
  body?: never;
  path: {
    name: string;
  };
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/mcp/{name}/connect';
};

export type McpConnectResponses = {
  /**
   * MCP 服务器连接成功
   */
  200: boolean;
};

export type McpConnectResponse = McpConnectResponses[keyof McpConnectResponses];

export type McpDisconnectData = {
  body?: never;
  path: {
    name: string;
  };
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/mcp/{name}/disconnect';
};

export type McpDisconnectResponses = {
  /**
   * MCP 服务器断开连接成功
   */
  200: boolean;
};

export type McpDisconnectResponse = McpDisconnectResponses[keyof McpDisconnectResponses];

export type TuiAppendPromptData = {
  body?: {
    text: string;
  };
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/tui/append-prompt';
};

export type TuiAppendPromptErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
};

export type TuiAppendPromptError = TuiAppendPromptErrors[keyof TuiAppendPromptErrors];

export type TuiAppendPromptResponses = {
  /**
   * Prompt 处理成功
   */
  200: boolean;
};

export type TuiAppendPromptResponse = TuiAppendPromptResponses[keyof TuiAppendPromptResponses];

export type TuiOpenHelpData = {
  body?: never;
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/tui/open-help';
};

export type TuiOpenHelpResponses = {
  /**
   * Help 对话框已成功打开
   */
  200: boolean;
};

export type TuiOpenHelpResponse = TuiOpenHelpResponses[keyof TuiOpenHelpResponses];

export type TuiOpenSessionsData = {
  body?: never;
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/tui/open-sessions';
};

export type TuiOpenSessionsResponses = {
  /**
   * Session 对话框已成功打开
   */
  200: boolean;
};

export type TuiOpenSessionsResponse = TuiOpenSessionsResponses[keyof TuiOpenSessionsResponses];

export type TuiOpenThemesData = {
  body?: never;
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/tui/open-themes';
};

export type TuiOpenThemesResponses = {
  /**
   * Theme 对话框已成功打开
   */
  200: boolean;
};

export type TuiOpenThemesResponse = TuiOpenThemesResponses[keyof TuiOpenThemesResponses];

export type TuiOpenModelsData = {
  body?: never;
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/tui/open-models';
};

export type TuiOpenModelsResponses = {
  /**
   * Model 对话框已成功打开
   */
  200: boolean;
};

export type TuiOpenModelsResponse = TuiOpenModelsResponses[keyof TuiOpenModelsResponses];

export type TuiSubmitPromptData = {
  body?: never;
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/tui/submit-prompt';
};

export type TuiSubmitPromptResponses = {
  /**
   * Prompt 提交成功
   */
  200: boolean;
};

export type TuiSubmitPromptResponse = TuiSubmitPromptResponses[keyof TuiSubmitPromptResponses];

export type TuiClearPromptData = {
  body?: never;
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/tui/clear-prompt';
};

export type TuiClearPromptResponses = {
  /**
   * Prompt 清除成功
   */
  200: boolean;
};

export type TuiClearPromptResponse = TuiClearPromptResponses[keyof TuiClearPromptResponses];

export type TuiExecuteCommandData = {
  body?: {
    command: string;
  };
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/tui/execute-command';
};

export type TuiExecuteCommandErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
};

export type TuiExecuteCommandError = TuiExecuteCommandErrors[keyof TuiExecuteCommandErrors];

export type TuiExecuteCommandResponses = {
  /**
   * Command 执行成功
   */
  200: boolean;
};

export type TuiExecuteCommandResponse =
  TuiExecuteCommandResponses[keyof TuiExecuteCommandResponses];

export type TuiShowToastData = {
  body?: {
    title?: string;
    message: string;
    variant: 'info' | 'success' | 'warning' | 'error';
    /**
     * Duration 以毫秒为单位
     */
    duration?: number;
  };
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/tui/show-toast';
};

export type TuiShowToastResponses = {
  /**
   * Toast 通知已成功显示
   */
  200: boolean;
};

export type TuiShowToastResponse = TuiShowToastResponses[keyof TuiShowToastResponses];

export type TuiPublishData = {
  body?: EventTuiPromptAppend | EventTuiCommandExecute | EventTuiToastShow | EventTuiSessionSelect;
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/tui/publish';
};

export type TuiPublishErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
};

export type TuiPublishError = TuiPublishErrors[keyof TuiPublishErrors];

export type TuiPublishResponses = {
  /**
   * Event 发布成功
   */
  200: boolean;
};

export type TuiPublishResponse = TuiPublishResponses[keyof TuiPublishResponses];

export type TuiSelectSessionData = {
  body?: {
    /**
     * Session ID 导航至
     */
    sessionID: string;
  };
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/tui/select-session';
};

export type TuiSelectSessionErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
  /**
   * 找到了 Not
   */
  404: NotFoundError;
};

export type TuiSelectSessionError = TuiSelectSessionErrors[keyof TuiSelectSessionErrors];

export type TuiSelectSessionResponses = {
  /**
   * Session 选择成功
   */
  200: boolean;
};

export type TuiSelectSessionResponse = TuiSelectSessionResponses[keyof TuiSelectSessionResponses];

export type TuiControlNextData = {
  body?: never;
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/tui/control/next';
};

export type TuiControlNextResponses = {
  /**
   * Next TUI 请求
   */
  200: {
    path: string;
    body: unknown;
  };
};

export type TuiControlNextResponse = TuiControlNextResponses[keyof TuiControlNextResponses];

export type TuiControlResponseData = {
  body?: unknown;
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/tui/control/response';
};

export type TuiControlResponseResponses = {
  /**
   * Response 提交成功
   */
  200: boolean;
};

export type TuiControlResponseResponse =
  TuiControlResponseResponses[keyof TuiControlResponseResponses];

export type InstanceDisposeData = {
  body?: never;
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/instance/dispose';
};

export type InstanceDisposeResponses = {
  /**
   * Instance 已处置
   */
  200: boolean;
};

export type InstanceDisposeResponse = InstanceDisposeResponses[keyof InstanceDisposeResponses];

export type PathGetData = {
  body?: never;
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/path';
};

export type PathGetResponses = {
  /**
   * Path
   */
  200: Path;
};

export type PathGetResponse = PathGetResponses[keyof PathGetResponses];

export type VcsGetData = {
  body?: never;
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/vcs';
};

export type VcsGetResponses = {
  /**
   * VCS 信息
   */
  200: VcsInfo;
};

export type VcsGetResponse = VcsGetResponses[keyof VcsGetResponses];

export type CommandListData = {
  body?: never;
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/command';
};

export type CommandListResponses = {
  /**
   * List 命令
   */
  200: Array<Command>;
};

export type CommandListResponse = CommandListResponses[keyof CommandListResponses];

export type AppLogData = {
  body?: {
    /**
     * Service 日志条目的名称
     */
    service: string;
    /**
     * Log 等级
     */
    level: 'debug' | 'info' | 'error' | 'warn';
    /**
     * Log 消息
     */
    message: string;
    /**
     * Additional 日志条目的元数据
     */
    extra?: {
      [key: string]: unknown;
    };
  };
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/log';
};

export type AppLogErrors = {
  /**
   * Bad 请求
   */
  400: BadRequestError;
};

export type AppLogError = AppLogErrors[keyof AppLogErrors];

export type AppLogResponses = {
  /**
   * Log 条目写入成功
   */
  200: boolean;
};

export type AppLogResponse = AppLogResponses[keyof AppLogResponses];

export type AppAgentsData = {
  body?: never;
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/agent';
};

export type AppAgentsResponses = {
  /**
   * List 代理
   */
  200: Array<Agent>;
};

export type AppAgentsResponse = AppAgentsResponses[keyof AppAgentsResponses];

export type AppSkillsData = {
  body?: never;
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/skill';
};

export type AppSkillsResponses = {
  /**
   * List 技能
   */
  200: Array<{
    name: string;
    description: string;
    location: string;
    content: string;
  }>;
};

export type AppSkillsResponse = AppSkillsResponses[keyof AppSkillsResponses];

export type LspStatusData = {
  body?: never;
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/lsp';
};

export type LspStatusResponses = {
  /**
   * LSP 服务器状态
   */
  200: Array<LspStatus>;
};

export type LspStatusResponse = LspStatusResponses[keyof LspStatusResponses];

export type FormatterStatusData = {
  body?: never;
  path?: never;
  query?: {
    directory?: string;
    workspace?: string;
  };
  url: '/formatter';
};

export type FormatterStatusResponses = {
  /**
   * Formatter 状态
   */
  200: Array<FormatterStatus>;
};

export type FormatterStatusResponse = FormatterStatusResponses[keyof FormatterStatusResponses];
