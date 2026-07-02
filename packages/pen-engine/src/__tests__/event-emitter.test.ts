import { describe, it, expect, vi } from 'vitest';
import { TypedEventEmitter } from '../core/event-emitter';

interface TestEvents {
  foo: (value: number) => void;
  bar: (a: string, b: boolean) => void;
  baz: () => void;
}

describe('TypedEventEmitter', () => {
  it('should call registered listeners', () => {
    const emitter = new TypedEventEmitter<TestEvents>();
    const cb = vi.fn();
    emitter.on('foo', cb);
    emitter.emit('foo', 42);
    expect(cb).toHaveBeenCalledWith(42);
  });

  it('should support multiple listeners', () => {
    const emitter = new TypedEventEmitter<TestEvents>();
    const cb1 = vi.fn();
    const cb2 = vi.fn();
    emitter.on('foo', cb1);
    emitter.on('foo', cb2);
    emitter.emit('foo', 7);
    expect(cb1).toHaveBeenCalledWith(7);
    expect(cb2).toHaveBeenCalledWith(7);
  });

  it('on() should return an unsubscribe function', () => {
    const emitter = new TypedEventEmitter<TestEvents>();
    const cb = vi.fn();
    const unsub = emitter.on('foo', cb);
    emitter.emit('foo', 1);
    expect(cb).toHaveBeenCalledTimes(1);
    unsub();
    emitter.emit('foo', 2);
    expect(cb).toHaveBeenCalledTimes(1);
  });

  it('off() should remove a listener', () => {
    const emitter = new TypedEventEmitter<TestEvents>();
    const cb = vi.fn();
    emitter.on('foo', cb);
    emitter.off('foo', cb);
    emitter.emit('foo', 1);
    expect(cb).not.toHaveBeenCalled();
  });

  it('should handle multi-arg events', () => {
    const emitter = new TypedEventEmitter<TestEvents>();
    const cb = vi.fn();
    emitter.on('bar', cb);
    emitter.emit('bar', 'hello', true);
    expect(cb).toHaveBeenCalledWith('hello', true);
  });

  it('should handle no-arg events', () => {
    const emitter = new TypedEventEmitter<TestEvents>();
    const cb = vi.fn();
    emitter.on('baz', cb);
    emitter.emit('baz');
    expect(cb).toHaveBeenCalledTimes(1);
  });

  it('should not throw when emitting with no listeners', () => {
    const emitter = new TypedEventEmitter<TestEvents>();
    expect(() => emitter.emit('foo', 1)).not.toThrow();
  });

  it('dispose() should remove all listeners', () => {
    const emitter = new TypedEventEmitter<TestEvents>();
    const cb1 = vi.fn();
    const cb2 = vi.fn();
    emitter.on('foo', cb1);
    emitter.on('bar', cb2);
    emitter.dispose();
    emitter.emit('foo', 1);
    emitter.emit('bar', 'x', false);
    expect(cb1).not.toHaveBeenCalled();
    expect(cb2).not.toHaveBeenCalled();
  });
});
