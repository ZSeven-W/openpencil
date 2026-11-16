// These tags are the wire-stable Jian PackedOpCode tags. Authored expressions
// are compiled in Rust; no authored string is evaluated as JavaScript.
const own = (value, key) => value != null && Object.hasOwn(value, key)
  ? value[key] : null;
const object = value => value !== null && typeof value === "object" && !Array.isArray(value);
const truthy = value => object(value) ? Object.keys(value).length > 0
  : Array.isArray(value) ? value.length > 0 : Boolean(value);
const display = value => value == null ? "" : typeof value === "object"
  ? JSON.stringify(value) : String(value);
const number = value => value == null ? 0 : ["boolean", "number"].includes(typeof value)
  ? Number(value) : typeof value === "string" && value.trim() === value &&
    /^[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?$/.test(value) ? Number(value) : NaN;
function equal(left, right) {
  if (left === right) return true;
  if (Array.isArray(left) && Array.isArray(right)) {
    return left.length === right.length && left.every((value, index) => equal(value, right[index]));
  }
  if (object(left) && object(right)) {
    const keys = Object.keys(left);
    return keys.length === Object.keys(right).length &&
      keys.every(key => Object.hasOwn(right, key) && equal(left[key], right[key]));
  }
  return false;
}
const finite = value => Number.isFinite(value) ? value : null;
function binary(tag, left, right) {
  if (tag === 13 && (typeof left === "string" || typeof right === "string")) {
    return display(left) + display(right);
  }
  if (tag === 13 && Array.isArray(left) && Array.isArray(right)) return left.concat(right);
  if (tag >= 18 && tag <= 21) return tag === 18 || tag === 20
    ? equal(left, right) : !equal(left, right);
  const a = number(left), b = number(right);
  switch (tag) {
    case 13: return finite(a + b);
    case 14: return finite(a - b);
    case 15: return finite(a * b);
    case 16: return finite(a / b);
    case 17: return finite(a % b);
    case 22: return a < b;
    case 23: return a > b;
    case 24: return a <= b;
    case 25: return a >= b;
  }
  return null;
}

const builtins = new Map(Object.entries({
  len: value => typeof value === "string" || Array.isArray(value) ? [...value].length : null,
  length: value => typeof value === "string" || Array.isArray(value) ? [...value].length : null,
  lower: value => typeof value === "string" ? value.toLowerCase() : null,
  upper: value => typeof value === "string" ? value.toUpperCase() : null,
  trim: value => typeof value === "string" ? value.trim() : null,
  contains: (value, part) => typeof value === "string" && typeof part === "string"
    ? value.includes(part) : Array.isArray(value) ? value.some(item => equal(item, part)) : null,
  startsWith: (value, part) => typeof value === "string" && typeof part === "string" ? value.startsWith(part) : null,
  endsWith: (value, part) => typeof value === "string" && typeof part === "string" ? value.endsWith(part) : null,
  abs: value => finite(Math.abs(number(value))),
  floor: value => finite(Math.floor(number(value))),
  ceil: value => finite(Math.ceil(number(value))),
  round: value => finite(Math.sign(number(value)) * Math.round(Math.abs(number(value)))),
  min: (...args) => finite(Math.min(...args.map(number))),
  max: (...args) => finite(Math.max(...args.map(number))),
  clamp: (value, lo, hi) => finite(Math.max(number(lo), Math.min(number(hi), number(value)))),
  keys: value => object(value) ? Object.keys(value).sort() : null,
  values: value => object(value) ? Object.keys(value).sort().map(key => value[key]) : null,
}));

function evaluate(ops, lookup) {
  const stack = [];
  const take = n => stack.splice(stack.length - n, n);
  for (let ip = 0; ip < ops.length; ip++) {
    const [tag, arg, count] = ops[ip];
    switch (tag) {
      case 0: case 1: case 3: case 7: stack.push(arg); break;
      case 2: stack.push(null); break;
      case 4: stack.push(lookup(arg)); break;
      case 5: stack.push(take(arg)); break;
      case 6: {
        const entries = take(arg * 2);
        const value = Object.create(null);
        for (let i = 0; i < entries.length; i += 2) value[entries[i]] = entries[i + 1];
        stack.push(value);
        break;
      }
      case 8: {
        const value = stack.pop();
        stack.push(object(value) ? own(value, arg) : null);
        break;
      }
      case 9: {
        const index = stack.pop(), value = stack.pop();
        stack.push(Array.isArray(value) && typeof index === "number" ? own(value, Math.trunc(index))
          : object(value) && typeof index === "string" ? own(value, index) : null);
        break;
      }
      case 10: stack.push(!truthy(stack.pop())); break;
      case 11: stack.push(-(number(stack.pop()) || 0)); break;
      case 12: stack.push(number(stack.pop()) || 0); break;
      case 26: case 27:
        if (truthy(stack.at(-1)) === (tag === 27)) ip += arg;
        else stack.pop();
        break;
      case 28: ip += arg; break;
      case 29: {
        const right = stack.pop();
        stack.push(stack.pop() ?? right);
        break;
      }
      case 30: {
        const right = display(stack.pop());
        stack.push(display(stack.pop()) + right);
        break;
      }
      case 31: {
        const args = take(count);
        stack.push(builtins.get(arg)?.(...args) ?? null);
        break;
      }
      case 32: return stack.pop() ?? null;
      default: {
        const right = stack.pop();
        stack.push(binary(tag, stack.pop(), right));
      }
    }
  }
  return null;
}
