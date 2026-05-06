// This 文件由 @hey-api/openapi-ts 自动生成

import { client } from './client.gen.js';
import {
  buildClientParams,
  type Client,
  type Options as Options2,
  type TDataShape,
} from './client/index.js';
import type {
  AgentPartInput,
  AppAgentsResponses,
  AppLogErrors,
  AppLogResponses,
  AppSkillsResponses,
  Auth as Auth3,
  AuthRemoveErrors,
  AuthRemoveResponses,
  AuthSetErrors,
  AuthSetResponses,
  CommandListResponses,
  Config as Config3,
  ConfigGetResponses,
  ConfigProvidersResponses,
  ConfigUpdateErrors,
  ConfigUpdateResponses,
  EventSubscribeResponses,
  EventTuiCommandExecute,
  EventTuiPromptAppend,
  EventTuiSessionSelect,
  EventTuiToastShow,
  ExperimentalResourceListResponses,
  ExperimentalSessionListResponses,
  ExperimentalWorkspaceCreateErrors,
  ExperimentalWorkspaceCreateResponses,
  ExperimentalWorkspaceListResponses,
  ExperimentalWorkspaceRemoveErrors,
  ExperimentalWorkspaceRemoveResponses,
  FileListResponses,
  FilePartInput,
  FilePartSource,
  FileReadResponses,
  FileStatusResponses,
  FindFilesResponses,
  FindSymbolsResponses,
  FindTextResponses,
  FormatterStatusResponses,
  GlobalConfigGetResponses,
  GlobalConfigUpdateErrors,
  GlobalConfigUpdateResponses,
  GlobalDisposeResponses,
  GlobalEventResponses,
  GlobalHealthResponses,
  InstanceDisposeResponses,
  LspStatusResponses,
  McpAddErrors,
  McpAddResponses,
  McpAuthAuthenticateErrors,
  McpAuthAuthenticateResponses,
  McpAuthCallbackErrors,
  McpAuthCallbackResponses,
  McpAuthRemoveErrors,
  McpAuthRemoveResponses,
  McpAuthStartErrors,
  McpAuthStartResponses,
  McpConnectResponses,
  McpDisconnectResponses,
  McpLocalConfig,
  McpRemoteConfig,
  McpStatusResponses,
  OutputFormat,
  Part as Part2,
  PartDeleteErrors,
  PartDeleteResponses,
  PartUpdateErrors,
  PartUpdateResponses,
  PathGetResponses,
  PermissionListResponses,
  PermissionReplyErrors,
  PermissionReplyResponses,
  PermissionRespondErrors,
  PermissionRespondResponses,
  PermissionRuleset,
  ProjectCurrentResponses,
  ProjectInitGitResponses,
  ProjectListResponses,
  ProjectUpdateErrors,
  ProjectUpdateResponses,
  ProviderAuthResponses,
  ProviderListResponses,
  ProviderOauthAuthorizeErrors,
  ProviderOauthAuthorizeResponses,
  ProviderOauthCallbackErrors,
  ProviderOauthCallbackResponses,
  PtyConnectErrors,
  PtyConnectResponses,
  PtyCreateErrors,
  PtyCreateResponses,
  PtyGetErrors,
  PtyGetResponses,
  PtyListResponses,
  PtyRemoveErrors,
  PtyRemoveResponses,
  PtyUpdateErrors,
  PtyUpdateResponses,
  QuestionAnswer,
  QuestionListResponses,
  QuestionRejectErrors,
  QuestionRejectResponses,
  QuestionReplyErrors,
  QuestionReplyResponses,
  SessionAbortErrors,
  SessionAbortResponses,
  SessionChildrenErrors,
  SessionChildrenResponses,
  SessionCommandErrors,
  SessionCommandResponses,
  SessionCreateErrors,
  SessionCreateResponses,
  SessionDeleteErrors,
  SessionDeleteMessageErrors,
  SessionDeleteMessageResponses,
  SessionDeleteResponses,
  SessionDiffResponses,
  SessionForkResponses,
  SessionGetErrors,
  SessionGetResponses,
  SessionInitErrors,
  SessionInitResponses,
  SessionListResponses,
  SessionMessageErrors,
  SessionMessageResponses,
  SessionMessagesErrors,
  SessionMessagesResponses,
  SessionPromptAsyncErrors,
  SessionPromptAsyncResponses,
  SessionPromptErrors,
  SessionPromptResponses,
  SessionRevertErrors,
  SessionRevertResponses,
  SessionShareErrors,
  SessionShareResponses,
  SessionShellErrors,
  SessionShellResponses,
  SessionStatusErrors,
  SessionStatusResponses,
  SessionSummarizeErrors,
  SessionSummarizeResponses,
  SessionTodoErrors,
  SessionTodoResponses,
  SessionUnrevertErrors,
  SessionUnrevertResponses,
  SessionUnshareErrors,
  SessionUnshareResponses,
  SessionUpdateErrors,
  SessionUpdateResponses,
  SubtaskPartInput,
  TextPartInput,
  ToolIdsErrors,
  ToolIdsResponses,
  ToolListErrors,
  ToolListResponses,
  TuiAppendPromptErrors,
  TuiAppendPromptResponses,
  TuiClearPromptResponses,
  TuiControlNextResponses,
  TuiControlResponseResponses,
  TuiExecuteCommandErrors,
  TuiExecuteCommandResponses,
  TuiOpenHelpResponses,
  TuiOpenModelsResponses,
  TuiOpenSessionsResponses,
  TuiOpenThemesResponses,
  TuiPublishErrors,
  TuiPublishResponses,
  TuiSelectSessionErrors,
  TuiSelectSessionResponses,
  TuiShowToastResponses,
  TuiSubmitPromptResponses,
  VcsGetResponses,
  WorktreeCreateErrors,
  WorktreeCreateInput,
  WorktreeCreateResponses,
  WorktreeListResponses,
  WorktreeRemoveErrors,
  WorktreeRemoveInput,
  WorktreeRemoveResponses,
  WorktreeResetErrors,
  WorktreeResetInput,
  WorktreeResetResponses,
} from './types.gen.js';

export type Options<
  TData extends TDataShape = TDataShape,
  ThrowOnError extends boolean = boolean,
> = Options2<TData, ThrowOnError> & {
  /**
   * You 可以提供
   * `createClient()` 返回的客户端实例，而不是单个选项。如果您想实现自定义客户端，This 也可能很有用。
   *
   */
  client?: Client;
  /**
   * You 可以通过
   * `meta` 对象传递任意值。 This 可用于访问未定义为 SDK 函数一部分的值。
   */
  meta?: Record<string, unknown>;
};

class HeyApiClient {
  protected client: Client;

  constructor(args?: { client?: Client }) {
    this.client = args?.client ?? client;
  }
}

class HeyApiRegistry<T> {
  private readonly defaultKey = 'default';

  private readonly instances: Map<string, T> = new Map();

  get(key?: string): T {
    const instance = this.instances.get(key ?? this.defaultKey);
    if (!instance) {
      throw new Error(
        `No SDK client found. Create one with "new OpencodeClient()" to fix this error.`,
      );
    }
    return instance;
  }

  set(value: T, key?: string): void {
    this.instances.set(key ?? this.defaultKey, value);
  }
}

export class Config extends HeyApiClient {
  /**
   * Get 全局配置
   *
   * Retrieve 当前全局 OpenCode
   配置设置和首选项。
   */
  public get<ThrowOnError extends boolean = false>(options?: Options<never, ThrowOnError>) {
    return (options?.client ?? this.client).get<GlobalConfigGetResponses, unknown, ThrowOnError>({
      url: '/global/config',
      ...options,
    });
  }

