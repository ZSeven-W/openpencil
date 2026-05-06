// This 文件由 @hey-api/openapi-ts 自动生成

export type AuthToken = string | undefined;

export interface Auth {
  /**
   * 我们使用 Which 请求的一部分来发送身份验证吗？
   *
   * @default '标题'
   */
  in?: 'header' | 'query' | 'cookie';
  /**
   * Header 或查询参数名称。
   *
   * @default 'Authorization'
   */
  name?: string;
  scheme?: 'basic' | 'bearer';
  type: 'apiKey' | 'http';
}

export const getAuthToken = async (
  auth: Auth,
  callback: ((auth: Auth) => Promise<AuthToken> | AuthToken) | AuthToken,
): Promise<string | undefined> => {
  const token = typeof callback === 'function' ? await callback(auth) : callback;

  if (!token) {
    return;
  }

  if (auth.scheme === 'bearer') {
    return `Bearer ${token}`;
  }

  if (auth.scheme === 'basic') {
    return `Basic ${btoa(token)}`;
  }

  return token;
};
