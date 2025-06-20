use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    // 告诉 Cargo 在这些文件改变时重新运行构建脚本
    println!("cargo:rerun-if-changed=drivers/");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/");
    println!("cargo:rerun-if-changed=Cargo.toml");

    // 获取目标信息
    let _target = env::var("TARGET").unwrap();
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();

    println!("cargo:rustc-env=TARGET_OS={}", target_os);
    println!("cargo:rustc-env=TARGET_ARCH={}", target_arch);

    // 获取输出目录
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    // 生成版本信息
    generate_version_info(&out_path);

    // 设置编译特性
    setup_compile_features(&target_os, &target_arch);

    // 编译 C 驱动程序 (仅在文件存在时)
    if std::path::Path::new("drivers/maijie_l7.c").exists() {
        compile_c_drivers(&out_path);
        // 生成 Rust 绑定
        generate_bindings(&out_path);
    } else {
        // 创建空的绑定文件
        let empty_bindings = "// No C bindings available\n";
        std::fs::write(out_path.join("bindings.rs"), empty_bindings)
            .expect("Failed to write empty bindings file");
    }

    // 链接库
    link_libraries(&target_os, &target_arch);

    // 检查依赖
    check_dependencies(&target_os);
}

fn compile_c_drivers(out_path: &Path) {
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

fn generate_bindings(out_path: &Path) {
    // 使用 bindgen 生成 Rust 绑定
    let bindings = bindgen::Builder::default()
        .header("drivers/maijie_l7.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    // 写入绑定文件
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

fn link_libraries(target_os: &str, target_arch: &str) {
    match target_os {
        "linux" => {
            // Linux 系统库
            println!("cargo:rustc-link-lib=pthread");
            println!("cargo:rustc-link-lib=dl");
            println!("cargo:rustc-link-lib=m");
            println!("cargo:rustc-link-lib=rt");

            // 设置库搜索路径
            println!("cargo:rustc-link-search=native=/usr/lib");
            println!("cargo:rustc-link-search=native=/usr/local/lib");

            match target_arch {
                "x86_64" => {
                    println!("cargo:rustc-link-search=native=/usr/lib/x86_64-linux-gnu");
                    println!("cargo:rustc-link-search=native=/lib/x86_64-linux-gnu");
                }
                "aarch64" => {
                    println!("cargo:rustc-link-search=native=/usr/lib/aarch64-linux-gnu");
                    println!("cargo:rustc-link-search=native=/lib/aarch64-linux-gnu");
                    println!("cargo:rustc-link-search=native=/usr/aarch64-linux-gnu/lib");
                }
                "armv7" => {
                    println!("cargo:rustc-link-search=native=/usr/lib/arm-linux-gnueabihf");
                    println!("cargo:rustc-link-search=native=/lib/arm-linux-gnueabihf");
                }
                _ => {}
            }
        }
        "macos" => {
            // macOS 系统库和框架
            println!("cargo:rustc-link-search=native=/usr/lib");
            println!("cargo:rustc-link-search=native=/usr/local/lib");
            println!("cargo:rustc-link-search=framework=/System/Library/Frameworks");

            println!("cargo:rustc-link-lib=framework=Foundation");
            println!("cargo:rustc-link-lib=framework=IOKit");
            println!("cargo:rustc-link-lib=framework=CoreFoundation");
        }
        "windows" => {
            // Windows 系统库
            println!("cargo:rustc-link-lib=ws2_32");
            println!("cargo:rustc-link-lib=userenv");
            println!("cargo:rustc-link-lib=kernel32");
            println!("cargo:rustc-link-lib=advapi32");
        }
        _ => {}
    }
}

/// 生成版本信息
fn generate_version_info(out_path: &Path) {
    // 获取 Git 信息
    let git_hash = get_git_hash().unwrap_or_else(|| "unknown".to_string());
    let git_branch = get_git_branch().unwrap_or_else(|| "unknown".to_string());
    let build_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
    println!("cargo:rustc-env=GIT_BRANCH={}", git_branch);
    println!("cargo:rustc-env=BUILD_TIME={}", build_time);

    // 生成版本文件
    let version_content = format!(
        r#"
/// 构建信息
pub const BUILD_INFO: BuildInfo = BuildInfo {{
    version: env!("CARGO_PKG_VERSION"),
    git_hash: "{}",
    git_branch: "{}",
    build_time: {},
    target_os: env!("TARGET_OS"),
    target_arch: env!("TARGET_ARCH"),
}};

/// 构建信息结构
#[derive(Debug, Clone)]
pub struct BuildInfo {{
    pub version: &'static str,
    pub git_hash: &'static str,
    pub git_branch: &'static str,
    pub build_time: u64,
    pub target_os: &'static str,
    pub target_arch: &'static str,
}}

impl BuildInfo {{
    /// 获取完整版本字符串
    pub fn full_version(&self) -> String {{
        format!("{{}} ({{}} on {{}})", self.version, self.git_hash, self.target_os)
    }}

    /// 获取平台信息
    pub fn platform(&self) -> String {{
        format!("{{}}-{{}}", self.target_os, self.target_arch)
    }}
}}
"#,
        git_hash, git_branch, build_time
    );

    let version_file = out_path.join("version.rs");
    std::fs::write(version_file, version_content).expect("Failed to write version file");
}

/// 获取 Git 哈希
fn get_git_hash() -> Option<String> {
    Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                None
            }
        })
}

