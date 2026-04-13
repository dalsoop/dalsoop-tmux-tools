fn main() {
    hardcoded_lint::check("src")
        .ipv4()
        .credentials()
        .env_fallback()
        .const_config()
        .run();
}
