// editor rows carry a stable id so React keeps inputs and expanded state attached while rows move
export interface Keyed<T> {
  id: string;
  value: T;
}

let nextId = 0;

export function keyed<T>(value: T): Keyed<T> {
  nextId += 1;
  return { id: `row-${nextId}`, value };
}

export function moveItem<T>(items: T[], index: number, offset: -1 | 1): T[] {
  const target = index + offset;
  if (target < 0 || target >= items.length) return items;

  const next = [...items];
  [next[index], next[target]] = [next[target], next[index]];
  return next;
}