  /**
   * Update 全局配置
   *
   * Update 全局 OpenCode
   配置设置和首选项。
   */
  public update<ThrowOnError extends boolean = false>(
    parameters?: {
      config?: Config3;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams([parameters], [{ args: [{ key: 'config', map: 'body' }] }]);
    return (options?.client ?? this.client).patch<
      GlobalConfigUpdateResponses,
      GlobalConfigUpdateErrors,
      ThrowOnError
    >({
      url: '/global/config',
      ...options,
      ...params,
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
        ...params.headers,
      },
    });
  }
}

export class Global extends HeyApiClient {
  /**
   * Get health
   *
   * Get 有关 OpenC
   ode 服务器的运行状况
   信息。
   */
  public health<ThrowOnError extends boolean = false>(options?: Options<never, ThrowOnError>) {
    return (options?.client ?? this.client).get<GlobalHealthResponses, unknown, ThrowOnError>({
      url: '/global/health',
      ...options,
    });
  }

  /**
   * Get 全局事件
   *
   * Subscribe 到使用服务器发
   送事件的 OpenCod
   e 系统中的全局事件。
   */
  public event<ThrowOnError extends boolean = false>(options?: Options<never, ThrowOnError>) {
    return (options?.client ?? this.client).sse.get<GlobalEventResponses, unknown, ThrowOnError>({
      url: '/global/event',
      ...options,
    });
  }

  /**
   * Dispose 实例
   *
   * Clean 启动并处置所有
   OpenCode
   实例，释放所有资源。
   */
  public dispose<ThrowOnError extends boolean = false>(options?: Options<never, ThrowOnError>) {
    return (options?.client ?? this.client).post<GlobalDisposeResponses, unknown, ThrowOnError>({
      url: '/global/dispose',
      ...options,
    });
  }

  private _config?: Config;
  get config(): Config {
    return (this._config ??= new Config({ client: this.client }));
  }
}

export class Auth extends HeyApiClient {
  /**
   * Remove 身份验证凭
   *
   * 据 Remove 身份验证凭据
   */
  public remove<ThrowOnError extends boolean = false>(
    parameters: {
      providerID: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams([parameters], [{ args: [{ in: 'path', key: 'providerID' }] }]);
    return (options?.client ?? this.client).delete<
      AuthRemoveResponses,
      AuthRemoveErrors,
      ThrowOnError
    >({
      url: '/auth/{providerID}',
      ...options,
      ...params,
    });
  }

  /**
   * Set 身份验证凭据
   *
   * Set 身份验证凭据
   */
  public set<ThrowOnError extends boolean = false>(
    parameters: {
      providerID: string;
      auth?: Auth3;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'path', key: 'providerID' },
            { key: 'auth', map: 'body' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).put<AuthSetResponses, AuthSetErrors, ThrowOnError>({
      url: '/auth/{providerID}',
      ...options,
      ...params,
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
        ...params.headers,
      },
    });
  }
}

export class Project extends HeyApiClient {
  /**
   * List 所有项目
   *
   * Get 已使用 OpenCode
   打开的项目列表。
   */
  public list<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).get<ProjectListResponses, unknown, ThrowOnError>({
      url: '/project',
      ...options,
      ...params,
    });
  }

  /**
   * Get 当前项目
   *
   * Retrieve OpenCode
   正在使用的当前活动项目。
   */
  public current<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).get<ProjectCurrentResponses, unknown, ThrowOnError>({
      url: '/project/current',
      ...options,
      ...params,
    });
  }

  /**
   * Initialize
   *
   * git 存储库 Create 当前项目的 git
   存储库并返回刷新的项目信
   息。
   */
  public initGit<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).post<ProjectInitGitResponses, unknown, ThrowOnError>({
      url: '/project/git/init',
      ...options,
      ...params,
    });
  }

  /**
   * Update 项目
   *
   * Update 项目属性，例如
   名称、图标和命令。
   */
  public update<ThrowOnError extends boolean = false>(
    parameters: {
      projectID: string;
      directory?: string;
      workspace?: string;
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
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'path', key: 'projectID' },
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
            { in: 'body', key: 'name' },
            { in: 'body', key: 'icon' },
            { in: 'body', key: 'commands' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).patch<
      ProjectUpdateResponses,
      ProjectUpdateErrors,
      ThrowOnError
    >({
      url: '/project/{projectID}',
      ...options,
      ...params,
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
        ...params.headers,
      },
    });
  }
}

export class Pty extends HeyApiClient {
  /**
   * List PTY 会话
   *
   * Get 由 OpenCode
   管理的所有活动伪终端
   (PTY) 会话的列表。
   */
  public list<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).get<PtyListResponses, unknown, ThrowOnError>({
      url: '/pty',
      ...options,
      ...params,
    });
  }

  /**
   * Create PTY
   *
   * 会话 Create 一个新的伪终端
   (PTY) 会话，用于运
   行 shell 命令和进程。
   */
  public create<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
      command?: string;
      args?: Array<string>;
      cwd?: string;
      title?: string;
      env?: {
        [key: string]: string;
      };
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
            { in: 'body', key: 'command' },
            { in: 'body', key: 'args' },
            { in: 'body', key: 'cwd' },
            { in: 'body', key: 'title' },
            { in: 'body', key: 'env' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).post<PtyCreateResponses, PtyCreateErrors, ThrowOnError>(
      {
        url: '/pty',
        ...options,
        ...params,
        headers: {
          'Content-Type': 'application/json',
          ...options?.headers,
          ...params.headers,
        },
      },
    );
  }

  /**
   * Remove PTY
   *
   * 会话 Remove 并终止特定的伪终
   端 (PTY) 会话。
   */
  public remove<ThrowOnError extends boolean = false>(
    parameters: {
      ptyID: string;
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'path', key: 'ptyID' },
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).delete<
      PtyRemoveResponses,
      PtyRemoveErrors,
      ThrowOnError
    >({
      url: '/pty/{ptyID}',
      ...options,
      ...params,
    });
  }

  /**
   * Get PTY 会话
   *
   * Retrieve 有关特定伪终
   端 (PTY)
   会话的详细信息。
   */
  public get<ThrowOnError extends boolean = false>(
    parameters: {
      ptyID: string;
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'path', key: 'ptyID' },
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).get<PtyGetResponses, PtyGetErrors, ThrowOnError>({
      url: '/pty/{ptyID}',
      ...options,
      ...params,
    });
  }

  /**
   * Update PTY
   *
   * 会话 Update 现有伪终端
   (PTY) 会话的属性。
   */
  public update<ThrowOnError extends boolean = false>(
    parameters: {
      ptyID: string;
      directory?: string;
      workspace?: string;
      title?: string;
      size?: {
        rows: number;
        cols: number;
      };
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'path', key: 'ptyID' },
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
            { in: 'body', key: 'title' },
            { in: 'body', key: 'size' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).put<PtyUpdateResponses, PtyUpdateErrors, ThrowOnError>({
      url: '/pty/{ptyID}',
      ...options,
      ...params,
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
        ...params.headers,
      },
    });
  }

  /**
   * Connect 到
   *
   * PTY 会话 Establish 一个
   WebSocket
   连接，用于与伪终端 (PTY) 会话实时交互。
   */
  public connect<ThrowOnError extends boolean = false>(
    parameters: {
      ptyID: string;
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'path', key: 'ptyID' },
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).get<
      PtyConnectResponses,
      PtyConnectErrors,
      ThrowOnError
    >({
      url: '/pty/{ptyID}/connect',
      ...options,
      ...params,
    });
  }
}

