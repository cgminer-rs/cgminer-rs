use std::env;
use std::path::PathBuf;

fn main() {
    // 告诉 Cargo 在这些文件改变时重新运行构建脚本
    println!("cargo:rerun-if-changed=drivers/");
    println!("cargo:rerun-if-changed=build.rs");
    
    // 获取输出目录
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    
    // 编译 C 驱动程序
    compile_c_drivers(&out_path);
    
    // 生成 Rust 绑定
    generate_bindings(&out_path);
    
    // 链接库
    link_libraries();
}

fn compile_c_drivers(out_path: &PathBuf) {
    let mut build = cc::Build::new();
    
    build
        .file("drivers/maijie_l7.c")
        .include("drivers/")
        .flag("-std=c99")
        .flag("-Wall")
        .flag("-Wextra")
        .flag("-O2");
    
    // 根据目标平台添加特定的编译选项
    let target = env::var("TARGET").unwrap();
    
    if target.contains("aarch64") || target.contains("arm") {
        build.flag("-march=armv8-a");
        build.flag("-mtune=cortex-a72");
    }
    
    if target.contains("linux") {
        build.flag("-D_GNU_SOURCE");
        build.flag("-pthread");
    }
    
    // 添加调试信息（在调试模式下）
    if env::var("PROFILE").unwrap() == "debug" {
        build.flag("-g");
        build.flag("-DDEBUG");
    } else {
        build.flag("-DNDEBUG");
    }
    
    // 编译静态库
    build.compile("maijie_l7");
    
    // 输出库文件路径
    println!("cargo:rustc-link-search=native={}", out_path.display());
    println!("cargo:rustc-link-lib=static=maijie_l7");
}

fn generate_bindings(out_path: &PathBuf) {
    // 使用 bindgen 生成 Rust 绑定
    let bindings = bindgen::Builder::default()
        .header("drivers/maijie_l7.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings");
    
    // 写入绑定文件
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

fn link_libraries() {
    // 链接系统库
    println!("cargo:rustc-link-lib=pthread");
    println!("cargo:rustc-link-lib=m");
    
    // 根据目标平台链接特定库
    let target = env::var("TARGET").unwrap();
    
    if target.contains("linux") {
        println!("cargo:rustc-link-lib=rt");
        println!("cargo:rustc-link-lib=dl");
    }
    
    // 如果是交叉编译，可能需要指定库搜索路径
    if let Ok(sysroot) = env::var("CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER") {
        if sysroot.contains("aarch64") {
            // 添加交叉编译工具链的库路径
            println!("cargo:rustc-link-search=native=/usr/aarch64-linux-gnu/lib");
        }
    }
}
