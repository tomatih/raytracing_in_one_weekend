# Raytracing In One Weekend
My attempt at working through the [_Ray Tracing in One Weekend_](https://raytracing.github.io/books/RayTracingInOneWeekend.html) book

## Progress
- [x] Overview
- [x] Output an Image
- [x] The vec3 Class
- [x] Rays, a Simple Camera, and Background
- [x] Adding a Sphere
- [x] Surface Normals and Multiple Objects
- [x] Antialiasing
- [x] Diffuse Materials
- [x] Metal
- [x] Dielectrics
- [x] Positionable Camera
- [x] Defocus Blur
- [x] Where Next?

# Personal extension
To speed up rendering process porting the code to Vulkan compute shaders

## Progress
- [x] Setup Vulkan boilerplate
- [x] Format data so it can be moved onto the GPU
- [x] Write shaders to process the data
- [x] Add staging buffers for world data
- [x] Move to unsafe binding while chasing UB
- [x] Cleanup the code
- [x] Move to multiple smaller command buffers for better compatibility
- [x] Setup SDL3 windowing
- [x] Single sample rendering to the window
- [x] Dynamic camera
- [ ] Window resizing
- [ ] Temporal multi sample
- [ ] Detection and support for hardware accelerated RT (rayquery)
- [ ] (?) Sofwfare implementation of BVH as a fallback