export class Config2 extends HeyApiClient {
  /**
   * Get 配置 Retri
   *
   * eve 当前 OpenCode
   配置设置和首选项。
   */
  public get<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).get<ConfigGetResponses, unknown, ThrowOnError>({
      url: '/config',
      ...options,
      ...params,
    });
  }

  /**
   * Update 配置
   *
   * Update OpenCode
   配置设置和首选项。
   */
  public update<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
      config?: Config3;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
            { key: 'config', map: 'body' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).patch<
      ConfigUpdateResponses,
      ConfigUpdateErrors,
      ThrowOnError
    >({
      url: '/config',
      ...options,
      ...params,
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
        ...params.headers,
      },
    });
  }

  /**
   * List 配置提供程序
   *
   * Get 所有配置的 AI
   提供程序及其默认模型的列
   表。
   */
  public providers<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).get<ConfigProvidersResponses, unknown, ThrowOnError>({
      url: '/config/providers',
      ...options,
      ...params,
    });
  }
}

export class Tool extends HeyApiClient {
  /**
   * List 工具 IDs
   *
   * Get 所有可用工具
   IDs 的列表，包括内置
   工具和动态注册工具。
   */
  public ids<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).get<ToolIdsResponses, ToolIdsErrors, ThrowOnError>({
      url: '/experimental/tool/ids',
      ...options,
      ...params,
    });
  }

  /**
   * List 工具 Get
   *
   * 可用工具列表及其针对特定
   提供程序和模型组合的
   JSON 架构参数。
   */
  public list<ThrowOnError extends boolean = false>(
    parameters: {
      directory?: string;
      workspace?: string;
      provider: string;
      model: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
            { in: 'query', key: 'provider' },
            { in: 'query', key: 'model' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).get<ToolListResponses, ToolListErrors, ThrowOnError>({
      url: '/experimental/tool',
      ...options,
      ...params,
    });
  }
}

export class Workspace extends HeyApiClient {
  /**
   * List 工作区
   *
   * List 所有工作区。
   */
  public list<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).get<
      ExperimentalWorkspaceListResponses,
      unknown,
      ThrowOnError
    >({
      url: '/experimental/workspace',
      ...options,
      ...params,
    });
  }

  /**
   * Create 工作空间
   *
   * Create 当前项目的工作空间
   。
   */
  public create<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
      id?: string;
      type?: string;
      branch?: string | null;
      extra?: unknown | null;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
            { in: 'body', key: 'id' },
            { in: 'body', key: 'type' },
            { in: 'body', key: 'branch' },
            { in: 'body', key: 'extra' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).post<
      ExperimentalWorkspaceCreateResponses,
      ExperimentalWorkspaceCreateErrors,
      ThrowOnError
    >({
      url: '/experimental/workspace',
      ...options,
      ...params,
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
        ...params.headers,
      },
    });
  }

  /**
   * Remove 工作区
   *
   * Remove 现有工作区。
   */
  public remove<ThrowOnError extends boolean = false>(
    parameters: {
      id: string;
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'path', key: 'id' },
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).delete<
      ExperimentalWorkspaceRemoveResponses,
      ExperimentalWorkspaceRemoveErrors,
      ThrowOnError
    >({
      url: '/experimental/workspace/{id}',
      ...options,
      ...params,
    });
  }
}

export class Session extends HeyApiClient {
  /**
   * List 会话 Get
   *
   * 跨项目的所有 OpenCo
   de 会话的列表，按最近
   更新排序。默认情况下排除 Archived 会话。
   */
  public list<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
      roots?: boolean;
      start?: number;
      cursor?: number;
      search?: string;
      limit?: number;
      archived?: boolean;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
            { in: 'query', key: 'roots' },
            { in: 'query', key: 'start' },
            { in: 'query', key: 'cursor' },
            { in: 'query', key: 'search' },
            { in: 'query', key: 'limit' },
            { in: 'query', key: 'archived' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).get<
      ExperimentalSessionListResponses,
      unknown,
      ThrowOnError
    >({
      url: '/experimental/session',
      ...options,
      ...params,
    });
  }
}

export class Resource extends HeyApiClient {
  /**
   * Get MCP 资源
   *
   * Get 来自连接的服务器的所有可用
   MCP 资源。
   Optionally 按名称过滤。
   */
  public list<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).get<
      ExperimentalResourceListResponses,
      unknown,
      ThrowOnError
    >({
      url: '/experimental/resource',
      ...options,
      ...params,
    });
  }
}

export class Experimental extends HeyApiClient {
  private _workspace?: Workspace;
  get workspace(): Workspace {
    return (this._workspace ??= new Workspace({ client: this.client }));
  }

  private _session?: Session;
  get session(): Session {
    return (this._session ??= new Session({ client: this.client }));
  }

  private _resource?: Resource;
  get resource(): Resource {
    return (this._resource ??= new Resource({ client: this.client }));
  }
}

export class Worktree extends HeyApiClient {
  /**
   * Remove workt
   *
   * ree Remove 一个
   git worktree
   并删除其分支。
   */
  public remove<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
      worktreeRemoveInput?: WorktreeRemoveInput;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
            { key: 'worktreeRemoveInput', map: 'body' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).delete<
      WorktreeRemoveResponses,
      WorktreeRemoveErrors,
      ThrowOnError
    >({
      url: '/experimental/worktree',
      ...options,
      ...params,
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
        ...params.headers,
      },
    });
  }

  /**
   * List worktre
   *
   * es List 当前项目的所
   有沙箱工作树。
   */
  public list<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).get<WorktreeListResponses, unknown, ThrowOnError>({
      url: '/experimental/worktree',
      ...options,
      ...params,
    });
  }

  /**
   * Create workt
   *
   * ree Create
   当前项目的新 git
   工作树并运行任何配置的启动脚本。
   */
  public create<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
      worktreeCreateInput?: WorktreeCreateInput;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
            { key: 'worktreeCreateInput', map: 'body' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).post<
      WorktreeCreateResponses,
      WorktreeCreateErrors,
      ThrowOnError
    >({
      url: '/experimental/worktree',
      ...options,
      ...params,
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
        ...params.headers,
      },
    });
  }

  /**
   * Reset 工作树
   *
   * Reset 工作树分支到主默
   认分支。
   */
  public reset<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
      worktreeResetInput?: WorktreeResetInput;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
            { key: 'worktreeResetInput', map: 'body' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).post<
      WorktreeResetResponses,
      WorktreeResetErrors,
      ThrowOnError
    >({
      url: '/experimental/worktree/reset',
      ...options,
      ...params,
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
        ...params.headers,
      },
    });
  }
}

