// This 文件由 @hey-api/openapi-ts 自动生成

import type { Auth, AuthToken } from './auth.gen';
import type { BodySerializer, QuerySerializer, QuerySerializerOptions } from './bodySerializer.gen';

export type HttpMethod =
  | 'connect'
  | 'delete'
  | 'get'
  | 'head'
  | 'options'
  | 'patch'
  | 'post'
  | 'put'
  | 'trace';

export type Client<
  RequestFn = never,
  Config = unknown,
  MethodFn = never,
  BuildUrlFn = never,
  SseFn = never,
> = {
  /**
   * Returns 最终请求 URL。
   */
  buildUrl: BuildUrlFn;
  getConfig: () => Config;
  request: RequestFn;
  setConfig: (config: Config) => Config;
} & {
  [K in HttpMethod]: MethodFn;
} & ([SseFn] extends [never] ? { sse?: never } : { sse: { [K in HttpMethod]: SseFn } });

export interface Config {
  /**
   * Auth 令牌或返回身份
   * 验证令牌的函数。 The 解析值将添加到由 `security` 数组定义的请求负载中。
   */
  auth?: ((auth: Auth) => Promise<AuthToken> | AuthToken) | AuthToken;
  /**
   * 序列化请求体参数的函数。 By 默认，
   * {@link JSON.stringify()} will be used.
   */
  bodySerializer?: BodySerializer | null;
  /**
   * An 对象，包含您想要预填充的任何 HTTP 标头
   * `Headers` 对象与。
   *
   * {@link https://developer.mozilla.org/docs/Web/API/Headers/Headers#init See more}
   */
  headers?:
    | RequestInit['headers']
    | Record<
        string,
        string | number | boolean | (string | number | boolean)[] | null | undefined | unknown
      >;
  /**
   * The 请求方法。
   *
   * {@link https://developer.mozilla.org/docs/Web/API/fetch#method See more}
   */
  method?: Uppercase<HttpMethod>;
  /**
   * 用于序列化请求查询参数的函数。 By 默认，数组
   * 将以表单样式爆炸，对象将以 deepObject 爆炸
   * 样式，保留字符是百分比编码的。
   *
   * 如果本机 `paramsSerializer()` Axios，则 This 方法将无效
   * 使用 API 函数。
   *
   * {@link https://swagger.io/docs/specification/serialization/#query View examples}
   */
  querySerializer?: QuerySerializer | QuerySerializerOptions;
  /**
   * 验证请求数据的函数。如果
   * 您想确保请求符合所需的形状，那么 This 非常有用，以便可以安全地将其发送到服务器。
   *
   */
  requestValidator?: (data: unknown) => Promise<unknown>;
  /**
   * 在返回之前转换响应数据的函数。 This 很有用
   * for post-processing data, e.g. converting ISO strings into Date objects.
   */
  responseTransformer?: (data: unknown) => Promise<unknown>;
  /**
   * 验证响应数据的函数。如果
   * 您想确保响应符合所需的形状，那么 This 非常有用，这样它就可以安全地传递到变压器并返回给用户。
   *
   */
  responseValidator?: (data: unknown) => Promise<unknown>;
}

type IsExactlyNeverOrNeverUndefined<T> = [T] extends [never]
  ? true
  : [T] extends [never | undefined]
    ? [undefined] extends [T]
      ? false
      : true
    : false;

export type OmitNever<T extends Record<string, unknown>> = {
  [K in keyof T as IsExactlyNeverOrNeverUndefined<T[K]> extends true ? never : K]: T[K];
};
