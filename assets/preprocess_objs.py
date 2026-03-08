#!/usr/bin/env python3
"""
Preprocess OBJ files: add smooth normals or triangulate quads.

Usage:
  # Add smooth per-vertex normals (for files with plain `f v1 v2 v3` faces):
  python preprocess_objs.py add-normals --input a.obj b.obj --output a_out.obj b_out.obj

  # Triangulate quad faces (and optionally fix MTL reference):
  python preprocess_objs.py triangulate --input shuttle_raw.obj --output shuttle.obj [--mtl shuttle.mtl]
"""

import argparse
import math
import os
import sys


def normalize(v):
    length = math.sqrt(sum(x * x for x in v))
    if length < 1e-10:
        return [0.0, 1.0, 0.0]
    return [x / length for x in v]


def cross(a, b):
    return [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]


def sub(a, b):
    return [a[i] - b[i] for i in range(3)]


def add_normals_to_obj(src, dst):
    """
    Parse an OBJ with plain `f v1 v2 v3` faces (no normals), compute
    smooth per-vertex normals, and rewrite as `f v1//n1 v2//n2 v3//n3`.
    """
    vertices = []
    faces = []  # list of [i0, i1, i2] (0-indexed)
    header_lines = []  # everything before the first vertex
    between_lines = []  # lines between vertices and faces (e.g. mtllib, g, s)

    in_vertices = False
    in_faces = False

    with open(src) as f:
        lines = f.readlines()

    for line in lines:
        s = line.strip()
        if s.startswith('v ') and not s.startswith('vt ') and not s.startswith('vn '):
            parts = s.split()
            vertices.append([float(parts[1]), float(parts[2]), float(parts[3])])
            in_vertices = True
        elif s.startswith('f '):
            parts = s.split()[1:]
            idxs = [int(p.split('/')[0]) - 1 for p in parts]
            if len(idxs) == 3:
                faces.append(idxs)
            elif len(idxs) == 4:
                # Fan triangulation
                faces.append([idxs[0], idxs[1], idxs[2]])
                faces.append([idxs[0], idxs[2], idxs[3]])
            in_faces = True
        elif not in_vertices and not in_faces:
            header_lines.append(line.rstrip('\n'))
        elif in_vertices and not in_faces:
            between_lines.append(line.rstrip('\n'))

    # Compute per-vertex smooth normals
    normals = [[0.0, 0.0, 0.0] for _ in vertices]
    for f in faces:
        v0, v1, v2 = vertices[f[0]], vertices[f[1]], vertices[f[2]]
        n = cross(sub(v1, v0), sub(v2, v0))
        for vi in f:
            normals[vi] = [normals[vi][j] + n[j] for j in range(3)]
    normals = [normalize(n) for n in normals]

    with open(dst, 'w') as out:
        for line in header_lines:
            out.write(line + '\n')
        for v in vertices:
            out.write(f'v {v[0]} {v[1]} {v[2]}\n')
        for line in between_lines:
            out.write(line + '\n')
        for n in normals:
            out.write(f'vn {n[0]:.6f} {n[1]:.6f} {n[2]:.6f}\n')
        for f in faces:
            i0, i1, i2 = f[0] + 1, f[1] + 1, f[2] + 1
            out.write(f'f {i0}//{i0} {i1}//{i1} {i2}//{i2}\n')

    print(f'  {os.path.basename(src)} -> {os.path.basename(dst)}: '
          f'{len(vertices)} verts, {len(faces)} tris, normals added')


def triangulate_obj(src, dst, mtl_fix=None):
    """
    Triangulate quad/polygon faces (fan), optionally fix MTL reference,
    keep existing v/vt/vn indices.
    """
    out_lines = []
    tri_count = 0
    poly_count = 0

    with open(src) as f:
        for line in f:
            s = line.strip()
            if mtl_fix and s.startswith('mtllib '):
                out_lines.append(f'mtllib {mtl_fix}\n')
                continue
            if s.startswith('f '):
                parts = s.split()[1:]
                if len(parts) == 3:
                    out_lines.append(line)
                    tri_count += 1
                else:
                    # Fan triangulate from first vertex
                    for i in range(1, len(parts) - 1):
                        out_lines.append(f'f {parts[0]} {parts[i]} {parts[i+1]}\n')
                    poly_count += 1
            else:
                out_lines.append(line)

    with open(dst, 'w') as out:
        out.writelines(out_lines)

    print(f'  {os.path.basename(src)} -> {os.path.basename(dst)}: '
          f'{tri_count} tris kept, {poly_count} polys triangulated')


def cmd_add_normals(args):
    if len(args.input) != len(args.output):
        sys.exit(f'error: --input has {len(args.input)} files but --output has {len(args.output)}')
    for src, dst in zip(args.input, args.output):
        if not os.path.exists(src):
            sys.exit(f'error: input file not found: {src}')
        add_normals_to_obj(src, dst)


def cmd_triangulate(args):
    if len(args.input) != len(args.output):
        sys.exit(f'error: --input has {len(args.input)} files but --output has {len(args.output)}')
    if args.mtl and len(args.input) > 1:
        sys.exit('error: --mtl can only be used with a single input file')
    for src, dst in zip(args.input, args.output):
        if not os.path.exists(src):
            sys.exit(f'error: input file not found: {src}')
        triangulate_obj(src, dst, mtl_fix=args.mtl)


def main():
    parser = argparse.ArgumentParser(
        description='Preprocess OBJ files: add smooth normals or triangulate quads.',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )
    sub = parser.add_subparsers(dest='command', required=True)

    p_normals = sub.add_parser('add-normals', help='Compute and add smooth per-vertex normals')
    p_normals.add_argument('--input',  '-i', nargs='+', required=True, metavar='FILE', help='Input OBJ file(s)')
    p_normals.add_argument('--output', '-o', nargs='+', required=True, metavar='FILE', help='Output OBJ file(s)')
    p_normals.set_defaults(func=cmd_add_normals)

    p_tri = sub.add_parser('triangulate', help='Triangulate quad/polygon faces')
    p_tri.add_argument('--input',  '-i', nargs='+', required=True, metavar='FILE', help='Input OBJ file(s)')
    p_tri.add_argument('--output', '-o', nargs='+', required=True, metavar='FILE', help='Output OBJ file(s)')
    p_tri.add_argument('--mtl', metavar='NAME', help='Replace mtllib reference with this filename (single-file only)')
    p_tri.set_defaults(func=cmd_triangulate)

    args = parser.parse_args()
    args.func(args)


if __name__ == '__main__':
    main()
