# Rotation Visualizer

## Representations:

- Axis-Angle (4d)
- Angle-Axis (3d)
    - Visualized as a flag
- Quaternion
    - Conventions: wxyz or xyzw
- Rotation Matrix
- Euler Angles
    - Conventions: xyz, zxy, yzx, xzy, yxz, zyx
    - Visualized as arcs

2 Modes:
1. Input/output sliders + text boxes for all representations (URL determines the default settings)
2. Text-box input where you can specify the representation, then get an output in the desired format
    Supported input languages: Python (double-list, single-list, or np.array), Matlab, R, yaml
The 2nd mode can be active when the first mode is selected, but as soon as the user interacts with the 2nd mode, the 1st mode is deactivated.

Visualization:
- The un-rotated base mesh (in gray) + a rendered 3D mesh, rotated in 3D (black)
    - Different mesh options (eg. Coordinate axes, colored cube, Monkey, airplane)
- An axis-angle representation (visualized in 3D as a flag)
- A representation where each point corresponds to a point in 3D (ball)
- Coordinate axes and rotated coordinate-axes (arrows)
- Euler-Angle Arcs: https://compsci290-s2016.github.io/CoursePage/Materials/EulerAnglesViz/
- Linear Tangent-space visualization (Lie Algebra)

Additional Visualizations:
1. Gimbal-lock visualization: Animation of a plane going into gimbal lock and watching the roll pitch and yaw try to adjust as it pushes through the lock
2. Discussion about cross-products
3. Discussion a