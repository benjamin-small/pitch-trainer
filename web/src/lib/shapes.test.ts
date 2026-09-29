import { describe, expect, it } from 'vitest';
import { shapeGeometry } from './shapes';
import type { Shape } from './noteVisuals';

const vertexCount = (points: string) => points.trim().split(/\s+/).length;

describe('shapeGeometry', () => {
  it('builds regular polygons with the right vertex counts', () => {
    const expected: [Shape, number][] = [
      ['triangle', 3], ['invertedTriangle', 3], ['square', 4], ['diamond', 4],
      ['pentagon', 5], ['hexagon', 6], ['octagon', 8], ['star', 10],
    ];
    for (const [shape, count] of expected) {
      const geo = shapeGeometry(shape);
      expect(geo.type).toBe('polygon');
      if (geo.type === 'polygon') expect(vertexCount(geo.points)).toBe(count);
    }
  });

  it('points the triangle up and the inverted triangle down', () => {
    const up = shapeGeometry('triangle');
    const down = shapeGeometry('invertedTriangle');
    if (up.type !== 'polygon' || down.type !== 'polygon') throw new Error('expected polygons');
    const firstY = (points: string) => Number(points.split(' ')[0].split(',')[1]);
    expect(firstY(up.points)).toBeLessThan(0);
    expect(firstY(down.points)).toBeGreaterThan(0);
  });

  it('keeps every polygon inside the viewBox', () => {
    for (const shape of ['triangle', 'square', 'star', 'octagon'] as Shape[]) {
      const geo = shapeGeometry(shape);
      if (geo.type !== 'polygon') continue;
      for (const pair of geo.points.split(' ')) {
        const [x, y] = pair.split(',').map(Number);
        expect(Math.abs(x)).toBeLessThanOrEqual(50);
        expect(Math.abs(y)).toBeLessThanOrEqual(50);
      }
    }
  });

  it('handles the non-polygon shapes', () => {
    expect(shapeGeometry('circle').type).toBe('circle');
    expect(shapeGeometry('ring').type).toBe('ring');
    expect(shapeGeometry('cross').type).toBe('path');
    expect(shapeGeometry('crescent').type).toBe('path');
  });
});
