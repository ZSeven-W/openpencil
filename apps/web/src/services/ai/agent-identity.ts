/**
 * Agent identity system for concurrent design generation.
 *
 * Assigns a unique color and name to each sub-agent so the user can
 * visually distinguish which agent is building which section on the canvas.
 */

const AGENT_COLORS = [
  '#FF6B6B', // coral red
  '#4ECDC4', // teal
  '#FFD93D', // golden yellow
  '#6C5CE7', // purple
  '#A8E6CF', // mint green
  '#FF8A5C', // warm orange
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
 * Assign unique identities (color + name) to `count` agents.
 * Colors cycle from the palette; names are shuffled randomly.
 */
export function assignAgentIdentities(count: number): AgentIdentity[] {
  // Shuffle names using Fisher-Yates
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
