vulkano_shaders::shader! {
    ty: "compute",
    path: "shaders/finalize.comp",
    spirv_version: "1.6",
    vulkan_version: "1.2",
    linalg_type: "cgmath"
}
