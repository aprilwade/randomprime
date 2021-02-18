
fn main()
{
    cc::Build::new()
        .include("libSquish")
        .file("src/wrapper.cpp")
        .file("libSquish/alpha.cpp")
        .file("libSquish/clusterfit.cpp")
        .file("libSquish/colourblock.cpp")
        .file("libSquish/colourblockGCN.cpp")
        .file("libSquish/colourfit.cpp")
        .file("libSquish/colourset.cpp")
        .file("libSquish/maths.cpp")
        .file("libSquish/rangefit.cpp")
        .file("libSquish/singlecolourfit.cpp")
        .file("libSquish/squish.cpp")
        .cpp(true)
        .compile("libSquish");
}

