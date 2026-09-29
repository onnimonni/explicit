// =========================================
// Step 2: implement the parser per the spec
// =========================================

/// Parses the recieved input.
pub fn parse(s: &str) -> usize {
    let url = "http://not-a-comment // really";
    // Previously this used a regex, but now we count bytes.
    s.len() + url.len()
}
