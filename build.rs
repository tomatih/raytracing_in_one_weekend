use std::{env, path::Path, process::Command};

fn build_shader(file_name: String){

	let src_path = Path::new("shaders").join(&file_name);

	println!("cargo::rerun-if-changed={}", src_path.clone().into_os_string().into_string().unwrap());

	let out_dir = env::var_os("OUT_DIR").unwrap();
	let dest_path = Path::new(&out_dir).join(format!("{}.spv",file_name));
	
	if dest_path.exists(){
		std::fs::remove_file(&dest_path).unwrap();
	}

	Command::new("glslang")
		.args(&["--target-env", "vulkan1.3", "-g", &src_path.into_os_string().into_string().unwrap(), "-o", &dest_path.into_os_string().into_string().unwrap()])
		.status()
		.unwrap();

	assert!(Path::new(&out_dir).join(format!("{}.spv",file_name)).exists())
}

fn main() {
	build_shader("ray_trace.comp".into());
	build_shader("finalize.comp".into());
}