export class Session2 extends HeyApiClient {
  /**
   * List 会话 Get
   *
   * 所有 OpenCode
   会话的列表，按最近更新排
   序。
   */
  public list<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
      roots?: boolean;
      start?: number;
      search?: string;
      limit?: number;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
            { in: 'query', key: 'roots' },
            { in: 'query', key: 'start' },
            { in: 'query', key: 'search' },
            { in: 'query', key: 'limit' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).get<SessionListResponses, unknown, ThrowOnError>({
      url: '/session',
      ...options,
      ...params,
    });
  }

  /**
   * Create 会话
   *
   * Create 一个新的
   OpenCode
   会话，用于与 AI 助手交互并管理对话。
   */
  public create<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
      parentID?: string;
      title?: string;
      permission?: PermissionRuleset;
      workspaceID?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
            { in: 'body', key: 'parentID' },
            { in: 'body', key: 'title' },
            { in: 'body', key: 'permission' },
            { in: 'body', key: 'workspaceID' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).post<
      SessionCreateResponses,
      SessionCreateErrors,
      ThrowOnError
    >({
      url: '/session',
      ...options,
      ...params,
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
        ...params.headers,
      },
    });
  }

  /**
   * Get 会话状态
   *
   * Retrieve 所有会话的当前状态
   ，包括活动、空闲和已完成
   状态。
   */
  public status<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).get<
      SessionStatusResponses,
      SessionStatusErrors,
      ThrowOnError
    >({
      url: '/session/status',
      ...options,
      ...params,
    });
  }

  /**
   * Delete 会话
   *
   * Delete 会话并永久删除
   所有关联数据，包括消息和
   历史记录。
   */
  public delete<ThrowOnError extends boolean = false>(
    parameters: {
      sessionID: string;
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'path', key: 'sessionID' },
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).delete<
      SessionDeleteResponses,
      SessionDeleteErrors,
      ThrowOnError
    >({
      url: '/session/{sessionID}',
      ...options,
      ...params,
    });
  }

  /**
   * Get 会话 Retri
   *
   * eve 有关特定
   OpenCode
   会话的详细信息。
   */
  public get<ThrowOnError extends boolean = false>(
    parameters: {
      sessionID: string;
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'path', key: 'sessionID' },
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).get<
      SessionGetResponses,
      SessionGetErrors,
      ThrowOnError
    >({
      url: '/session/{sessionID}',
      ...options,
      ...params,
    });
  }

  /**
   * Update 会话
   *
   * Update 现有会话的属性
   ，例如标题或其他元数据。
   */
  public update<ThrowOnError extends boolean = false>(
    parameters: {
      sessionID: string;
      directory?: string;
      workspace?: string;
      title?: string;
      time?: {
        archived?: number;
      };
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'path', key: 'sessionID' },
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
            { in: 'body', key: 'title' },
            { in: 'body', key: 'time' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).patch<
      SessionUpdateResponses,
      SessionUpdateErrors,
      ThrowOnError
    >({
      url: '/session/{sessionID}',
      ...options,
      ...params,
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
        ...params.headers,
      },
    });
  }

  /**
   * Get session
   *
   * Children Retrieve
   从指定父会话派生的所有子
   会话。
   */
  public children<ThrowOnError extends boolean = false>(
    parameters: {
      sessionID: string;
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'path', key: 'sessionID' },
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).get<
      SessionChildrenResponses,
      SessionChildrenErrors,
      ThrowOnError
    >({
      url: '/session/{sessionID}/children',
      ...options,
      ...params,
    });
  }

  /**
   * Get 会话待办事项
   *
   * Retrieve 与特定会话关联的
   待办事项列表，显示任务和
   操作项。
   */
  public todo<ThrowOnError extends boolean = false>(
    parameters: {
      sessionID: string;
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'path', key: 'sessionID' },
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).get<
      SessionTodoResponses,
      SessionTodoErrors,
      ThrowOnError
    >({
      url: '/session/{sessionID}/todo',
      ...options,
      ...params,
    });
  }

  /**
   * Initialize
   *
   * 会话 Analyze
   当前应用程序，并使用特定
   于项目的代理配置创建 AGENTS.md 文件。
   */
  public init<ThrowOnError extends boolean = false>(
    parameters: {
      sessionID: string;
      directory?: string;
      workspace?: string;
      modelID?: string;
      providerID?: string;
      messageID?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'path', key: 'sessionID' },
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
            { in: 'body', key: 'modelID' },
            { in: 'body', key: 'providerID' },
            { in: 'body', key: 'messageID' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).post<
      SessionInitResponses,
      SessionInitErrors,
      ThrowOnError
    >({
      url: '/session/{sessionID}/init',
      ...options,
      ...params,
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
        ...params.headers,
      },
    });
  }

  /**
   * Fork 会话
   *
   * Create 通过在特定
   消息点分叉现有会话来创建
   新会话。
   */
  public fork<ThrowOnError extends boolean = false>(
    parameters: {
      sessionID: string;
      directory?: string;
      workspace?: string;
      messageID?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'path', key: 'sessionID' },
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
            { in: 'body', key: 'messageID' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).post<SessionForkResponses, unknown, ThrowOnError>({
      url: '/session/{sessionID}/fork',
      ...options,
      ...params,
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
        ...params.headers,
      },
    });
  }

  /**
   * Abort 会话
   *
   * Abort 一个活动会话并
   停止任何正在进行的 AI
   处理或命令执行。
   */
  public abort<ThrowOnError extends boolean = false>(
    parameters: {
      sessionID: string;
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'path', key: 'sessionID' },
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).post<
      SessionAbortResponses,
      SessionAbortErrors,
      ThrowOnError
    >({
      url: '/session/{sessionID}/abort',
      ...options,
      ...params,
    });
  }

  /**
   * Unshare
   *
   * session Remove
   会话的可共享链接，使其再
   次变为私有。
   */
  public unshare<ThrowOnError extends boolean = false>(
    parameters: {
      sessionID: string;
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'path', key: 'sessionID' },
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).delete<
      SessionUnshareResponses,
      SessionUnshareErrors,
      ThrowOnError
    >({
      url: '/session/{sessionID}/share',
      ...options,
      ...params,
    });
  }

  /**
   * Share sessio
   *
   * n Create
   会话的可共享链接，允许其
   他人查看对话。
   */
  public share<ThrowOnError extends boolean = false>(
    parameters: {
      sessionID: string;
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'path', key: 'sessionID' },
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).post<
      SessionShareResponses,
      SessionShareErrors,
      ThrowOnError
    >({
      url: '/session/{sessionID}/share',
      ...options,
      ...params,
    });
  }

  /**
   * Get 消息 diff
   *
   * Get 会话中特定用户消息导致的
   文件更改 (diff)。
   */
  public diff<ThrowOnError extends boolean = false>(
    parameters: {
      sessionID: string;
      directory?: string;
      workspace?: string;
      messageID?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'path', key: 'sessionID' },
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
            { in: 'query', key: 'messageID' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).get<SessionDiffResponses, unknown, ThrowOnError>({
      url: '/session/{sessionID}/diff',
      ...options,
      ...params,
    });
  }

  /**
   * Summarize 会话
   *
   * Generate 使用 AI
   压缩来保留关键信息的会话
   的简洁摘要。
   */
  public summarize<ThrowOnError extends boolean = false>(
    parameters: {
      sessionID: string;
      directory?: string;
      workspace?: string;
      providerID?: string;
      modelID?: string;
      auto?: boolean;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'path', key: 'sessionID' },
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
            { in: 'body', key: 'providerID' },
            { in: 'body', key: 'modelID' },
            { in: 'body', key: 'auto' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).post<
      SessionSummarizeResponses,
      SessionSummarizeErrors,
      ThrowOnError
    >({
      url: '/session/{sessionID}/summarize',
      ...options,
      ...params,
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
        ...params.headers,
      },
    });
  }

  /**
   * Get 会话消息
   *
   * Retrieve 会话中的所有消息，包括
   用户提示和 AI 响应。
   */
  public messages<ThrowOnError extends boolean = false>(
    parameters: {
      sessionID: string;
      directory?: string;
      workspace?: string;
      limit?: number;
      before?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'path', key: 'sessionID' },
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
            { in: 'query', key: 'limit' },
            { in: 'query', key: 'before' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).get<
      SessionMessagesResponses,
      SessionMessagesErrors,
      ThrowOnError
    >({
      url: '/session/{sessionID}/message',
      ...options,
      ...params,
    });
  }

  /**
   * Send 消息
   *
   * Create 并向会话发
   送一条新消息，流式传输
   AI 响应。
   */
  public prompt<ThrowOnError extends boolean = false>(
    parameters: {
      sessionID: string;
      directory?: string;
      workspace?: string;
      messageID?: string;
      model?: {
        providerID: string;
        modelID: string;
      };
      agent?: string;
      noReply?: boolean;
      tools?: {
        [key: string]: boolean;
      };
      format?: OutputFormat;
      system?: string;
      variant?: string;
      parts?: Array<TextPartInput | FilePartInput | AgentPartInput | SubtaskPartInput>;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'path', key: 'sessionID' },
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
            { in: 'body', key: 'messageID' },
            { in: 'body', key: 'model' },
            { in: 'body', key: 'agent' },
            { in: 'body', key: 'noReply' },
            { in: 'body', key: 'tools' },
            { in: 'body', key: 'format' },
            { in: 'body', key: 'system' },
            { in: 'body', key: 'variant' },
            { in: 'body', key: 'parts' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).post<
      SessionPromptResponses,
      SessionPromptErrors,
      ThrowOnError
    >({
      url: '/session/{sessionID}/message',
      ...options,
      ...params,
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
        ...params.headers,
      },
    });
  }

  /**
   * Delete messa
   *
   * ge Permanently
   从会话中删除特定消息（及
   其所有部分）。 This 不会恢复处理消息时可能进行的任何文件更改。
   */
  public deleteMessage<ThrowOnError extends boolean = false>(
    parameters: {
      sessionID: string;
      messageID: string;
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'path', key: 'sessionID' },
            { in: 'path', key: 'messageID' },
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).delete<
      SessionDeleteMessageResponses,
      SessionDeleteMessageErrors,
      ThrowOnError
    >({
      url: '/session/{sessionID}/message/{messageID}',
      ...options,
      ...params,
    });
  }

  /**
   * Get 消息 Retri
   *
   * eve 来自会话的特定消
   息（通过其消息 ID）。
   */
  public message<ThrowOnError extends boolean = false>(
    parameters: {
      sessionID: string;
      messageID: string;
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'path', key: 'sessionID' },
            { in: 'path', key: 'messageID' },
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).get<
      SessionMessageResponses,
      SessionMessageErrors,
      ThrowOnError
    >({
      url: '/session/{sessionID}/message/{messageID}',
      ...options,
      ...params,
    });
  }

  /**
   * Send 异步消息
   *
   * Create 并向会话异步发送新消息
   ，如果需要则启动会话并立
   即返回。
   */
  public promptAsync<ThrowOnError extends boolean = false>(
    parameters: {
      sessionID: string;
      directory?: string;
      workspace?: string;
      messageID?: string;
      model?: {
        providerID: string;
        modelID: string;
      };
      agent?: string;
      noReply?: boolean;
      tools?: {
        [key: string]: boolean;
      };
      format?: OutputFormat;
      system?: string;
      variant?: string;
      parts?: Array<TextPartInput | FilePartInput | AgentPartInput | SubtaskPartInput>;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'path', key: 'sessionID' },
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
            { in: 'body', key: 'messageID' },
            { in: 'body', key: 'model' },
            { in: 'body', key: 'agent' },
            { in: 'body', key: 'noReply' },
            { in: 'body', key: 'tools' },
            { in: 'body', key: 'format' },
            { in: 'body', key: 'system' },
            { in: 'body', key: 'variant' },
            { in: 'body', key: 'parts' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).post<
      SessionPromptAsyncResponses,
      SessionPromptAsyncErrors,
      ThrowOnError
    >({
      url: '/session/{sessionID}/prompt_async',
      ...options,
      ...params,
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
        ...params.headers,
      },
    });
  }

  /**
   * Send 命令 Send
   *
   * 会话的新命令，由 AI
   助手执行。
   */
  public command<ThrowOnError extends boolean = false>(
    parameters: {
      sessionID: string;
      directory?: string;
      workspace?: string;
      messageID?: string;
      agent?: string;
      model?: string;
      arguments?: string;
      command?: string;
      variant?: string;
      parts?: Array<{
        id?: string;
        type: 'file';
        mime: string;
        filename?: string;
        url: string;
        source?: FilePartSource;
      }>;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'path', key: 'sessionID' },
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
            { in: 'body', key: 'messageID' },
            { in: 'body', key: 'agent' },
            { in: 'body', key: 'model' },
            { in: 'body', key: 'arguments' },
            { in: 'body', key: 'command' },
            { in: 'body', key: 'variant' },
            { in: 'body', key: 'parts' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).post<
      SessionCommandResponses,
      SessionCommandErrors,
      ThrowOnError
    >({
      url: '/session/{sessionID}/command',
      ...options,
      ...params,
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
        ...params.headers,
      },
    });
  }

  /**
   * Run shell 命令
   *
   * Execute 会话上下文中的
   shell 命令并返回
   AI 的响应。
   */
  public shell<ThrowOnError extends boolean = false>(
    parameters: {
      sessionID: string;
      directory?: string;
      workspace?: string;
      agent?: string;
      model?: {
        providerID: string;
        modelID: string;
      };
      command?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'path', key: 'sessionID' },
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
            { in: 'body', key: 'agent' },
            { in: 'body', key: 'model' },
            { in: 'body', key: 'command' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).post<
      SessionShellResponses,
      SessionShellErrors,
      ThrowOnError
    >({
      url: '/session/{sessionID}/shell',
      ...options,
      ...params,
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
        ...params.headers,
      },
    });
  }

  /**
   * Revert messa
   *
   * ge Revert
   会话中的特定消息，撤消其
   效果并恢复之前的状态。
   */
  public revert<ThrowOnError extends boolean = false>(
    parameters: {
      sessionID: string;
      directory?: string;
      workspace?: string;
      messageID?: string;
      partID?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'path', key: 'sessionID' },
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
            { in: 'body', key: 'messageID' },
            { in: 'body', key: 'partID' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).post<
      SessionRevertResponses,
      SessionRevertErrors,
      ThrowOnError
    >({
      url: '/session/{sessionID}/revert',
      ...options,
      ...params,
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
        ...params.headers,
      },
    });
  }

  /**
   * Restore 恢复消息
   *
   * Restore 会话中所有先前恢复的消息。
   */
  public unrevert<ThrowOnError extends boolean = false>(
    parameters: {
      sessionID: string;
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'path', key: 'sessionID' },
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).post<
      SessionUnrevertResponses,
      SessionUnrevertErrors,
      ThrowOnError
    >({
      url: '/session/{sessionID}/unrevert',
      ...options,
      ...params,
    });
  }
}

