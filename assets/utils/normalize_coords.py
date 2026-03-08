#!/usr/bin/env python3
"""
Center OBJ meshes at origin and rotate from Y-up to Z-up convention.

Usage:
  python normalize_coords.py -i input1.obj [input2.obj ...] -o output1.obj [output2.obj ...]

Optional flags:
  --no-rotate   Skip Y-up -> Z-up rotation (centering still applied)
  --no-center   Skip centroid centering (rotation still applied)

Transform applied (order: center first, then rotate):

  Y-up -> Z-up rotation is Rx(+90 deg):

      | 1   0   0 |
  R = | 0   0  -1 |
      | 0   1   0 |

  Vertices:   R * (v - centroid)    (center, then rotate)
  Normals:    R * n                 (rotate only, no translation)
"""

import argparse
import os
import sys


def matvec3(m, v):
    """Multiply a 3x3 row-major matrix by a 3-element vector."""
    return [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]


# Rx(+90°): maps Y-up axis (0,1,0) -> Z-up axis (0,0,1)
#   x' = x
#   y' = -z
#   z' = y
ROT_Y_UP_TO_Z_UP = [
    [1,  0,  0],
    [0,  0, -1],
    [0,  1,  0],
]


def normalize_obj(src, dst, do_center=True, do_rotate=True):
    with open(src) as f:
        lines = f.readlines()

    vertices = []
    normals = []

    # First pass: collect vertices and normals; track line types in order
    # Each entry: ('v', index) | ('vn', index) | ('other', original_line)
    parsed = []

    for line in lines:
        s = line.strip()
        if s.startswith('v ') and not s.startswith('vt ') and not s.startswith('vn '):
            parts = s.split()
            vertices.append([float(parts[1]), float(parts[2]), float(parts[3])])
            parsed.append(('v', len(vertices) - 1))
        elif s.startswith('vn '):
            parts = s.split()
            normals.append([float(parts[1]), float(parts[2]), float(parts[3])])
            parsed.append(('vn', len(normals) - 1))
        else:
            parsed.append(('other', line))

    if not vertices:
        sys.exit(f'error: no vertices found in {src}')

    # Compute centroid
    n = len(vertices)
    cx = sum(v[0] for v in vertices) / n
    cy = sum(v[1] for v in vertices) / n
    cz = sum(v[2] for v in vertices) / n

    # Bounding box before transform (for diagnostics)
    xs = [v[0] for v in vertices]
    ys = [v[1] for v in vertices]
    zs = [v[2] for v in vertices]
    bb_min = [min(xs), min(ys), min(zs)]
    bb_max = [max(xs), max(ys), max(zs)]

    # Build transformed vertices: center, then optionally rotate
    transformed_verts = []
    for v in vertices:
        centered = [
            v[0] - (cx if do_center else 0.0),
            v[1] - (cy if do_center else 0.0),
            v[2] - (cz if do_center else 0.0),
        ]
        if do_rotate:
            transformed_verts.append(matvec3(ROT_Y_UP_TO_Z_UP, centered))
        else:
            transformed_verts.append(centered)

    # Build transformed normals: rotate only (no translation)
    transformed_normals = []
    for nv in normals:
        if do_rotate:
            transformed_normals.append(matvec3(ROT_Y_UP_TO_Z_UP, nv))
        else:
            transformed_normals.append(list(nv))

    # Write output in original line order
    with open(dst, 'w') as out:
        for kind, data in parsed:
            if kind == 'v':
                tv = transformed_verts[data]
                out.write(f'v {tv[0]} {tv[1]} {tv[2]}\n')
            elif kind == 'vn':
                tn = transformed_normals[data]
                out.write(f'vn {tn[0]:.6f} {tn[1]:.6f} {tn[2]:.6f}\n')
            else:
                out.write(data)

    ops = []
    if do_center:
        ops.append(f'centered ({cx:.4f}, {cy:.4f}, {cz:.4f})')
    if do_rotate:
        ops.append('Rx(+90) Y->Z')
    ops_str = ', '.join(ops) if ops else 'no-op'

    print(f'  {os.path.basename(src)} -> {os.path.basename(dst)}: '
          f'{len(vertices)} verts, {len(normals)} normals | {ops_str}')
    print(f'    bbox before: [{bb_min[0]:.4f},{bb_min[1]:.4f},{bb_min[2]:.4f}] '
          f'.. [{bb_max[0]:.4f},{bb_max[1]:.4f},{bb_max[2]:.4f}]')


def main():
    parser = argparse.ArgumentParser(
        description='Center OBJ meshes at origin and rotate Y-up -> Z-up.',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )
    parser.add_argument('--input',     '-i', nargs='+', required=True, metavar='FILE', help='Input OBJ file(s)')
    parser.add_argument('--output',    '-o', nargs='+', required=True, metavar='FILE', help='Output OBJ file(s)')
    parser.add_argument('--no-rotate', action='store_true', help='Skip Y-up -> Z-up rotation')
    parser.add_argument('--no-center', action='store_true', help='Skip centroid centering')

    args = parser.parse_args()

    if len(args.input) != len(args.output):
        sys.exit(f'error: -i has {len(args.input)} files but -o has {len(args.output)}')

    for src, dst in zip(args.input, args.output):
        if not os.path.exists(src):
            sys.exit(f'error: input file not found: {src}')
        normalize_obj(src, dst, do_center=not args.no_center, do_rotate=not args.no_rotate)


if __name__ == '__main__':
    main()
