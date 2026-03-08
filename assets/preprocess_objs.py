#!/usr/bin/env python3
"""
Preprocess OBJ files to add smooth normals (for files without them)
and triangulate quads. Also fix the space shuttle MTL reference.
Run from the assets/ directory.
"""

import math
import os


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
            # Only handle triangles here (quads handled separately)
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
            # Use same index for vertex and normal (per-vertex normals)
            out.write(f'f {i0}//{i0} {i1}//{i1} {i2}//{i2}\n')

    print(f'  {os.path.basename(src)}: {len(vertices)} verts, {len(faces)} tris, normals added')


def triangulate_shuttle(src, dst, mtl_fix=None):
    """
    Triangulate quad faces (fan), fix MTL reference, keep v/vt/vn indices.
    """
    out_lines = []
    tri_count = 0
    quad_count = 0

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
                elif len(parts) == 4:
                    # Fan triangulate: 0-1-2 and 0-2-3
                    out_lines.append(f'f {parts[0]} {parts[1]} {parts[2]}\n')
                    out_lines.append(f'f {parts[0]} {parts[2]} {parts[3]}\n')
                    quad_count += 1
                else:
                    # Polygon: fan from first vertex
                    for i in range(1, len(parts) - 1):
                        out_lines.append(f'f {parts[0]} {parts[i]} {parts[i+1]}\n')
                    quad_count += 1
            else:
                out_lines.append(line)

    with open(dst, 'w') as out:
        out.writelines(out_lines)

    print(f'  {os.path.basename(src)}: {tri_count} tris kept, {quad_count} quads → 2 tris each')


def main():
    assets_dir = os.path.dirname(os.path.abspath(__file__))

    # Files that need normals added (plain `f v1 v2 v3` format)
    needs_normals = [
        'cow.obj',
        'teapot.obj',
        'stanford-bunny.obj',
        'lucy.obj',
        'xyzrgb_dragon.obj',
    ]

    print('Adding smooth normals:')
    for name in needs_normals:
        src = os.path.join(assets_dir, name)
        if os.path.exists(src):
            add_normals_to_obj(src, src)
        else:
            print(f'  SKIP (not found): {name}')

    # Space shuttle: triangulate quads + fix MTL reference
    print('Fixing space shuttle (triangulate + MTL reference):')
    shuttle = os.path.join(assets_dir, 'space_shuttle_low_poly.obj')
    if os.path.exists(shuttle):
        triangulate_shuttle(shuttle, shuttle, mtl_fix='space_shuttle_low_poly.mtl')
    else:
        print('  SKIP: space_shuttle_low_poly.obj not found')

    print('Done.')


if __name__ == '__main__':
    main()
