
fn main()
{
    // This is based Nod's cmake files. If those change, this probably needs to too.
    let mut build = cpp_build::Config::new();
    build
        .include("nod/include/")
        .include("nod/logvisor/include/")
        .include("nod/logvisor/fmt/include/")
        .file("nod/lib/aes.cpp")
        .file("nod/lib/DirectoryEnumerator.cpp")
        .file("nod/lib/DiscBase.cpp")
        .file("nod/lib/DiscGCN.cpp")
        .file("nod/lib/DiscIOISO.cpp")
        .file("nod/lib/DiscIONFS.cpp")
        .file("nod/lib/DiscIOWBFS.cpp")
        .file("nod/lib/DiscWii.cpp")
        .file("nod/lib/aes.cpp")
        .file("nod/lib/nod.cpp")
        .file("nod/lib/sha1.c")
        .file("nod/logvisor/lib/logvisor.cpp")
        .file("nod/logvisor/fmt/src/format.cc")
        .file("nod/logvisor/fmt/src/os.cc");

    let target = std::env::var("TARGET").unwrap();
    let is_windows = target.contains("windows");
    let is_msvc = target.contains("msvc");

    if is_windows {
        build.file("nod/lib/FileIOWin32.cpp");
    } else {
        build.file("nod/lib/FileIOFILE.cpp");
    }
    if is_msvc {
        build
            .flag("/std:c++17")
            .flag("/EHsc")
            .flag("-DUNICODE=1")
            .flag("-D_UNICODE=1")
            .flag("-D__SSE__=1")
            .flag("-D_CRT_SECURE_NO_WARNINGS=1")
            .flag("-DD_SCL_SECURE_NO_WARNINGS=1")
            .flag("/IGNORE:4221")
            .flag("/wd4018")
            .flag("/wd4800")
            .flag("/wd4005")
            .flag("/wd4311")
            .flag("/wd4267")
            .flag("/wd4244")
            .flag("/wd4200")
            .flag("/wd4305")
            .flag("/wd4067")
            .flag("/wd4146");
    } else {
        build
            .flag("-std=c++17")
            .flag("-Wno-unused-parameter")
            .flag("-Wno-unused-variable")
            .flag("-Wno-sign-compare")
            .flag("-Wno-deprecated");
        if target.contains("x86") {
            build.flag("-maes");
        }
    }
    build.build("src/lib.rs");
}
