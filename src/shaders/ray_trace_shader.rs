vulkano_shaders::shader! {
    ty: "compute",
    path: "shaders/ray_trace.comp",
    spirv_version: "1.6",
    vulkan_version: "1.2",
    linalg_type: "cgmath"
}
