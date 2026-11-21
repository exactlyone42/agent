use crate::drift::finding::Finding;

pub fn codex_prompt(path: &str, finding: &Finding) -> String {
    let proofs: Vec<String> = finding
        .evidence
        .iter()
        .map(|proof| format!("- {}:{} \"{}\"", proof.path, proof.line, proof.quote))
        .collect();
    format!(
        "Another reviewer says the file {path} in the current directory is out of date.\n\
         Verdict: {verdict}\n\
         Reason: {reason}\n\
         Evidence:\n{proofs}\n\
         Check the claim yourself by reading the real files. Do not change any file. Set agree to true only if you find the same problem; otherwise set it to false.\n\
         Write the reason in plain Traditional Chinese, at most 120 characters. Answer with the JSON object only.",
        verdict = finding.verdict.name(),
        reason = finding.reason,
        proofs = proofs.join("\n"),
    )
}