/// 获取 Git 分支
fn get_git_branch() -> Option<String> {
    Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                None
            }
        })
}

/// 设置基础编译特性（编译器优化已移至外置核心）
fn setup_compile_features(target_os: &str, target_arch: &str) {
    println!("cargo:warning=🔧 Setting up basic compile features for {}-{}", target_os, target_arch);

    // 基础平台检测
    match target_os {
        "linux" => {
            println!("cargo:rustc-cfg=target_os_linux");
        }
        "macos" => {
            println!("cargo:rustc-cfg=target_os_macos");
        }
        "windows" => {
            println!("cargo:rustc-cfg=target_os_windows");
        }
        _ => {}
    }

    // 基础架构检测
    match target_arch {
        "x86_64" => {
            println!("cargo:rustc-cfg=target_arch_x86_64");
        }
        "aarch64" => {
            println!("cargo:rustc-cfg=target_arch_aarch64");
        }
        "armv7" => {
            println!("cargo:rustc-cfg=target_arch_armv7");
        }
        _ => {}
    }

    // 检查是否为调试构建
    let profile = env::var("PROFILE").unwrap();
    if profile == "debug" {
        println!("cargo:rustc-cfg=debug_build");
    } else {
        println!("cargo:rustc-cfg=release_build");
    }
}

// 注意：编译器优化和平台优化函数已移至外置核心
// 主程序只保留基础的平台检测功能

/// 检查依赖
fn check_dependencies(target_os: &str) {
    match target_os {
        "linux" => {
            // 检查 Linux 特定依赖
            check_library_exists("pthread");
            check_library_exists("dl");
            check_library_exists("m");

            // 检查开发库
            check_header_exists("linux/spi/spidev.h");
            check_header_exists("linux/gpio.h");
        }
        "macos" => {
            // 检查 macOS 特定依赖
            check_framework_exists("IOKit");
            check_framework_exists("CoreFoundation");
        }
        "windows" => {
            // 检查 Windows 特定依赖
            check_library_exists("ws2_32");
            check_library_exists("kernel32");
        }
        _ => {}
    }
}

/// 检查库是否存在
fn check_library_exists(lib_name: &str) {
    let output = Command::new("pkg-config")
        .args(["--exists", lib_name])
        .output();

    match output {
        Ok(output) if output.status.success() => {
            println!("cargo:rustc-cfg=has_lib_{}", lib_name.replace("-", "_"));
        }
        _ => {
            println!("cargo:warning=Library {} not found", lib_name);
        }
    }
}

/// 检查头文件是否存在
fn check_header_exists(header_path: &str) {
    let header_exists = std::path::Path::new(&format!("/usr/include/{}", header_path)).exists()
        || std::path::Path::new(&format!("/usr/local/include/{}", header_path)).exists();

    if header_exists {
        let header_name = header_path.replace("/", "_").replace(".", "_");
        println!("cargo:rustc-cfg=has_header_{}", header_name);
    } else {
        println!("cargo:warning=Header {} not found", header_path);
    }
}

/// 检查框架是否存在 (macOS)
fn check_framework_exists(framework_name: &str) {
    let framework_path = format!("/System/Library/Frameworks/{}.framework", framework_name);

    if std::path::Path::new(&framework_path).exists() {
        println!("cargo:rustc-cfg=has_framework_{}", framework_name.to_lowercase());
    } else {
        println!("cargo:warning=Framework {} not found", framework_name);
    }
}
