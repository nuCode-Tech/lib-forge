pub fn ping(name: String) -> String {
    format!("hello {}", name)
}

uniffi::include_scaffolding!("uniffi-xforge");
