//! Prints the aggregated `OpenAPI` JSON to stdout. The CI workflow and the
//! frontend codegen pipeline pipe it into a file.

use my_fam_tree_openapi::ApiDoc;

#[allow(clippy::print_stdout, reason = "this binary's sole purpose is to print the spec to stdout")]
fn main() -> anyhow::Result<()> {
    let doc = ApiDoc::with_cookie_auth();
    let json = serde_json::to_string_pretty(&doc)?;
    println!("{json}");
    Ok(())
}