export class Part extends HeyApiClient {
  /**
   * Delete 消息的一部分
   */
  public delete<ThrowOnError extends boolean = false>(
    parameters: {
      sessionID: string;
      messageID: string;
      partID: string;
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'path', key: 'sessionID' },
            { in: 'path', key: 'messageID' },
            { in: 'path', key: 'partID' },
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).delete<
      PartDeleteResponses,
      PartDeleteErrors,
      ThrowOnError
    >({
      url: '/session/{sessionID}/message/{messageID}/part/{partID}',
      ...options,
      ...params,
    });
  }

  /**
   * Update 消息中的一部分
   */
  public update<ThrowOnError extends boolean = false>(
    parameters: {
      sessionID: string;
      messageID: string;
      partID: string;
      directory?: string;
      workspace?: string;
      part?: Part2;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'path', key: 'sessionID' },
            { in: 'path', key: 'messageID' },
            { in: 'path', key: 'partID' },
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
            { key: 'part', map: 'body' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).patch<
      PartUpdateResponses,
      PartUpdateErrors,
      ThrowOnError
    >({
      url: '/session/{sessionID}/message/{messageID}/part/{partID}',
      ...options,
      ...params,
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
        ...params.headers,
      },
    });
  }
}

export class Permission extends HeyApiClient {
  /**
   * Respond 权限
   *
   * Approve 或拒绝 AI 助手的权限请求。
   *
   * @deprecated
   */
  public respond<ThrowOnError extends boolean = false>(
    parameters: {
      sessionID: string;
      permissionID: string;
      directory?: string;
      workspace?: string;
      response?: 'once' | 'always' | 'reject';
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'path', key: 'sessionID' },
            { in: 'path', key: 'permissionID' },
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
            { in: 'body', key: 'response' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).post<
      PermissionRespondResponses,
      PermissionRespondErrors,
      ThrowOnError
    >({
      url: '/session/{sessionID}/permissions/{permissionID}',
      ...options,
      ...params,
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
        ...params.headers,
      },
    });
  }

  /**
   * Respond 向
   *
   * Approve 发送权限请求或拒绝 AI 助手的权限请求。
   */
  public reply<ThrowOnError extends boolean = false>(
    parameters: {
      requestID: string;
      directory?: string;
      workspace?: string;
      reply?: 'once' | 'always' | 'reject';
      message?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'path', key: 'requestID' },
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
            { in: 'body', key: 'reply' },
            { in: 'body', key: 'message' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).post<
      PermissionReplyResponses,
      PermissionReplyErrors,
      ThrowOnError
    >({
      url: '/permission/{requestID}/reply',
      ...options,
      ...params,
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
        ...params.headers,
      },
    });
  }

  /**
   * List 待处理权限
   *
   * Get 所有会话中的所有待处理权限请求。
   */
  public list<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).get<PermissionListResponses, unknown, ThrowOnError>({
      url: '/permission',
      ...options,
      ...params,
    });
  }
}

