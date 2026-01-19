# Rotation Visualizer
This tool shows different representations of a rotation in 3D, including a visual representation and various algebraic representations. All of the representations are a "view" into the same underlying rotation, and update automatically as the user interacts with the sliders or text boxes.

## Visual Layout:
On the right of the screen is a black canvas with the un-rotated (gray) and rotated (white) version of the same mesh. There are checkboxes / dropdowns to configure:
    - Different meshes (eg. Coordinate axes, colored cube, Monkey, airplane)
    - Include the axis-angle representation (visualized in 3D as a flag)
    - Include the coordinate axes and rotated coordinate-axes (arrows)
    - Include the Euler-Angle Arcs: https://compsci290-s2016.github.io/CoursePage/Materials/EulerAnglesViz/
On the left of the screen are the algebraic representations of the rotation (see below). Each representation includes a text box from/into which the user can copy/paste/type, and also sliders that follow the values of the representation.
There is also a button to toggle the matrix/vector formatting between languages (Python, Matlab, R, yaml, or user-defined). The URL of the site determines the settings.

## Representations:

- Axis-Angle (3d)
    - Visualization (can be toggled on/off): the space of possibilities is a ball; each possibility is an axis with the direction as a flag.
    - Widgets: Linear sliders for the three components
- Axis-Angle (4d)
    - Constraints: the axis must be a unit vector
    - Widgets: Linear sliders for the four components
- Quaternion
    - Conventions (checkboxes): 
        - wxyz or xyzw
    - Quirk: Double-cover: Also show the other quaternion that represents the same rotation
    - Constraints: the axis must be a unit vector
    - Widgets: Linear sliders for each of the 4 components
- Rotation Matrix
    - Constraints: the matrix must be orthonormal and determinant 1
- Euler Angles
    - Conventions (checkboxes): 
        - xyz, zxy, yzx, xzy, yxz, zyx ("roll-pitch-yaw")
        - radians or degrees
    - Visualization (can be toggled on/off): three 3D arcs similar to a physical gimbal.
    - Quirk: Gimbal lock - Numbers turn red if gimbal lock is detected
    - Widgets: Circular sliders for roll, pitch, and yaw
- First 5 numbers of a rotation matrix (the full matrix can be determined via the gram-schmidt process)

Additional Visualizations:
1. Gimbal-lock visualization: Animation of a plane going into gimbal lock and watching the roll pitch and yaw try to adjust as it pushes through the lock
2. Discussion about cross-products
3. Discussion about Lie Algebra and Groups
4. Discussion on how AI learns these representations (mechanistic interpretability)
5. Discussion on the pros and cons of different representations

Desirable properties of a rotation representation:
- Easy to visualize (ie. make a mental picture in your head given the values of the representation and vice versa)
- Easy to check equality
    - Rotation Matrix: Uniqueness is guaranteed
    - Axis-Angle (3d): modulo 2π
    - Quaternion: Double-cover
- Minimal number of parameters
    - Quaternion
- No singularities
    - Rotation Matrix
    - Axis-Angle (3d)
    - Quaternion

## References
- https://github.com/deniz-hofmeister/transforms


## Development
To run the development server, run the following, then open the browser to see the app.

```
trunk serve
```

(You can also build the project without trunk with the following command:)
```
cargo build --target wasm32-unknown-unknown --release
```