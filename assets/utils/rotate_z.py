#!/usr/bin/env python3
"""
Rotate OBJ meshes around the Z axis by a given angle.

Usage:
  python rotate_z.py -i input1.obj [input2.obj ...] -o output1.obj [output2.obj ...] --angle <degrees>

Both vertex positions and vertex normals are rotated.
Face indices and all other data are preserved unchanged.
"""

import argparse
import math
import os
import sys


def matvec3(m, v):
    """Multiply a 3x3 row-major matrix by a 3-element vector."""
    return [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]


def rz(degrees):
    """Rotation matrix around Z axis by the given angle in degrees."""
    t = math.radians(degrees)
    c, s = math.cos(t), math.sin(t)
    return [
        [ c, -s, 0],
        [ s,  c, 0],
        [ 0,  0, 1],
    ]


def rotate_obj(src, dst, degrees):
    R = rz(degrees)

    with open(src) as f:
        lines = f.readlines()

    vertices = []
    normals = []
    parsed = []  # ('v', idx) | ('vn', idx) | ('other', line)

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

    transformed_verts   = [matvec3(R, v)  for v in vertices]
    transformed_normals = [matvec3(R, nv) for nv in normals]

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

    print(f'  {os.path.basename(src)} -> {os.path.basename(dst)}: '
          f'{len(vertices)} verts, {len(normals)} normals | Rz({degrees:+g}°)')


def main():
    parser = argparse.ArgumentParser(
        description='Rotate OBJ meshes around the Z axis.',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )
    parser.add_argument('--input',  '-i', nargs='+', required=True, metavar='FILE')
    parser.add_argument('--output', '-o', nargs='+', required=True, metavar='FILE')
    parser.add_argument('--angle',  '-a', type=float, required=True, metavar='DEG',
                        help='Rotation angle in degrees (positive = CCW when viewed from +Z)')

    args = parser.parse_args()

    if len(args.input) != len(args.output):
        sys.exit(f'error: -i has {len(args.input)} files but -o has {len(args.output)}')

    for src, dst in zip(args.input, args.output):
        if not os.path.exists(src):
            sys.exit(f'error: input file not found: {src}')
        rotate_obj(src, dst, args.angle)


if __name__ == '__main__':
    main()