export class Question extends HeyApiClient {
  /**
   * List 待处理问题
   *
   * Get 所有会话中的所有待处理问题请求。
   */
  public list<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).get<QuestionListResponses, unknown, ThrowOnError>({
      url: '/question',
      ...options,
      ...params,
    });
  }

  /**
   * Reply 回答问题请求
   *
   * Provide 回答 AI 助手的问题请求。
   */
  public reply<ThrowOnError extends boolean = false>(
    parameters: {
      requestID: string;
      directory?: string;
      workspace?: string;
      answers?: Array<QuestionAnswer>;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'path', key: 'requestID' },
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
            { in: 'body', key: 'answers' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).post<
      QuestionReplyResponses,
      QuestionReplyErrors,
      ThrowOnError
    >({
      url: '/question/{requestID}/reply',
      ...options,
      ...params,
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
        ...params.headers,
      },
    });
  }

  /**
   * Reject 问题请求
   *
   * Reject 来自 AI 助理的问题请求。
   */
  public reject<ThrowOnError extends boolean = false>(
    parameters: {
      requestID: string;
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'path', key: 'requestID' },
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).post<
      QuestionRejectResponses,
      QuestionRejectErrors,
      ThrowOnError
    >({
      url: '/question/{requestID}/reject',
      ...options,
      ...params,
    });
  }
}

export class Oauth extends HeyApiClient {
  /**
   * OAuth 授权
   *
   * Initiate OAuth
   授权特定 AI
   提供商以获得授权 URL。
   */
  public authorize<ThrowOnError extends boolean = false>(
    parameters: {
      providerID: string;
      directory?: string;
      workspace?: string;
      method?: number;
      inputs?: {
        [key: string]: string;
      };
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'path', key: 'providerID' },
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
            { in: 'body', key: 'method' },
            { in: 'body', key: 'inputs' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).post<
      ProviderOauthAuthorizeResponses,
      ProviderOauthAuthorizeErrors,
      ThrowOnError
    >({
      url: '/provider/{providerID}/oauth/authorize',
      ...options,
      ...params,
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
        ...params.headers,
      },
    });
  }

  /**
   * OAuth 回调
   *
   * Handle 用户授权后来自
   提供商的 OAuth
   回调。
   */
  public callback<ThrowOnError extends boolean = false>(
    parameters: {
      providerID: string;
      directory?: string;
      workspace?: string;
      method?: number;
      code?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'path', key: 'providerID' },
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
            { in: 'body', key: 'method' },
            { in: 'body', key: 'code' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).post<
      ProviderOauthCallbackResponses,
      ProviderOauthCallbackErrors,
      ThrowOnError
    >({
      url: '/provider/{providerID}/oauth/callback',
      ...options,
      ...params,
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
        ...params.headers,
      },
    });
  }
}

export class Provider extends HeyApiClient {
  /**
   * List 提供程序
   *
   * Get 所有可用 AI
   提供程序的列表，包括可用
   的和已连接的提供程序。
   */
  public list<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).get<ProviderListResponses, unknown, ThrowOnError>({
      url: '/provider',
      ...options,
      ...params,
    });
  }

  /**
   * Get 提供程序身份验证
   *
   * 方法 Retrieve 适用于所有 AI
   提供程序的可用身份验证方
   法。
   */
  public auth<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).get<ProviderAuthResponses, unknown, ThrowOnError>({
      url: '/provider/auth',
      ...options,
      ...params,
    });
  }

  private _oauth?: Oauth;
  get oauth(): Oauth {
    return (this._oauth ??= new Oauth({ client: this.client }));
  }
}

export class Find extends HeyApiClient {
  /**
   * Find text
   *
   * Search 用于使用
   ripgrep
   项目中跨文件的文本模式。
   */
  public text<ThrowOnError extends boolean = false>(
    parameters: {
      directory?: string;
      workspace?: string;
      pattern: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
            { in: 'query', key: 'pattern' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).get<FindTextResponses, unknown, ThrowOnError>({
      url: '/find',
      ...options,
      ...params,
    });
  }

  /**
   * Find 文件
   *
   * Search 用于项目目
   录中按名称或模式排列的文
   件或目录。
   */
  public files<ThrowOnError extends boolean = false>(
    parameters: {
      directory?: string;
      workspace?: string;
      query: string;
      dirs?: 'true' | 'false';
      type?: 'file' | 'directory';
      limit?: number;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
            { in: 'query', key: 'query' },
            { in: 'query', key: 'dirs' },
            { in: 'query', key: 'type' },
            { in: 'query', key: 'limit' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).get<FindFilesResponses, unknown, ThrowOnError>({
      url: '/find/file',
      ...options,
      ...params,
    });
  }

  /**
   * Find 符号
   *
   * Search 用于使用
   LSP 的函数、类和变量
   等工作区符号。
   */
  public symbols<ThrowOnError extends boolean = false>(
    parameters: {
      directory?: string;
      workspace?: string;
      query: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
            { in: 'query', key: 'query' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).get<FindSymbolsResponses, unknown, ThrowOnError>({
      url: '/find/symbol',
      ...options,
      ...params,
    });
  }
}

export class File extends HeyApiClient {
  /**
   * List 文件 List
   *
   * 指定路径中的文件和目录。
   */
  public list<ThrowOnError extends boolean = false>(
    parameters: {
      directory?: string;
      workspace?: string;
      path: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
            { in: 'query', key: 'path' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).get<FileListResponses, unknown, ThrowOnError>({
      url: '/file',
      ...options,
      ...params,
    });
  }

  /**
   * Read 文件 Read
   *
   * 指定文件的内容。
   */
  public read<ThrowOnError extends boolean = false>(
    parameters: {
      directory?: string;
      workspace?: string;
      path: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
            { in: 'query', key: 'path' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).get<FileReadResponses, unknown, ThrowOnError>({
      url: '/file/content',
      ...options,
      ...params,
    });
  }

  /**
   * Get 文件状态 Get
   *
   * 项目中所有文件的 git
   状态。
   */
  public status<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).get<FileStatusResponses, unknown, ThrowOnError>({
      url: '/file/status',
      ...options,
      ...params,
    });
  }
}

export class Event extends HeyApiClient {
  /**
   * Subscribe
   *
   * 至事件 Get 事件
   */
  public subscribe<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).sse.get<EventSubscribeResponses, unknown, ThrowOnError>(
      {
        url: '/event',
        ...options,
        ...params,
      },
    );
  }
}

export class Auth2 extends HeyApiClient {
  /**
   * Remove MCP
   *
   * OAuth Remove
   OAuth MCP
   服务器的凭据
   */
  public remove<ThrowOnError extends boolean = false>(
    parameters: {
      name: string;
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'path', key: 'name' },
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).delete<
      McpAuthRemoveResponses,
      McpAuthRemoveErrors,
      ThrowOnError
    >({
      url: '/mcp/{name}/auth',
      ...options,
      ...params,
    });
  }

