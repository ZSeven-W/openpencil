// This 文件由 @hey-api/openapi-ts 自动生成

import type { ArrayStyle, ObjectStyle, SerializerOptions } from './pathSerializer.gen.js';

export type QuerySerializer = (query: Record<string, unknown>) => string;

export type BodySerializer = (body: any) => any;

type QuerySerializerOptionsObject = {
  allowReserved?: boolean;
  array?: Partial<SerializerOptions<ArrayStyle>>;
  object?: Partial<SerializerOptions<ObjectStyle>>;
};

export type QuerySerializerOptions = QuerySerializerOptionsObject & {
  /**
   * Per-参数序列化覆盖。
   * When 提供，这些设置会覆盖特定参数名称的全局 array/object 设置。
   */
  parameters?: Record<string, QuerySerializerOptionsObject>;
};

const serializeFormDataPair = (data: FormData, key: string, value: unknown): void => {
  if (typeof value === 'string' || value instanceof Blob) {
    data.append(key, value);
  } else if (value instanceof Date) {
    data.append(key, value.toISOString());
  } else {
    data.append(key, JSON.stringify(value));
  }
};

const serializeUrlSearchParamsPair = (data: URLSearchParams, key: string, value: unknown): void => {
  if (typeof value === 'string') {
    data.append(key, value);
  } else {
    data.append(key, JSON.stringify(value));
  }
};

export const formDataBodySerializer = {
  bodySerializer: <T extends Record<string, any> | Array<Record<string, any>>>(
    body: T,
  ): FormData => {
    const data = new FormData();

    Object.entries(body).forEach(([key, value]) => {
      if (value === undefined || value === null) {
        return;
      }
      if (Array.isArray(value)) {
        value.forEach((v) => serializeFormDataPair(data, key, v));
      } else {
        serializeFormDataPair(data, key, value);
      }
    });

    return data;
  },
};

export const jsonBodySerializer = {
  bodySerializer: <T>(body: T): string =>
    JSON.stringify(body, (_key, value) => (typeof value === 'bigint' ? value.toString() : value)),
};

export const urlSearchParamsBodySerializer = {
  bodySerializer: <T extends Record<string, any> | Array<Record<string, any>>>(body: T): string => {
    const data = new URLSearchParams();

    Object.entries(body).forEach(([key, value]) => {
      if (value === undefined || value === null) {
        return;
      }
      if (Array.isArray(value)) {
        value.forEach((v) => serializeUrlSearchParamsPair(data, key, v));
      } else {
        serializeUrlSearchParamsPair(data, key, value);
      }
    });

    return data.toString();
  },
};
