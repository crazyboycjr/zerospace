fn main() {
    // SkyLight is a private framework, so it is not on the default framework search path
    println!("cargo:rustc-link-search=framework=/System/Library/PrivateFrameworks");
}