  /**
   * Start MCP
   *
   * OAuth Start
   OAuth Model
   Context Protocol (MCP) 服务器的身份验证流程。
   */
  public start<ThrowOnError extends boolean = false>(
    parameters: {
      name: string;
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'path', key: 'name' },
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).post<
      McpAuthStartResponses,
      McpAuthStartErrors,
      ThrowOnError
    >({
      url: '/mcp/{name}/auth',
      ...options,
      ...params,
    });
  }

  /**
   * Complete MCP
   *
   * OAuth Complete
   OAuth 使用授权码对
   Model Context Protocol (MCP) 服务器进行身份验证。
   */
  public callback<ThrowOnError extends boolean = false>(
    parameters: {
      name: string;
      directory?: string;
      workspace?: string;
      code?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'path', key: 'name' },
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
            { in: 'body', key: 'code' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).post<
      McpAuthCallbackResponses,
      McpAuthCallbackErrors,
      ThrowOnError
    >({
      url: '/mcp/{name}/auth/callback',
      ...options,
      ...params,
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
        ...params.headers,
      },
    });
  }

  /**
   * Authenticate
   *
   * MCP OAuth Start OAuth
   流程并等待回调（打开浏览
   器）
   */
  public authenticate<ThrowOnError extends boolean = false>(
    parameters: {
      name: string;
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'path', key: 'name' },
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).post<
      McpAuthAuthenticateResponses,
      McpAuthAuthenticateErrors,
      ThrowOnError
    >({
      url: '/mcp/{name}/auth/authenticate',
      ...options,
      ...params,
    });
  }
}

export class Mcp extends HeyApiClient {
  /**
   * Get MCP 状态
   *
   * Get 所有 Model
   Context
   Protocol (MCP) 服务器的状态。
   */
  public status<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).get<McpStatusResponses, unknown, ThrowOnError>({
      url: '/mcp',
      ...options,
      ...params,
    });
  }

  /**
   * Add MCP 服务器
   *
   * Dynamically
   将新的 Model
   Context Protocol (MCP) 服务器添加到系统。
   */
  public add<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
      name?: string;
      config?: McpLocalConfig | McpRemoteConfig;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
            { in: 'body', key: 'name' },
            { in: 'body', key: 'config' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).post<McpAddResponses, McpAddErrors, ThrowOnError>({
      url: '/mcp',
      ...options,
      ...params,
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
        ...params.headers,
      },
    });
  }

  /**
   * Connect 一个 MCP 服务器
   */
  public connect<ThrowOnError extends boolean = false>(
    parameters: {
      name: string;
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'path', key: 'name' },
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).post<McpConnectResponses, unknown, ThrowOnError>({
      url: '/mcp/{name}/connect',
      ...options,
      ...params,
    });
  }

  /**
   * Disconnect 一个 MCP 服务器
   */
  public disconnect<ThrowOnError extends boolean = false>(
    parameters: {
      name: string;
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'path', key: 'name' },
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).post<McpDisconnectResponses, unknown, ThrowOnError>({
      url: '/mcp/{name}/disconnect',
      ...options,
      ...params,
    });
  }

  private _auth?: Auth2;
  get auth(): Auth2 {
    return (this._auth ??= new Auth2({ client: this.client }));
  }
}

export class Control extends HeyApiClient {
  /**
   * Get 下一个 TUI
   *
   * 请求 Retrieve 队列中的下一个
   TUI (Termina
   l User Interface) 请求进行处理。
   */
  public next<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).get<TuiControlNextResponses, unknown, ThrowOnError>({
      url: '/tui/control/next',
      ...options,
      ...params,
    });
  }

  /**
   * Submit TUI
   *
   * 响应 Submit 对 TUI
   请求队列的响应，以完成待
   处理的请求。
   */
  public response<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
      body?: unknown;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
            { key: 'body', map: 'body' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).post<
      TuiControlResponseResponses,
      unknown,
      ThrowOnError
    >({
      url: '/tui/control/response',
      ...options,
      ...params,
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
        ...params.headers,
      },
    });
  }
}

export class Tui extends HeyApiClient {
  /**
   * Append TUI
   *
   * 提示 Append 提示到 TUI
   */
  public appendPrompt<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
      text?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
            { in: 'body', key: 'text' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).post<
      TuiAppendPromptResponses,
      TuiAppendPromptErrors,
      ThrowOnError
    >({
      url: '/tui/append-prompt',
      ...options,
      ...params,
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
        ...params.headers,
      },
    });
  }

  /**
   * Open 帮助对话框
   *
   * Open TUI 中的帮助对话框
   用于显示用户帮助信息。
   */
  public openHelp<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).post<TuiOpenHelpResponses, unknown, ThrowOnError>({
      url: '/tui/open-help',
      ...options,
      ...params,
    });
  }

  /**
   * Open 会话对话框
   *
   * Open 会话对话框
   */
  public openSessions<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).post<TuiOpenSessionsResponses, unknown, ThrowOnError>({
      url: '/tui/open-sessions',
      ...options,
      ...params,
    });
  }

  /**
   * Open 主题对话框
   *
   * Open 主题对话框
   */
  public openThemes<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).post<TuiOpenThemesResponses, unknown, ThrowOnError>({
      url: '/tui/open-themes',
      ...options,
      ...params,
    });
  }

  /**
   * Open 模型对话框
   *
   * Open 模型对话框
   */
  public openModels<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).post<TuiOpenModelsResponses, unknown, ThrowOnError>({
      url: '/tui/open-models',
      ...options,
      ...params,
    });
  }

  /**
   * Submit TUI
   *
   * 提示 Submit 提示
   */
  public submitPrompt<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).post<TuiSubmitPromptResponses, unknown, ThrowOnError>({
      url: '/tui/submit-prompt',
      ...options,
      ...params,
    });
  }

  /**
   * Clear TUI 提示
   *
   * Clear 提示
   */
  public clearPrompt<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).post<TuiClearPromptResponses, unknown, ThrowOnError>({
      url: '/tui/clear-prompt',
      ...options,
      ...params,
    });
  }

  /**
   * Execute TUI
   *
   * 命令 Execute TUI
   命令（例如 agent_
   cycle）
   */
  public executeCommand<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
      command?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
            { in: 'body', key: 'command' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).post<
      TuiExecuteCommandResponses,
      TuiExecuteCommandErrors,
      ThrowOnError
    >({
      url: '/tui/execute-command',
      ...options,
      ...params,
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
        ...params.headers,
      },
    });
  }

  /**
   * Show TUI
   *
   * toast Show TUI
   中的 Toast 通知
   */
  public showToast<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
      title?: string;
      message?: string;
      variant?: 'info' | 'success' | 'warning' | 'error';
      duration?: number;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
            { in: 'body', key: 'title' },
            { in: 'body', key: 'message' },
            { in: 'body', key: 'variant' },
            { in: 'body', key: 'duration' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).post<TuiShowToastResponses, unknown, ThrowOnError>({
      url: '/tui/show-toast',
      ...options,
      ...params,
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
        ...params.headers,
      },
    });
  }

  /**
   * Publish TUI
   *
   * 事件 Publish TUI 事件
   */
  public publish<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
      body?:
        | EventTuiPromptAppend
        | EventTuiCommandExecute
        | EventTuiToastShow
        | EventTuiSessionSelect;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
            { key: 'body', map: 'body' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).post<
      TuiPublishResponses,
      TuiPublishErrors,
      ThrowOnError
    >({
      url: '/tui/publish',
      ...options,
      ...params,
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
        ...params.headers,
      },
    });
  }

  /**
   * Select sessi
   *
   * on Navigate
   TUI 显示指定的会话。
   */
  public selectSession<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
      sessionID?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
            { in: 'body', key: 'sessionID' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).post<
      TuiSelectSessionResponses,
      TuiSelectSessionErrors,
      ThrowOnError
    >({
      url: '/tui/select-session',
      ...options,
      ...params,
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
        ...params.headers,
      },
    });
  }

  private _control?: Control;
  get control(): Control {
    return (this._control ??= new Control({ client: this.client }));
  }
}

