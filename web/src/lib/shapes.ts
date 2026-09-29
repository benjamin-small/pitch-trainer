import type { Shape } from './noteVisuals';

/** Geometry in the SVG space viewBox="-50 -50 100 100". */
export type ShapeGeometry =
  | { type: 'circle'; r: number }
  | { type: 'ring'; r: number; width: number }
  | { type: 'polygon'; points: string }
  | { type: 'path'; d: string };

/** Vertices around the origin; rotation 0 puts the first vertex straight up. */
function radial(radii: number[], rotationDeg: number): string {
  return radii
    .map((r, i) => {
      const angle = ((rotationDeg + (360 * i) / radii.length - 90) * Math.PI) / 180;
      return `${(r * Math.cos(angle)).toFixed(2)},${(r * Math.sin(angle)).toFixed(2)}`;
    })
    .join(' ');
}

const regular = (sides: number, radius: number, rotationDeg = 0) =>
  radial(Array(sides).fill(radius), rotationDeg);

export function shapeGeometry(shape: Shape): ShapeGeometry {
  switch (shape) {
    case 'circle':
      return { type: 'circle', r: 40 };
    case 'ring':
      return { type: 'ring', r: 34, width: 12 };
    case 'triangle':
      return { type: 'polygon', points: regular(3, 46) };
    case 'invertedTriangle':
      return { type: 'polygon', points: regular(3, 46, 180) };
    case 'square':
      return { type: 'polygon', points: regular(4, 48, 45) };
    case 'diamond':
      return { type: 'polygon', points: regular(4, 46) };
    case 'pentagon':
      return { type: 'polygon', points: regular(5, 44) };
    case 'star':
      return { type: 'polygon', points: radial([46, 19, 46, 19, 46, 19, 46, 19, 46, 19], 0) };
    case 'hexagon':
      return { type: 'polygon', points: regular(6, 44) };
    case 'cross':
      return { type: 'path', d: 'M-13,-42 H13 V-13 H42 V13 H13 V42 H-13 V13 H-42 V-13 H-13 Z' };
    case 'crescent':
      return { type: 'path', d: 'M25,-38 A40,40 0 1 0 25,38 A44,44 0 0 1 25,-38 Z' };
    case 'octagon':
      return { type: 'polygon', points: regular(8, 44, 22.5) };
  }
}
