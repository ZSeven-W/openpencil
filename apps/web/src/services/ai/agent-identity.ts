/**
 * 并发设计场景下的子代理身份分配器。
 *
 * 为每个子代理分配唯一的颜色和名字，
 * 方便用户在画布上直观看出是谁在绘制哪一部分。
 */

const AGENT_COLORS = [
  '#FF6B6B', // 珊瑚红
  '#4ECDC4', // 青色
  '#FFD93D', // 金黄色
  '#6C5CE7', // 紫色
  '#A8E6CF', // 薄荷绿
  '#FF8A5C', // 暖橙色
];

const AGENT_NAMES = [
  'Kiki',
  'Mochi',
  'Pixel',
  'Nova',
  'Zuri',
  'Cleo',
  'Boba',
  'Rune',
  'Fern',
  'Echo',
  'Puck',
  'Sage',
];

export interface AgentIdentity {
  color: string;
  name: string;
}

/**
 * 为 `count` 个代理分配唯一身份（颜色 + 名字）。
 * 颜色按调色板循环，名字会先做一次随机洗牌。
 */
export function assignAgentIdentities(count: number): AgentIdentity[] {
  // 用 Fisher-Yates 算法打乱名字顺序
  const shuffled = [...AGENT_NAMES];
  for (let i = shuffled.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [shuffled[i], shuffled[j]] = [shuffled[j], shuffled[i]];
  }

  const identities: AgentIdentity[] = [];
  for (let i = 0; i < count; i++) {
    identities.push({
      color: AGENT_COLORS[i % AGENT_COLORS.length],
      name: shuffled[i % shuffled.length],
    });
  }
  return identities;
}