export class Instance extends HeyApiClient {
  /**
   * Dispose 实例
   *
   * Clean 启动并处置当前
   OpenCode
   实例，释放所有资源。
   */
  public dispose<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).post<InstanceDisposeResponses, unknown, ThrowOnError>({
      url: '/instance/dispose',
      ...options,
      ...params,
    });
  }
}

export class Path extends HeyApiClient {
  /**
   * Get 路径 Retri
   *
   * eve OpenCode
   实例的当前工作目录和相关
   路径信息。
   */
  public get<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).get<PathGetResponses, unknown, ThrowOnError>({
      url: '/path',
      ...options,
      ...params,
    });
  }
}

export class Vcs extends HeyApiClient {
  /**
   * Get VCS info
   *
   * Retrieve
   当前项目的版本控制系统（
   VCS）信息，例如 git 分支。
   */
  public get<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).get<VcsGetResponses, unknown, ThrowOnError>({
      url: '/vcs',
      ...options,
      ...params,
    });
  }
}

export class Command extends HeyApiClient {
  /**
   * List 命令 Get
   *
   * OpenCode
   系统中所有可用命令的列表
   。
   */
  public list<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).get<CommandListResponses, unknown, ThrowOnError>({
      url: '/command',
      ...options,
      ...params,
    });
  }
}

export class App extends HeyApiClient {
  /**
   * Write log
   *
   * Write 具有指定级别
   和元数据的服务器日志的日
   志条目。
   */
  public log<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
      service?: string;
      level?: 'debug' | 'info' | 'error' | 'warn';
      message?: string;
      extra?: {
        [key: string]: unknown;
      };
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
            { in: 'body', key: 'service' },
            { in: 'body', key: 'level' },
            { in: 'body', key: 'message' },
            { in: 'body', key: 'extra' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).post<AppLogResponses, AppLogErrors, ThrowOnError>({
      url: '/log',
      ...options,
      ...params,
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
        ...params.headers,
      },
    });
  }

  /**
   * List 代理 Get
   *
   * OpenCode
   系统中所有可用 AI
   代理的列表。
   */
  public agents<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).get<AppAgentsResponses, unknown, ThrowOnError>({
      url: '/agent',
      ...options,
      ...params,
    });
  }

  /**
   * List 技能 Get
   *
   * OpenCode
   系统中所有可用技能的列表
   。
   */
  public skills<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).get<AppSkillsResponses, unknown, ThrowOnError>({
      url: '/skill',
      ...options,
      ...params,
    });
  }
}

export class Lsp extends HeyApiClient {
  /**
   * Get LSP 状态
   *
   * Get LSP 服务器状态
   */
  public status<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).get<LspStatusResponses, unknown, ThrowOnError>({
      url: '/lsp',
      ...options,
      ...params,
    });
  }
}

export class Formatter extends HeyApiClient {
  /**
   * Get 格式化程序状态
   *
   * Get 格式化程序状态
   */
  public status<ThrowOnError extends boolean = false>(
    parameters?: {
      directory?: string;
      workspace?: string;
    },
    options?: Options<never, ThrowOnError>,
  ) {
    const params = buildClientParams(
      [parameters],
      [
        {
          args: [
            { in: 'query', key: 'directory' },
            { in: 'query', key: 'workspace' },
          ],
        },
      ],
    );
    return (options?.client ?? this.client).get<FormatterStatusResponses, unknown, ThrowOnError>({
      url: '/formatter',
      ...options,
      ...params,
    });
  }
}

export class OpencodeClient extends HeyApiClient {
  public static readonly __registry = new HeyApiRegistry<OpencodeClient>();

  constructor(args?: { client?: Client; key?: string }) {
    super(args);
    OpencodeClient.__registry.set(this, args?.key);
  }

  private _global?: Global;
  get global(): Global {
    return (this._global ??= new Global({ client: this.client }));
  }

  private _auth?: Auth;
  get auth(): Auth {
    return (this._auth ??= new Auth({ client: this.client }));
  }

  private _project?: Project;
  get project(): Project {
    return (this._project ??= new Project({ client: this.client }));
  }

  private _pty?: Pty;
  get pty(): Pty {
    return (this._pty ??= new Pty({ client: this.client }));
  }

  private _config?: Config2;
  get config(): Config2 {
    return (this._config ??= new Config2({ client: this.client }));
  }

  private _tool?: Tool;
  get tool(): Tool {
    return (this._tool ??= new Tool({ client: this.client }));
  }

  private _experimental?: Experimental;
  get experimental(): Experimental {
    return (this._experimental ??= new Experimental({ client: this.client }));
  }

  private _worktree?: Worktree;
  get worktree(): Worktree {
    return (this._worktree ??= new Worktree({ client: this.client }));
  }

  private _session?: Session2;
  get session(): Session2 {
    return (this._session ??= new Session2({ client: this.client }));
  }

  private _part?: Part;
  get part(): Part {
    return (this._part ??= new Part({ client: this.client }));
  }

  private _permission?: Permission;
  get permission(): Permission {
    return (this._permission ??= new Permission({ client: this.client }));
  }

  private _question?: Question;
  get question(): Question {
    return (this._question ??= new Question({ client: this.client }));
  }

  private _provider?: Provider;
  get provider(): Provider {
    return (this._provider ??= new Provider({ client: this.client }));
  }

  private _find?: Find;
  get find(): Find {
    return (this._find ??= new Find({ client: this.client }));
  }

  private _file?: File;
  get file(): File {
    return (this._file ??= new File({ client: this.client }));
  }

  private _event?: Event;
  get event(): Event {
    return (this._event ??= new Event({ client: this.client }));
  }

  private _mcp?: Mcp;
  get mcp(): Mcp {
    return (this._mcp ??= new Mcp({ client: this.client }));
  }

  private _tui?: Tui;
  get tui(): Tui {
    return (this._tui ??= new Tui({ client: this.client }));
  }

  private _instance?: Instance;
  get instance(): Instance {
    return (this._instance ??= new Instance({ client: this.client }));
  }

  private _path?: Path;
  get path(): Path {
    return (this._path ??= new Path({ client: this.client }));
  }

  private _vcs?: Vcs;
  get vcs(): Vcs {
    return (this._vcs ??= new Vcs({ client: this.client }));
  }

  private _command?: Command;
  get command(): Command {
    return (this._command ??= new Command({ client: this.client }));
  }

  private _app?: App;
  get app(): App {
    return (this._app ??= new App({ client: this.client }));
  }

  private _lsp?: Lsp;
  get lsp(): Lsp {
    return (this._lsp ??= new Lsp({ client: this.client }));
  }

  private _formatter?: Formatter;
  get formatter(): Formatter {
    return (this._formatter ??= new Formatter({ client: this.client }));
  }
}
