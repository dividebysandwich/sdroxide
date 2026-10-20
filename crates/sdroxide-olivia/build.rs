fn main() {
    cc::Build::new()
        .cpp(true)
        .std("c++14")
        .warnings(false)
        .include("cxx")
        .file("cxx/olivia_shim.cpp")
        .compile("olivia_engine");

    println!("cargo:rerun-if-changed=cxx/olivia_shim.cpp");
    println!("cargo:rerun-if-changed=cxx/olivia_shim.h");
    println!("cargo:rerun-if-changed=cxx/pj_mfsk.h");
}
